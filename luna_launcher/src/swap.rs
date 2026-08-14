//! Replacing the app binary, and putting it back if that goes wrong.
//!
//! Windows will not let a running executable be overwritten, which is the whole reason
//! the launcher exists as a separate process. Once the app has exited, its file can be
//! moved aside and a freshly built one put in its place.
//!
//! The previous binary is always kept. A tool that does not compile, or that compiles
//! into an app that will not start, must never leave the user with nothing that runs.

use std::path::{Path, PathBuf};

/// Where the app binaries live, relative to the install directory.
pub struct BinaryPaths {
    /// The binary the launcher runs.
    pub current: PathBuf,
    /// The last binary known to start, kept for rollback.
    pub previous: PathBuf,
}

impl BinaryPaths {
    pub fn in_dir(install_dir: &Path) -> Self {
        let suffix = std::env::consts::EXE_SUFFIX;

        return Self {
            current: install_dir.join(format!("luna_app{suffix}")),
            previous: install_dir.join(format!("luna_app.prev{suffix}")),
        };
    }
}

/// What happened during a swap, for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapOutcome {
    /// The new binary is in place and the old one is kept as `.prev`.
    Swapped,
    /// The new binary was identical, so nothing moved.
    AlreadyCurrent,
}

/// Moves the current binary aside and puts `fresh` in its place.
///
/// The old binary is kept at [`BinaryPaths::previous`], which [`rollback`] restores.
///
/// ## Errors
/// Any IO failure, with the current binary left untouched if the move aside fails.
pub fn install(paths: &BinaryPaths, fresh: &Path) -> std::io::Result<SwapOutcome> {
    if !fresh.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("freshly built binary not found at {}", fresh.display()),
        ));
    }

    // Nothing to do if the build produced the same bytes, which happens when the
    // rebuild was triggered by something that does not affect the binary.
    if same_contents(fresh, &paths.current) {
        return Ok(SwapOutcome::AlreadyCurrent);
    }

    if paths.current.exists() {
        // Replaces any existing .prev. Only one generation is kept: the point is to
        // recover from a bad build, not to keep history.
        std::fs::rename(&paths.current, &paths.previous)?;
    }

    // Copy rather than rename, because the build output stays where cargo put it and
    // may be on a different volume from the install directory.
    match std::fs::copy(fresh, &paths.current) {
        Ok(_) => Ok(SwapOutcome::Swapped),
        Err(e) => {
            // Put the old binary back before reporting, so a failed copy does not
            // leave the install with no app at all.
            let _ = std::fs::rename(&paths.previous, &paths.current);
            Err(e)
        }
    }
}

/// Restores the previous binary after a build or launch that did not work out.
///
/// ## Errors
/// If there is no previous binary to restore, or the restore itself fails.
pub fn rollback(paths: &BinaryPaths) -> std::io::Result<()> {
    if !paths.previous.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no previous binary to roll back to",
        ));
    }

    // The broken binary is discarded rather than kept: it is reproducible from the
    // sources that produced it, and keeping it invites restoring it by accident.
    if paths.current.exists() {
        std::fs::remove_file(&paths.current)?;
    }

    std::fs::rename(&paths.previous, &paths.current)?;

    return Ok(());
}

/// Whether a rollback is possible.
pub fn can_roll_back(paths: &BinaryPaths) -> bool {
    return paths.previous.is_file();
}

fn same_contents(a: &Path, b: &Path) -> bool {
    let (Ok(a), Ok(b)) = (std::fs::read(a), std::fs::read(b)) else {
        return false;
    };

    return a == b;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, BinaryPaths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = BinaryPaths::in_dir(dir.path());
        return (dir, paths);
    }

    fn write(path: &Path, contents: &str) {
        std::fs::write(path, contents).unwrap();
    }

    fn read(path: &Path) -> String {
        return std::fs::read_to_string(path).unwrap();
    }

    #[test]
    fn installs_the_new_binary_and_keeps_the_old_one() {
        let (dir, paths) = setup();
        write(&paths.current, "old");

        let fresh = dir.path().join("fresh.exe");
        write(&fresh, "new");

        assert_eq!(install(&paths, &fresh).unwrap(), SwapOutcome::Swapped);

        assert_eq!(read(&paths.current), "new");
        assert_eq!(read(&paths.previous), "old", "the old binary must be recoverable");
    }

    #[test]
    fn installs_into_a_fresh_directory_with_nothing_to_keep() {
        let (dir, paths) = setup();

        let fresh = dir.path().join("fresh.exe");
        write(&fresh, "new");

        assert_eq!(install(&paths, &fresh).unwrap(), SwapOutcome::Swapped);

        assert_eq!(read(&paths.current), "new");
        assert!(!paths.previous.exists());
        assert!(!can_roll_back(&paths));
    }

    #[test]
    fn skips_the_swap_when_the_build_changed_nothing() {
        let (dir, paths) = setup();
        write(&paths.current, "same");

        let fresh = dir.path().join("fresh.exe");
        write(&fresh, "same");

        assert_eq!(install(&paths, &fresh).unwrap(), SwapOutcome::AlreadyCurrent);

        assert_eq!(read(&paths.current), "same");
        assert!(!paths.previous.exists(), "an unchanged build must not rotate .prev");
    }

    #[test]
    fn rollback_restores_the_previous_binary() {
        let (dir, paths) = setup();
        write(&paths.current, "good");

        let fresh = dir.path().join("fresh.exe");
        write(&fresh, "broken");

        install(&paths, &fresh).unwrap();
        assert_eq!(read(&paths.current), "broken");

        rollback(&paths).unwrap();

        assert_eq!(read(&paths.current), "good", "the working binary must come back");
        assert!(!paths.previous.exists(), "the rollback consumes .prev");
    }

    #[test]
    fn a_missing_build_output_is_reported_and_changes_nothing() {
        let (dir, paths) = setup();
        write(&paths.current, "good");

        let err = install(&paths, &dir.path().join("never-built.exe")).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(read(&paths.current), "good", "a failed build must not disturb the app");
        assert!(!paths.previous.exists());
    }

    #[test]
    fn rollback_without_a_previous_binary_is_an_error_not_a_panic() {
        let (_dir, paths) = setup();
        write(&paths.current, "only");

        assert!(!can_roll_back(&paths));
        assert!(rollback(&paths).is_err());
        assert_eq!(read(&paths.current), "only");
    }

    #[test]
    fn a_full_bad_build_cycle_leaves_a_working_app() {
        let (dir, paths) = setup();
        write(&paths.current, "working");

        // A tool is added, the build succeeds, but the resulting app does not start.
        let fresh = dir.path().join("fresh.exe");
        write(&fresh, "does-not-start");
        install(&paths, &fresh).unwrap();

        // The launcher notices and puts things back.
        assert!(can_roll_back(&paths));
        rollback(&paths).unwrap();

        assert_eq!(read(&paths.current), "working");
    }

    #[test]
    fn binary_names_carry_the_platform_suffix() {
        let paths = BinaryPaths::in_dir(Path::new("/install"));

        let current = paths.current.file_name().unwrap().to_string_lossy().to_string();
        let previous = paths.previous.file_name().unwrap().to_string_lossy().to_string();

        assert!(current.starts_with("luna_app"));
        assert!(previous.starts_with("luna_app.prev"));
        assert!(current.ends_with(std::env::consts::EXE_SUFFIX));
        assert!(previous.ends_with(std::env::consts::EXE_SUFFIX));
    }
}
