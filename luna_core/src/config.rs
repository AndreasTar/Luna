//! Settings files.
//!
//! Two levels: [`AppConfig`] in `config/app.toml` for the whole application, and
//! [`ToolConfig`] in `config/tools/<tool-id>.toml` for each tool. Both are TOML, so
//! they stay small, human-editable and diffable, and both are written through
//! [`crate::atomic`] so a kill mid-save cannot corrupt them.
//!
//! Every field carries a serde default. Adding a field in a later version therefore
//! reads an older config without complaint, which matters because Luna rebuilds itself
//! on the user's machine and a config can outlive several versions of the app.
//!
//! A config that fails to parse is never fatal. It is renamed aside and defaults are
//! used, so a hand-edited typo cannot stop Luna from starting.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{IoResultExt, Result};
use crate::{atomic, epoch_seconds};

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// The result of loading a config file that may have been missing or unreadable.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadOutcome<T> {
    /// The config to use. Defaults if the file was missing or unparsable.
    pub value: T,
    /// Set when an existing file could not be parsed and was renamed to this path.
    /// The caller should tell the user, since their settings just reverted.
    pub quarantined: Option<PathBuf>,
}

impl<T> LoadOutcome<T> {
    fn clean(value: T) -> Self {
        return Self { value, quarantined: None };
    }
}

/// Application-wide settings, stored in `config/app.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Where Luna keeps its data. `None` means `<install>/data`.
    ///
    /// This is the only setting that can point outside the install directory, and it
    /// is only ever set by the user.
    pub data_root: Option<PathBuf>,

    /// Id of the palette applied to the whole app. Tools may override it.
    ///
    /// Matches the `id` of a file in `<install>/palettes`. Light and dark are separate
    /// palettes rather than variants of one, so there is no companion mode flag: a
    /// user wanting light picks a light palette.
    ///
    /// If the id names a palette that is missing or unreadable, the picker reports it
    /// and the built-in default is used, rather than starting up unstyled.
    pub palette: String,

    /// Tool whose page was open when Luna last exited, restored on startup.
    pub last_active_tool: Option<String>,

    /// Window geometry, so a restart comes back where it was.
    pub window: WindowConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        return Self {
            data_root: None,
            palette: "night_sky".to_string(),
            last_active_tool: None,
            window: WindowConfig::default(),
        };
    }
}

impl AppConfig {
    /// Reads `config/app.toml`, falling back to defaults if it is missing or broken.
    pub fn load(path: &Path) -> Result<LoadOutcome<Self>> {
        return load_toml(path);
    }

    /// Writes `config/app.toml` atomically.
    pub fn save(&self, path: &Path) -> Result<()> {
        return save_toml(self, path);
    }
}

/// Saved window geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    /// Position, if the window was ever moved. `None` lets the OS place it.
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        return Self {
            width: 1000,
            height: 600,
            x: None,
            y: None,
            maximized: false,
        };
    }
}

/// Per-tool settings, stored in `config/tools/<tool-id>.toml`.
///
/// Keeping these in their own file means a tool's settings are self-contained:
/// deleting the file resets exactly that tool and nothing else.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolConfig {
    /// Whether the tool runs and appears in the sidebar.
    ///
    /// Independent of whether it is compiled in. Everything compiled is present;
    /// this is the runtime toggle.
    pub enabled: bool,

    /// Whether the tool's transient UI state is restored when its page reopens.
    pub remember_ui_state: bool,

    /// How long a saved UI state stays valid.
    pub ui_state_ttl: UiStateTtl,

    /// Palette id for this tool, overriding the application palette.
    pub palette: Option<String>,

    /// Individual colour roles overridden for this tool, as `role -> colour`.
    ///
    /// Colours are kept as written until the palette layer lands; this layer only
    /// persists them.
    pub color_overrides: BTreeMap<String, String>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        return Self {
            enabled: true,
            remember_ui_state: true,
            ui_state_ttl: UiStateTtl::For(Duration::from_secs(60)),
            palette: None,
            color_overrides: BTreeMap::new(),
        };
    }
}

impl ToolConfig {
    /// Reads a tool's config, falling back to defaults if missing or broken.
    pub fn load(path: &Path) -> Result<LoadOutcome<Self>> {
        return load_toml(path);
    }

    /// Writes a tool's config atomically.
    pub fn save(&self, path: &Path) -> Result<()> {
        return save_toml(self, path);
    }
}

