//! How Luna stops, and why.
//!
//! Luna runs as two processes: a small supervisor (`luna_launcher`) and the app
//! itself. The app cannot replace its own executable while running, which is what the
//! add-a-tool flow needs, so it asks the supervisor to do it by exiting with a
//! particular code.
//!
//! This module holds the pieces both sides need to agree on: the exit-code protocol,
//! how new tool folders are noticed, and whether a rebuild is possible at all on this
//! machine.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::manifest::ToolManifest;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// Why the app is exiting, and what the supervisor should do next.
///
/// Communicated as a process exit code because it has to survive the app dying, which
/// rules out any channel that lives inside the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownIntent {
    /// The user quit. The supervisor exits too.
    Exit,
    /// Relaunch the same binary, keeping saved state.
    Restart,
    /// Rebuild, swap in the new binary, then relaunch.
    Rebuild,
}

/// The user quit; nothing further to do.
pub const EXIT_NORMAL: i32 = 0;
/// Relaunch the same binary.
pub const EXIT_RESTART: i32 = 10;
/// Rebuild, swap, relaunch.
pub const EXIT_REBUILD: i32 = 11;

impl ShutdownIntent {
    /// The exit code the app should terminate with.
    pub fn exit_code(self) -> i32 {
        return match self {
            ShutdownIntent::Exit => EXIT_NORMAL,
            ShutdownIntent::Restart => EXIT_RESTART,
            ShutdownIntent::Rebuild => EXIT_REBUILD,
        };
    }

    /// Reads an exit code back, for the supervisor.
    ///
    /// Any other code means the app died rather than asked for something, which the
    /// supervisor must treat as a crash rather than as a request.
    pub fn from_exit_code(code: i32) -> Option<Self> {
        return match code {
            EXIT_NORMAL => Some(ShutdownIntent::Exit),
            EXIT_RESTART => Some(ShutdownIntent::Restart),
            EXIT_REBUILD => Some(ShutdownIntent::Rebuild),
            _ => None,
        };
    }
}

/// The difference between the tools on disk and the tools in this binary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolScan {
    /// Tool ids present in `tools/` but not compiled in. A rebuild would add them.
    pub added: Vec<String>,
    /// Tool ids compiled in but no longer in `tools/`. A rebuild would drop them.
    pub removed: Vec<String>,
    /// Folders holding a manifest that could not be read or parsed.
    ///
    /// Reported rather than treated as new, because a rebuild would fail on them and
    /// telling the user which folder is broken is more useful than a compiler error.
    pub unreadable: Vec<PathBuf>,
}

impl ToolScan {
    /// Whether a rebuild would change which tools exist.
    pub fn has_changes(&self) -> bool {
        return !self.added.is_empty() || !self.removed.is_empty();
    }

    /// A one-line summary for the user, or `None` if nothing changed.
    pub fn summary(&self) -> Option<String> {
        if !self.has_changes() {
            return None;
        }

        let mut parts = Vec::new();

        if !self.added.is_empty() {
            parts.push(format!("{} to add ({})", self.added.len(), self.added.join(", ")));
        }

        if !self.removed.is_empty() {
            parts.push(format!(
                "{} to remove ({})",
                self.removed.len(),
                self.removed.join(", ")
            ));
        }

        return Some(parts.join(", "));
    }
}

