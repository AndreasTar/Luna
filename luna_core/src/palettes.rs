//! Loading palette files from disk.
//!
//! Palettes are TOML files in `<install>/palettes`. Unlike tools they are **data, not
//! code**, so they load at runtime: drop a file in, reopen the picker, and it is
//! there. No rebuild.
//!
//! The colour model itself lives in [`luna::palette`], which is pure and knows nothing
//! about files. This module is the bridge: it parses, validates, and reports what it
//! could not use.
//!
//! ## Why problems are collected rather than returned
//!
//! One unreadable palette must not stop the others loading, and it must not be
//! silently skipped either: the user wrote that file and is waiting to see it. Every
//! failure is kept in [`PaletteSet::problems`] with the reason and the path, so the
//! picker can list it as unusable and say why.

use std::path::{Path, PathBuf};

use luna::palette::{Appearance, Color, Overrides, Palette, Resolved, Role};

use crate::config::ToolConfig;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// The palette format version this build understands.
const SUPPORTED_FORMAT: u32 = 1;

/// A palette file that could not be used, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteProblem {
    pub path: PathBuf,
    pub reason: String,
}

impl std::fmt::Display for PaletteProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}: {}", self.path.display(), self.reason);
    }
}

/// Every palette found on disk, plus the ones that could not be read.
#[derive(Debug, Clone, Default)]
pub struct PaletteSet {
    palettes: Vec<Palette>,
    /// Files that failed to load. Surfaced in the picker rather than swallowed.
    pub problems: Vec<PaletteProblem>,
}

impl PaletteSet {
    /// Reads every `.toml` in `dir`.
    ///
    /// A missing directory is not an error: it yields an empty set, and the built-in
    /// fallback keeps the app usable.
    pub fn load_dir(dir: &Path) -> Self {
        let mut set = Self::default();

        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return set,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }

            match load_file(&path) {
                Ok(palette) => set.palettes.push(palette),
                Err(reason) => set.problems.push(PaletteProblem { path, reason }),
            }
        }

        // Two files claiming the same id is ambiguous, so neither is trusted silently.
        set.reject_duplicate_ids();

        set.palettes.sort_by(|a, b| a.name.cmp(&b.name));
        set.problems.sort_by(|a, b| a.path.cmp(&b.path));

        return set;
    }

    /// Moves every palette whose id is shared with another into `problems`.
    fn reject_duplicate_ids(&mut self) {
        let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
        for palette in &self.palettes {
            *counts.entry(palette.id.clone()).or_default() += 1;
        }

        let duplicated: Vec<String> = counts
            .into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(id, _)| id)
            .collect();

        for id in duplicated {
            self.palettes.retain(|p| p.id != id);
            self.problems.push(PaletteProblem {
                path: PathBuf::from(format!("{id}.toml")),
                reason: format!(
                    "more than one palette file declares the id {id:?}, so none of them \
                     were loaded. Give each palette a unique id."
                ),
            });
        }
    }

    /// Every usable palette, sorted by name.
    pub fn all(&self) -> &[Palette] {
        return &self.palettes;
    }

    /// Palettes matching an appearance, for grouping the picker.
    ///
    /// Light and dark are separate palettes rather than variants of one, so this only
    /// groups; it does not select.
    pub fn by_appearance(&self, appearance: Appearance) -> Vec<&Palette> {
        return self
            .palettes
            .iter()
            .filter(|p| p.appearance == appearance)
            .collect();
    }

    pub fn get(&self, id: &str) -> Option<&Palette> {
        return self.palettes.iter().find(|p| p.id == id);
    }

    pub fn is_empty(&self) -> bool {
        return self.palettes.is_empty();
    }

    /// The palette to use as the application base.
    ///
    /// Falls back to any loaded palette, then to [`built_in`], so the app is never
    /// unstyled because of a typo in a setting or an empty palettes folder.
    pub fn app_palette(&self, configured_id: &str) -> Palette {
        if let Some(palette) = self.get(configured_id) {
            return palette.clone();
        }

        if let Some(first) = self.palettes.first() {
            return first.clone();
        }

        return built_in();
    }

    /// The finished palette for one tool, with its overrides applied.
    ///
    /// The tool's own palette and per-role overrides come from its settings; anything
    /// it does not override falls through to the application palette.
    pub fn resolve_for_tool(&self, app: &Palette, tool: &ToolConfig) -> Resolved {
        let consumer = tool.palette.as_deref().and_then(|id| self.get(id));

        let mut overrides = Overrides::new();
        for (name, value) in &tool.color_overrides {
            // A bad role name or colour in a hand-edited config is skipped rather than
            // fatal. The palette editor validates before writing; this path exists for
            // files edited by hand.
            if let (Ok(role), Ok(color)) = (name.parse::<Role>(), Color::from_hex(value)) {
                overrides.set(role, color);
            }
        }

        return overrides.resolve(app, consumer);
    }
}