/// How long a tool's saved UI state remains valid.
///
/// Serialised as a string: `"never"`, `"session"`, `"forever"`, or a duration such as
/// `"30s"`, `"15m"`, `"6h"`, `"7d"`, `"2w"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiStateTtl {
    /// Never persist UI state at all.
    Never,
    /// Keep it only while the app is running; discard on exit.
    Session,
    /// Keep it until the tool or the user clears it.
    Forever,
    /// Keep it for a fixed period after it was written.
    For(Duration),
}

impl std::fmt::Display for UiStateTtl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            UiStateTtl::Never => write!(f, "never"),
            UiStateTtl::Session => write!(f, "session"),
            UiStateTtl::Forever => write!(f, "forever"),
            UiStateTtl::For(d) => write!(f, "{}", format_duration(*d)),
        };
    }
}

impl std::str::FromStr for UiStateTtl {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let trimmed = s.trim();
        return match trimmed.to_ascii_lowercase().as_str() {
            "never" => Ok(UiStateTtl::Never),
            "session" => Ok(UiStateTtl::Session),
            "forever" => Ok(UiStateTtl::Forever),
            _ => parse_duration(trimmed)
                .map(UiStateTtl::For)
                .ok_or_else(|| {
                    format!(
                        "expected \"never\", \"session\", \"forever\" or a duration \
                         like \"7d\", got {trimmed:?}"
                    )
                }),
        };
    }
}

impl Serialize for UiStateTtl {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        return serializer.serialize_str(&self.to_string());
    }
}

impl<'de> Deserialize<'de> for UiStateTtl {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        return raw.parse().map_err(serde::de::Error::custom);
    }
}

/// Parses `"7d"`, `"30s"` and friends into a [`Duration`].
///
/// Accepted suffixes are `s`, `m`, `h`, `d` and `w`. A bare number is rejected, since
/// a silent unit choice would be a poor guess.
fn parse_duration(text: &str) -> Option<Duration> {
    let text = text.trim();
    let (digits, suffix) = text.split_at(text.find(|c: char| !c.is_ascii_digit())?);

    if digits.is_empty() {
        return None;
    }

    let amount: u64 = digits.parse().ok()?;

    let seconds_per_unit = match suffix.trim() {
        "s" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        "w" => 7 * 24 * 60 * 60,
        _ => return None,
    };

    return amount.checked_mul(seconds_per_unit).map(Duration::from_secs);
}

/// Renders a [`Duration`] using the largest unit that divides it evenly.
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();

    if secs == 0 {
        return "0s".to_string();
    }

    for (unit_secs, suffix) in [
        (7 * 24 * 60 * 60, "w"),
        (24 * 60 * 60, "d"),
        (60 * 60, "h"),
        (60, "m"),
    ] {
        if secs % unit_secs == 0 {
            return format!("{}{}", secs / unit_secs, suffix);
        }
    }

    return format!("{secs}s");
}

/// Reads and parses a TOML file, quarantining it if it cannot be parsed.
fn load_toml<T>(path: &Path) -> Result<LoadOutcome<T>>
where
    T: Default + for<'de> Deserialize<'de>,
{
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LoadOutcome::clean(T::default()));
        }
        Err(e) => return Err(e).at_path(path),
    };

    return match toml::from_str::<T>(&raw) {
        Ok(value) => Ok(LoadOutcome::clean(value)),
        Err(_) => {
            // The file exists but is not valid. Losing the user's settings silently
            // would be worse than losing them loudly, so keep the original where they
            // can find it and carry on with defaults.
            let quarantined = quarantine(path)?;
            Ok(LoadOutcome {
                value: T::default(),
                quarantined: Some(quarantined),
            })
        }
    };
}

/// Serialises a value to TOML and writes it atomically.
fn save_toml<T: Serialize>(value: &T, path: &Path) -> Result<()> {
    let text = toml::to_string_pretty(value)?;
    return atomic::write_string(path, &text);
}

