//! Running cargo on the user's behalf.

use std::path::{Path, PathBuf};
use std::process::Command;

/// What a rebuild produced.
pub struct RebuildResult {
    pub succeeded: bool,
    /// Compiler output, kept whether or not the build worked.
    ///
    /// On failure this is what the user needs to see, so it is written to a log and
    /// handed back rather than streamed and forgotten.
    pub output: String,
}

/// Where the freshly built app binary lands.
pub fn built_binary(install_dir: &Path, profile: &str) -> PathBuf {
    return install_dir
        .join("target")
        .join(profile)
        .join(format!("luna_ui{}", std::env::consts::EXE_SUFFIX));
}

/// Builds the app in the install directory.
///
/// Runs with the install directory as the working directory, because that is where
/// the workspace and the `tools/` folder live.
pub fn run(install_dir: &Path, profile: &str) -> RebuildResult {
    let mut command = Command::new("cargo");
    command.current_dir(install_dir).arg("build").arg("-p").arg("luna_ui");

    if profile == "release" {
        command.arg("--release");
    }

    let output = match command.output() {
        Ok(output) => output,
        Err(e) => {
            return RebuildResult {
                succeeded: false,
                output: format!(
                    "could not run cargo: {e}\n\
                     Adding tools needs a Rust toolchain on PATH."
                ),
            };
        }
    };

    // cargo writes diagnostics to stderr and little of interest to stdout, but both
    // are kept so a confusing failure has everything available.
    let mut text = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !stdout.trim().is_empty() {
        text.push_str("\n--- stdout ---\n");
        text.push_str(&stdout);
    }

    return RebuildResult {
        succeeded: output.status.success(),
        output: text,
    };
}

/// Writes build output where the app can find it after relaunching.
///
/// A failed rebuild is reported by the *next* run of the app, which has no other way
/// to know what went wrong.
pub fn write_log(logs_dir: &Path, result: &RebuildResult) -> PathBuf {
    let path = logs_dir.join("last-rebuild.log");

    let _ = std::fs::create_dir_all(logs_dir);

    let header = if result.succeeded {
        "Rebuild succeeded.\n\n"
    } else {
        "Rebuild FAILED. The previous version of Luna is still in use.\n\n"
    };

    let _ = std::fs::write(&path, format!("{header}{}", result.output));

    return path;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_binary_follows_the_profile() {
        let debug = built_binary(Path::new("/install"), "debug");
        let release = built_binary(Path::new("/install"), "release");

        assert!(debug.to_string_lossy().contains("target"));
        assert!(debug.to_string_lossy().contains("debug"));
        assert!(release.to_string_lossy().contains("release"));
        assert!(debug
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(std::env::consts::EXE_SUFFIX));
    }

    #[test]
    fn a_failed_build_log_says_the_old_version_is_still_running() {
        let dir = tempfile::tempdir().unwrap();

        let path = write_log(
            dir.path(),
            &RebuildResult { succeeded: false, output: "error[E0432]: unresolved import".into() },
        );

        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains("FAILED"));
        assert!(text.contains("previous version"));
        assert!(text.contains("E0432"), "compiler output must be preserved");
    }

    #[test]
    fn a_successful_build_log_says_so() {
        let dir = tempfile::tempdir().unwrap();

        let path = write_log(
            dir.path(),
            &RebuildResult { succeeded: true, output: "Finished".into() },
        );

        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains("succeeded"));
        assert!(!text.contains("FAILED"));
    }

    #[test]
    fn log_directory_is_created_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("does").join("not").join("exist");

        let path = write_log(&logs, &RebuildResult { succeeded: true, output: String::new() });

        assert!(path.is_file());
    }

    #[test]
    fn a_missing_cargo_is_reported_rather_than_panicking() {
        let dir = tempfile::tempdir().unwrap();

        // Not a workspace, so cargo either fails or is absent. Either way the launcher
        // must come back with a message instead of dying.
        let result = run(dir.path(), "debug");

        assert!(!result.succeeded);
        assert!(!result.output.is_empty());
    }
}