/// Parses one palette file.
fn load_file(path: &Path) -> Result<Palette, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("could not be read: {e}"))?;

    let doc: toml::Value = toml::from_str(&text).map_err(|e| format!("is not valid TOML: {e}"))?;

    let table = doc.as_table().ok_or("is not a TOML table")?;

    let string = |key: &str| -> Option<String> {
        return table.get(key).and_then(|v| v.as_str()).map(|s| s.to_string());
    };

    if let Some(version) = table.get("luna_palette_version").and_then(|v| v.as_integer()) {
        if version as u32 > SUPPORTED_FORMAT {
            return Err(format!(
                "is format version {version}, but this build of Luna understands only \
                 up to {SUPPORTED_FORMAT}"
            ));
        }
    }

    let id = string("id").ok_or("has no id")?;
    let name = string("name").unwrap_or_else(|| id.clone());

    let mut palette = Palette::new(id, name);
    palette.description = string("description").unwrap_or_default();
    palette.author = string("author").unwrap_or_default();

    if let Some(appearance) = string("appearance") {
        palette.appearance = appearance
            .parse::<Appearance>()
            .map_err(|e| format!("has an invalid appearance: {e}"))?;
    }

    let colors = table
        .get("colors")
        .and_then(|v| v.as_table())
        .ok_or("has no [colors] section")?;

    for (key, value) in colors {
        let role = key
            .parse::<Role>()
            .map_err(|_| format!("lists an unknown colour role {key:?}"))?;

        let text = value
            .as_str()
            .ok_or_else(|| format!("gives a non-string colour for {key:?}"))?;

        let color = Color::from_hex(text).map_err(|e| format!("{e} (role {key})"))?;

        palette.set(role, color);
    }

    // A palette missing a role would leave that colour resolving to the loud fallback
    // somewhere far from the mistake, so it is refused with the missing names listed.
    let missing = palette.missing_roles();
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|r| r.as_str()).collect();
        return Err(format!(
            "is missing {} colour role(s): {}",
            names.len(),
            names.join(", ")
        ));
    }

    return Ok(palette);
}