/// Renames an unparsable config aside so the user can recover it by hand.
fn quarantine(path: &Path) -> Result<PathBuf> {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bad-{}", epoch_seconds()));

    let target = path.with_file_name(name);
    std::fs::rename(path, &target).at_path(path)?;

    return Ok(target);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_config_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.toml");

        let mut config = AppConfig::default();
        config.palette = "monochrome_gray".to_string();
        config.last_active_tool = Some("luna.calendar".to_string());
        config.window.width = 1440;

        config.save(&path).unwrap();
        let loaded = AppConfig::load(&path).unwrap();

        assert_eq!(loaded.value, config);
        assert_eq!(loaded.quarantined, None);
    }

    #[test]
    fn tool_config_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("luna.calendar.toml");

        let mut config = ToolConfig::default();
        config.enabled = false;
        config.ui_state_ttl = UiStateTtl::Session;
        config.palette = Some("night_sky".to_string());
        config
            .color_overrides
            .insert("warning".to_string(), "#ffdd00".to_string());

        config.save(&path).unwrap();
        let loaded = ToolConfig::load(&path).unwrap();

        assert_eq!(loaded.value, config);
    }

    #[test]
    fn missing_file_yields_defaults_without_quarantine() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("absent.toml");

        let loaded = AppConfig::load(&path).unwrap();

        assert_eq!(loaded.value, AppConfig::default());
        assert_eq!(loaded.quarantined, None);
        assert!(!path.exists(), "loading must not create the file");
    }

    #[test]
    fn corrupt_file_is_quarantined_and_defaults_are_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.toml");
        std::fs::write(&path, "this is not = = valid toml [[[").unwrap();

        let loaded = AppConfig::load(&path).unwrap();

        assert_eq!(loaded.value, AppConfig::default());

        let quarantined = loaded.quarantined.expect("should have been quarantined");
        assert!(quarantined.exists());
        assert!(!path.exists(), "the broken file should have been moved aside");
        assert!(quarantined.file_name().unwrap().to_string_lossy().contains(".bad-"));
    }

    #[test]
    fn unknown_fields_are_ignored_and_missing_fields_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.toml");

        // A config written by a future version, and one written by an older one.
        std::fs::write(&path, "palette = \"custom\"\nsomething_new = 42\n").unwrap();

        let loaded = AppConfig::load(&path).unwrap();

        assert_eq!(loaded.quarantined, None, "forward compat must not quarantine");
        assert_eq!(loaded.value.palette, "custom");
        assert_eq!(loaded.value.window, AppConfig::default().window);
    }

    #[test]
    fn parses_every_ttl_form() {
        assert_eq!("never".parse(), Ok(UiStateTtl::Never));
        assert_eq!("session".parse(), Ok(UiStateTtl::Session));
        assert_eq!("FOREVER".parse(), Ok(UiStateTtl::Forever));
        assert_eq!("30s".parse(), Ok(UiStateTtl::For(Duration::from_secs(30))));
        assert_eq!("15m".parse(), Ok(UiStateTtl::For(Duration::from_secs(900))));
        assert_eq!("6h".parse(), Ok(UiStateTtl::For(Duration::from_secs(21600))));
        assert_eq!("7d".parse(), Ok(UiStateTtl::For(Duration::from_secs(604800))));
        assert_eq!("2w".parse(), Ok(UiStateTtl::For(Duration::from_secs(1209600))));
    }

    #[test]
    fn rejects_ambiguous_or_malformed_ttls() {
        for bad in ["", "7", "d", "7y", "-3d", "7 days", "abc"] {
            assert!(bad.parse::<UiStateTtl>().is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn ttl_survives_a_string_round_trip() {
        let cases = [
            UiStateTtl::Never,
            UiStateTtl::Session,
            UiStateTtl::Forever,
            UiStateTtl::For(Duration::from_secs(45)),
            UiStateTtl::For(Duration::from_secs(90 * 60)),
            UiStateTtl::For(Duration::from_secs(7 * 24 * 60 * 60)),
        ];

        for case in cases {
            assert_eq!(case.to_string().parse(), Ok(case), "{case:?}");
        }
    }

    #[test]
    fn formats_durations_with_the_largest_clean_unit() {
        assert_eq!(format_duration(Duration::from_secs(604800)), "1w");
        assert_eq!(format_duration(Duration::from_secs(86400)), "1d");
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h");
        assert_eq!(format_duration(Duration::from_secs(60)), "1m");
        assert_eq!(format_duration(Duration::from_secs(90)), "90s");
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
    }

    #[test]
    fn saving_replaces_the_previous_file_completely() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.toml");

        let mut config = AppConfig::default();
        config.last_active_tool = Some("luna.base_converter".to_string());
        config.save(&path).unwrap();

        config.last_active_tool = None;
        config.save(&path).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("base_converter"), "stale content survived: {text}");
    }
}
