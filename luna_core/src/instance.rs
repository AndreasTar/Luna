//! Making sure only one Luna runs per install.
//!
//! Luna stays resident, often with no window showing. Without a guard, clicking the
//! shortcut again starts a second copy that shares the same database and settings
//! files, and the two quietly disagree about everything.
//!
//! The lock is a file in the install directory, held open for the life of the process
//! with write access unshared, so the operating system itself refuses a second opener
//! and releases the lock the instant the process dies, however it dies. That matters
//! more than it sounds: a lock that has to be cleaned up on exit is a lock that
//! survives a crash and stops the app from ever starting again.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// Name of the lock file inside the install directory.
const LOCK_FILE: &str = "luna.lock";

/// A held single-instance lock.
///
/// Released when dropped, and by the operating system if the process dies without
/// dropping it.
pub struct InstanceLock {
    path: PathBuf,
    /// Held open for the life of the lock. In an `Option` so [`Drop`] can close it
    /// before removing the file: Windows will not delete a file that still has an
    /// open handle.
    file: Option<File>,
}

impl InstanceLock {
    pub fn path(&self) -> &Path {
        return &self.path;
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        // Close the handle first. The handle closing is what actually frees the lock;
        // removing the file is tidiness, and Windows refuses it while the handle is
        // open. A crash that skips both leaves only an empty file, which the next
        // start truncates.
        self.file.take();

        let _ = std::fs::remove_file(&self.path);
    }
}

/// The result of trying to become the running instance.
pub enum InstanceCheck {
    /// This process is now the one running instance.
    Acquired(InstanceLock),

    /// Another Luna already holds the lock for this install.
    ///
    /// The right response is to bring that one to the front and exit, not to report an
    /// error: the user clicked the shortcut because they wanted Luna, and one is
    /// already there.
    AlreadyRunning {
        /// The other process, if it recorded itself. Advisory only.
        pid: Option<u32>,
    },
}

impl InstanceCheck {
    pub fn is_acquired(&self) -> bool {
        return matches!(self, InstanceCheck::Acquired(_));
    }
}

/// Tries to become the single running instance for an install.
///
/// Two Lunas in *different* install folders are fine and expected: the lock is per
/// install, because that is the unit that shares a database.
pub fn acquire(install_dir: &Path) -> std::io::Result<InstanceCheck> {
    let path = install_dir.join(LOCK_FILE);

    let existing_pid = read_pid(&path);

    return match open_exclusive(&path) {
        Ok(mut file) => {
            // Advisory, for diagnostics. Nothing depends on it being accurate.
            let _ = write!(file, "{}", std::process::id());
            let _ = file.flush();

            Ok(InstanceCheck::Acquired(InstanceLock { path, file: Some(file) }))
        }

        Err(e) if is_locked(&e) => Ok(InstanceCheck::AlreadyRunning { pid: existing_pid }),

        Err(e) => Err(e),
    };
}

/// Opens the lock file so that no second process can open it.
#[cfg(windows)]
fn open_exclusive(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    // FILE_SHARE_READ (1), not zero. Sharing nothing would also stop the *reader* that
    // wants the pid out of this file, so a second instance could not report which
    // process is holding it. Sharing reads still denies a second writer, which is what
    // the lock is for. Windows frees it when the handle closes, including on a hard
    // kill, so there is no stale lock to clean up after a crash.
    const FILE_SHARE_READ: u32 = 1;

    return OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .share_mode(FILE_SHARE_READ)
        .open(path);
}

/// Fallback for platforms without exclusive-open semantics.
///
/// `create_new` fails if the file exists, which is a weaker guarantee: a crash leaves
/// the file behind and the next start has to decide whether the recorded process is
/// still alive. Windows is the target, so this is a courtesy rather than a promise.
#[cfg(not(windows))]
fn open_exclusive(path: &Path) -> std::io::Result<File> {
    return OpenOptions::new().write(true).create_new(true).open(path);
}

/// Whether an error means someone else holds the lock, rather than something being
/// wrong.
fn is_locked(error: &std::io::Error) -> bool {
    if matches!(
        error.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::AlreadyExists
    ) {
        return true;
    }

    #[cfg(windows)]
    {
        // ERROR_SHARING_VIOLATION and ERROR_LOCK_VIOLATION, which is what a second
        // opener actually gets. `std` maps both to `Uncategorized`, so they have to be
        // matched by number rather than by kind.
        const ERROR_SHARING_VIOLATION: i32 = 32;
        const ERROR_LOCK_VIOLATION: i32 = 33;

        if matches!(
            error.raw_os_error(),
            Some(ERROR_SHARING_VIOLATION) | Some(ERROR_LOCK_VIOLATION)
        ) {
            return true;
        }
    }

    return false;
}

/// Reads the pid a previous instance recorded, if any.
fn read_pid(path: &Path) -> Option<u32> {
    return std::fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse().ok());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_instance_takes_the_lock() {
        let dir = tempfile::tempdir().unwrap();

        let check = acquire(dir.path()).unwrap();

        assert!(check.is_acquired());
    }

    #[test]
    fn a_second_instance_is_told_one_is_already_running() {
        let dir = tempfile::tempdir().unwrap();

        let _first = acquire(dir.path()).unwrap();
        let second = acquire(dir.path()).unwrap();

        assert!(
            !second.is_acquired(),
            "a second Luna in the same install must not start"
        );
    }

    #[test]
    fn the_lock_is_released_when_the_first_instance_goes() {
        let dir = tempfile::tempdir().unwrap();

        {
            let first = acquire(dir.path()).unwrap();
            assert!(first.is_acquired());
        }

        // The next start must succeed, or quitting would make Luna unstartable.
        assert!(acquire(dir.path()).unwrap().is_acquired());
    }

    #[test]
    fn separate_installs_do_not_block_each_other() {
        let one = tempfile::tempdir().unwrap();
        let two = tempfile::tempdir().unwrap();

        let _a = acquire(one.path()).unwrap();
        let b = acquire(two.path()).unwrap();

        // The lock is per install because that is the unit sharing a database.
        assert!(b.is_acquired());
    }

    #[test]
    fn the_lock_file_lives_in_the_install_directory() {
        let dir = tempfile::tempdir().unwrap();

        let InstanceCheck::Acquired(lock) = acquire(dir.path()).unwrap() else {
            panic!("should have acquired");
        };

        assert_eq!(lock.path(), dir.path().join(LOCK_FILE));
        assert!(lock.path().exists());
    }

    #[test]
    fn the_running_instance_records_its_pid() {
        let dir = tempfile::tempdir().unwrap();

        let InstanceCheck::Acquired(lock) = acquire(dir.path()).unwrap() else {
            panic!("should have acquired");
        };

        assert_eq!(read_pid(lock.path()), Some(std::process::id()));
    }

    #[test]
    fn a_second_attempt_reports_the_holders_pid() {
        let dir = tempfile::tempdir().unwrap();

        let _first = acquire(dir.path()).unwrap();

        match acquire(dir.path()).unwrap() {
            InstanceCheck::AlreadyRunning { pid } => {
                assert_eq!(pid, Some(std::process::id()));
            }
            InstanceCheck::Acquired(_) => panic!("should not have acquired"),
        }
    }

    #[test]
    fn the_lock_file_is_cleaned_up_on_a_tidy_exit() {
        let dir = tempfile::tempdir().unwrap();
        let path;

        {
            let InstanceCheck::Acquired(lock) = acquire(dir.path()).unwrap() else {
                panic!("should have acquired");
            };
            path = lock.path().to_path_buf();
        }

        assert!(!path.exists());
    }
}