/// Compares the tool folders on disk against the tools compiled into this binary.
///
/// Deliberately stateless: there is no record of what was seen last time, because the
/// compiled-in registry already is that record. A tool is new precisely when its
/// manifest is on disk and its id is not in the binary.
///
/// ## Arguments
/// * `tools_dir` - the `tools/` folder to scan.
/// * `known_ids` - ids of the tools compiled into this binary.
pub fn scan_tools_dir(tools_dir: &Path, known_ids: &[String]) -> ToolScan {
    let mut scan = ToolScan::default();

    let known: BTreeSet<&str> = known_ids.iter().map(|s| s.as_str()).collect();
    let mut found: BTreeSet<String> = BTreeSet::new();

    let entries = match std::fs::read_dir(tools_dir) {
        Ok(entries) => entries,
        // No tools folder at all: nothing was added, and everything compiled in counts
        // as removed only if it genuinely was. Treating this as "all removed" would be
        // alarming and wrong, since a prebuilt distribution has no tools folder.
        Err(_) => return scan,
    };

    for entry in entries.flatten() {
        let dir = entry.path();

        if !dir.is_dir() {
            continue;
        }

        let manifest_path = dir.join("manifest.toml");

        if !manifest_path.exists() {
            // Not a tool folder. Scratch directories are allowed to sit here.
            continue;
        }

        match std::fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|text| ToolManifest::from_toml(&text).ok())
        {
            Some(manifest) => {
                if !known.contains(manifest.id.as_str()) {
                    scan.added.push(manifest.id.clone());
                }
                found.insert(manifest.id);
            }
            None => scan.unreadable.push(dir),
        }
    }

    for id in known_ids {
        if !found.contains(id) {
            scan.removed.push(id.clone());
        }
    }

    scan.added.sort();
    scan.removed.sort();
    scan.unreadable.sort();

    return scan;
}

/// Why Luna can or cannot rebuild itself on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebuildCapability {
    /// Sources and a toolchain are both present.
    Available,
    /// No `cargo` on PATH.
    NoToolchain,
    /// No sources next to the executable, so there is nothing to build.
    ///
    /// This is the ordinary state for a released build: tools arrive through a new
    /// release rather than by being compiled locally.
    NoSources,
}

impl RebuildCapability {
    pub fn is_available(self) -> bool {
        return self == RebuildCapability::Available;
    }

    /// What to tell the user when the add-a-tool path is unavailable.
    pub fn explanation(self) -> &'static str {
        return match self {
            RebuildCapability::Available => "Luna can rebuild itself to add new tools.",
            RebuildCapability::NoToolchain => {
                "Adding tools needs a Rust toolchain, which was not found on this machine. \
                 Install one from rustup.rs, or use a Luna release that already includes \
                 the tools you want."
            }
            RebuildCapability::NoSources => {
                "This copy of Luna was installed without its sources, so it cannot rebuild \
                 itself. Tools come with a new release instead."
            }
        };
    }
}

/// Whether this install can rebuild itself.
///
/// Checked at startup so the add-a-tool UI can be shown or hidden honestly, rather
/// than offering a rebuild that will fail.
pub fn rebuild_capability(install_dir: &Path) -> RebuildCapability {
    if !has_sources(install_dir) {
        return RebuildCapability::NoSources;
    }

    if !has_toolchain() {
        return RebuildCapability::NoToolchain;
    }

    return RebuildCapability::Available;
}

/// Whether the install directory holds buildable sources.
///
/// The rebuild flow compiles tools, which means the install *is* the source tree. A
/// prebuilt copy has no `Cargo.toml` and cannot rebuild.
pub fn has_sources(install_dir: &Path) -> bool {
    return install_dir.join("Cargo.toml").is_file();
}