/// A minimal dark palette compiled into the binary.
///
/// Used only when no palette file can be loaded at all, so that a missing or empty
/// palettes folder produces a plain but usable app rather than an unstyled one.
pub fn built_in() -> Palette {
    let mut p = Palette::new("built_in", "Built-in");
    p.description = "Fallback palette, used when no palette files could be loaded.".to_string();
    p.appearance = Appearance::Dark;

    let bg = Color::rgb(25, 25, 25);
    let fg = Color::rgb(230, 230, 230);
    let accent = Color::rgb(34, 102, 204);

    for &role in Role::ALL {
        let color = match role {
            Role::Background => bg,
            Role::BackgroundSecondary => Color::rgb(35, 35, 35),
            Role::BackgroundTertiary => Color::rgb(45, 45, 45),
            Role::BackgroundQuaternary => Color::rgb(55, 55, 55),

            Role::Text => fg,
            Role::TextSecondary => Color::rgb(190, 190, 190),
            Role::TextTertiary => Color::rgb(150, 150, 150),
            // Anything darker than about 130 drops below 4.5:1 on this background.
            Role::TextQuaternary => Color::rgb(140, 140, 140),

            Role::Primary => accent,
            Role::Secondary => Color::rgb(74, 144, 226),
            Role::Tertiary => Color::rgb(18, 88, 162),
            Role::Quaternary => Color::rgb(221, 238, 255),

            Role::Border => Color::rgb(70, 70, 70),
            Role::BorderSecondary => Color::rgb(90, 90, 90),
            Role::BorderTertiary => Color::rgb(110, 110, 110),
            Role::BorderQuaternary => accent,

            // These are rendered as text and signals on the background, so they are
            // lighter than the accent, which is only ever used as a fill or a border.
            // The readability test below keeps them honest.
            Role::Success => Color::rgb(80, 200, 120),
            Role::Warning => Color::rgb(230, 160, 40),
            Role::Error => Color::rgb(235, 105, 105),
            Role::Info => Color::rgb(90, 160, 240),
            Role::Danger => Color::rgb(240, 90, 160),

            Role::Inactive => Color::rgb(120, 120, 120),
            Role::Disabled => Color::rgb(80, 80, 80),

            Role::Highlight => Color::rgb(255, 213, 74),
        };

        p.set(role, color);
    }

    return p;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_body() -> String {
        let mut body = String::from("[colors]\n");
        for role in Role::ALL {
            body.push_str(&format!("{} = \"#101010\"\n", role.as_str()));
        }
        return body;
    }

    fn write_palette(dir: &Path, file: &str, header: &str) {
        std::fs::write(dir.join(file), format!("{header}\n{}", complete_body())).unwrap();
    }

    #[test]
    fn loads_a_complete_palette() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(
            dir.path(),
            "dusk.toml",
            "id = \"dusk\"\nname = \"Dusk\"\nappearance = \"dark\"",
        );

        let set = PaletteSet::load_dir(dir.path());

        assert!(set.problems.is_empty(), "{:?}", set.problems);
        assert_eq!(set.all().len(), 1);

        let palette = set.get("dusk").unwrap();
        assert_eq!(palette.name, "Dusk");
        assert!(palette.is_complete());
    }

    #[test]
    fn a_missing_folder_is_empty_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let set = PaletteSet::load_dir(&dir.path().join("nope"));

        assert!(set.is_empty());
        assert!(set.problems.is_empty());
    }

    #[test]
    fn an_incomplete_palette_is_refused_and_names_the_missing_roles() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("partial.toml"),
            "id = \"partial\"\n[colors]\ntext = \"#ffffff\"\n",
        )
        .unwrap();

        let set = PaletteSet::load_dir(dir.path());

        assert!(set.is_empty(), "an incomplete palette must not be usable");
        assert_eq!(set.problems.len(), 1);
        assert!(
            set.problems[0].reason.contains("background"),
            "should name what is missing: {}",
            set.problems[0].reason
        );
    }

    #[test]
    fn an_unknown_role_is_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let mut body = complete_body();
        body.push_str("chartreuse = \"#00ff00\"\n");
        std::fs::write(dir.path().join("odd.toml"), format!("id = \"odd\"\n{body}")).unwrap();

        let set = PaletteSet::load_dir(dir.path());

        assert!(set.is_empty());
        assert!(set.problems[0].reason.contains("chartreuse"), "{:?}", set.problems);
    }

    #[test]
    fn a_bad_colour_is_refused_and_names_the_role() {
        let dir = tempfile::tempdir().unwrap();
        let body = complete_body().replace("warning = \"#101010\"", "warning = \"not a colour\"");
        std::fs::write(dir.path().join("bad.toml"), format!("id = \"bad\"\n{body}")).unwrap();

        let set = PaletteSet::load_dir(dir.path());

        assert!(set.is_empty());
        assert!(set.problems[0].reason.contains("warning"), "{:?}", set.problems);
    }

    #[test]
    fn one_broken_file_does_not_stop_the_others() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(dir.path(), "good.toml", "id = \"good\"\nname = \"Good\"");
        std::fs::write(dir.path().join("broken.toml"), "= = not toml").unwrap();

        let set = PaletteSet::load_dir(dir.path());

        assert_eq!(set.all().len(), 1);
        assert_eq!(set.problems.len(), 1);
        assert!(set.get("good").is_some());
    }

    #[test]
    fn duplicate_ids_disqualify_all_of_them() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(dir.path(), "one.toml", "id = \"clash\"\nname = \"One\"");
        write_palette(dir.path(), "two.toml", "id = \"clash\"\nname = \"Two\"");

        let set = PaletteSet::load_dir(dir.path());

        // Picking one arbitrarily would mean the file the user edits is not
        // necessarily the one taking effect.
        assert!(set.get("clash").is_none());
        assert!(set.problems.iter().any(|p| p.reason.contains("clash")));
    }

    #[test]
    fn a_future_format_version_is_refused_rather_than_guessed_at() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(
            dir.path(),
            "future.toml",
            "luna_palette_version = 99\nid = \"future\"",
        );

        let set = PaletteSet::load_dir(dir.path());

        assert!(set.is_empty());
        assert!(set.problems[0].reason.contains("99"), "{:?}", set.problems);
    }

    #[test]
    fn non_toml_files_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(dir.path(), "real.toml", "id = \"real\"");
        std::fs::write(dir.path().join("README.md"), "not a palette").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "also not").unwrap();

        let set = PaletteSet::load_dir(dir.path());

        assert_eq!(set.all().len(), 1);
        assert!(set.problems.is_empty());
    }

    #[test]
    fn groups_by_appearance() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(dir.path(), "d.toml", "id = \"d\"\nappearance = \"dark\"");
        write_palette(dir.path(), "l.toml", "id = \"l\"\nappearance = \"light\"");

        let set = PaletteSet::load_dir(dir.path());

        assert_eq!(set.by_appearance(Appearance::Dark).len(), 1);
        assert_eq!(set.by_appearance(Appearance::Light).len(), 1);
    }

    #[test]
    fn an_unknown_configured_palette_falls_back_instead_of_leaving_the_app_unstyled() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(dir.path(), "only.toml", "id = \"only\"\nname = \"Only\"");

        let set = PaletteSet::load_dir(dir.path());

        assert_eq!(set.app_palette("no_such_palette").id, "only");
    }

    #[test]
    fn an_empty_folder_falls_back_to_the_built_in_palette() {
        let dir = tempfile::tempdir().unwrap();
        let set = PaletteSet::load_dir(dir.path());

        let palette = set.app_palette("anything");

        assert_eq!(palette.id, "built_in");
        assert!(palette.is_complete());
    }

    #[test]
    fn the_built_in_palette_is_complete_and_readable() {
        let palette = built_in();

        assert!(palette.is_complete());
        assert!(
            palette.readability_warnings().is_empty(),
            "the fallback should not ship with unreadable text: {:?}",
            palette.readability_warnings()
        );
    }

    #[test]
    fn a_tool_override_changes_only_that_role() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(dir.path(), "base.toml", "id = \"base\"");
        let set = PaletteSet::load_dir(dir.path());
        let app = set.app_palette("base");

        let mut tool = ToolConfig::default();
        tool.color_overrides
            .insert("warning".to_string(), "#ffdd00".to_string());

        let resolved = set.resolve_for_tool(&app, &tool);

        assert_eq!(resolved.get(Role::Warning), Color::from_hex("#ffdd00").unwrap());
        assert_eq!(resolved.get(Role::Error), Color::from_hex("#101010").unwrap());
    }

    #[test]
    fn a_tool_can_use_a_different_palette_entirely() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(dir.path(), "app.toml", "id = \"app\"");
        std::fs::write(
            dir.path().join("other.toml"),
            format!("id = \"other\"\n{}", complete_body().replace("#101010", "#909090")),
        )
        .unwrap();

        let set = PaletteSet::load_dir(dir.path());
        let app = set.app_palette("app");

        let mut tool = ToolConfig::default();
        tool.palette = Some("other".to_string());

        let resolved = set.resolve_for_tool(&app, &tool);

        assert_eq!(resolved.get(Role::Text), Color::from_hex("#909090").unwrap());
    }

    #[test]
    fn a_hand_edited_bad_override_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        write_palette(dir.path(), "base.toml", "id = \"base\"");
        let set = PaletteSet::load_dir(dir.path());
        let app = set.app_palette("base");

        let mut tool = ToolConfig::default();
        tool.color_overrides
            .insert("not_a_role".to_string(), "#ffffff".to_string());
        tool.color_overrides
            .insert("warning".to_string(), "not a colour".to_string());

        let resolved = set.resolve_for_tool(&app, &tool);

        // Both are ignored, and the palette still resolves fully.
        assert_eq!(resolved.get(Role::Warning), Color::from_hex("#101010").unwrap());
        assert_eq!(resolved.iter().count(), Role::COUNT);
    }

    #[test]
    fn the_shipped_palettes_all_load() {
        // The four files in <repo>/palettes are the ones users see first, so a typo in
        // one of them is worth catching here rather than in the picker.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("palettes");

        if !dir.is_dir() {
            return;
        }

        let set = PaletteSet::load_dir(&dir);

        assert!(set.problems.is_empty(), "shipped palettes failed: {:?}", set.problems);
        assert!(set.all().len() >= 4, "expected the bundled palettes, got {}", set.all().len());

        for palette in set.all() {
            assert!(palette.is_complete(), "{} is incomplete", palette.id);
        }
    }
}