/// Whether a usable `cargo` is on PATH.
pub fn has_toolchain() -> bool {
    return std::process::Command::new("cargo")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tool(tools_dir: &Path, folder: &str, id: &str) {
        let dir = tools_dir.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.toml"),
            format!("id = \"{id}\"\nname = \"T\"\nversion = \"1.0.0\"\n"),
        )
        .unwrap();
    }

    #[test]
    fn exit_codes_round_trip() {
        for intent in [
            ShutdownIntent::Exit,
            ShutdownIntent::Restart,
            ShutdownIntent::Rebuild,
        ] {
            assert_eq!(ShutdownIntent::from_exit_code(intent.exit_code()), Some(intent));
        }
    }

    #[test]
    fn an_unexpected_exit_code_is_not_a_request() {
        // A crash must never be mistaken for a rebuild request.
        for code in [1, 2, 101, 143, -1] {
            assert_eq!(ShutdownIntent::from_exit_code(code), None, "code {code}");
        }
    }

    #[test]
    fn notices_a_new_tool_folder() {
        let dir = tempfile::tempdir().unwrap();
        write_tool(dir.path(), "base_converter", "luna.base_converter");
        write_tool(dir.path(), "brand_new", "luna.brand_new");

        let scan = scan_tools_dir(dir.path(), &["luna.base_converter".to_string()]);

        assert_eq!(scan.added, vec!["luna.brand_new".to_string()]);
        assert!(scan.removed.is_empty());
        assert!(scan.has_changes());
    }

    #[test]
    fn notices_a_removed_tool_folder() {
        let dir = tempfile::tempdir().unwrap();
        write_tool(dir.path(), "base_converter", "luna.base_converter");

        let scan = scan_tools_dir(
            dir.path(),
            &["luna.base_converter".to_string(), "luna.gone".to_string()],
        );

        assert_eq!(scan.removed, vec!["luna.gone".to_string()]);
        assert!(scan.added.is_empty());
    }

    #[test]
    fn reports_nothing_when_disk_and_binary_agree() {
        let dir = tempfile::tempdir().unwrap();
        write_tool(dir.path(), "a", "luna.a");
        write_tool(dir.path(), "b", "luna.b");

        let scan = scan_tools_dir(dir.path(), &["luna.a".to_string(), "luna.b".to_string()]);

        assert_eq!(scan, ToolScan::default());
        assert!(!scan.has_changes());
        assert_eq!(scan.summary(), None);
    }

    #[test]
    fn matches_on_id_not_folder_name() {
        let dir = tempfile::tempdir().unwrap();
        // Folder name and id deliberately differ.
        write_tool(dir.path(), "some_folder", "luna.the_tool");

        let scan = scan_tools_dir(dir.path(), &["luna.the_tool".to_string()]);

        assert!(!scan.has_changes(), "should have matched by id: {scan:?}");
    }

    #[test]
    fn reports_broken_manifests_instead_of_calling_them_new() {
        let dir = tempfile::tempdir().unwrap();
        let broken = dir.path().join("broken");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join("manifest.toml"), "this is not = = toml").unwrap();

        let scan = scan_tools_dir(dir.path(), &[]);

        assert!(scan.added.is_empty(), "a broken manifest is not a new tool");
        assert_eq!(scan.unreadable, vec![broken]);
    }

    #[test]
    fn ignores_folders_without_a_manifest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("scratch")).unwrap();
        std::fs::write(dir.path().join("README.md"), "not a tool").unwrap();

        let scan = scan_tools_dir(dir.path(), &[]);

        assert_eq!(scan, ToolScan::default());
    }

    #[test]
    fn a_missing_tools_folder_is_not_a_mass_removal() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("no-such-folder");

        let scan = scan_tools_dir(&absent, &["luna.a".to_string()]);

        // A prebuilt distribution ships no tools folder. Claiming every tool had been
        // removed would be alarming and wrong.
        assert!(scan.removed.is_empty());
        assert!(!scan.has_changes());
    }

    #[test]
    fn summary_names_what_would_change() {
        let scan = ToolScan {
            added: vec!["luna.new".to_string()],
            removed: vec!["luna.old".to_string()],
            unreadable: vec![],
        };

        let summary = scan.summary().unwrap();
        assert!(summary.contains("luna.new"), "{summary}");
        assert!(summary.contains("luna.old"), "{summary}");
    }

    #[test]
    fn a_copy_without_sources_cannot_rebuild() {
        let dir = tempfile::tempdir().unwrap();

        assert!(!has_sources(dir.path()));
        assert_eq!(rebuild_capability(dir.path()), RebuildCapability::NoSources);
    }

    #[test]
    fn sources_are_detected_by_the_workspace_manifest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\n").unwrap();

        assert!(has_sources(dir.path()));
    }

    #[test]
    fn every_capability_explains_itself() {
        for capability in [
            RebuildCapability::Available,
            RebuildCapability::NoToolchain,
            RebuildCapability::NoSources,
        ] {
            assert!(!capability.explanation().is_empty());
        }
    }
}
