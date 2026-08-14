//! Crash-safe file writes.
//!
//! Luna is designed never to close, which means it will eventually be killed rather
//! than shut down. Every file write therefore goes through [`write`], which makes a
//! torn or half-written file impossible: the content lands in a temporary file in the
//! same directory, is flushed to disk, and is then renamed over the target. A rename
//! within one directory is atomic on both Windows and Unix, so a reader sees either
//! the complete old file or the complete new one.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{IoResultExt, Result};

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// Writes `contents` to `path` atomically, creating parent directories as needed.
///
/// If the write fails at any point, the target file is left untouched and the
/// temporary file is cleaned up.
///
/// ## Arguments
/// * `path` - the file to write. Its parent directory is created if missing.
/// * `contents` - the bytes to write.
///
/// ## Returns
/// `Ok(())` if the new content is durably in place.
pub fn write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        // A bare filename: treat the current directory as the parent.
        _ => Path::new("."),
    };

    fs::create_dir_all(parent).at_path(parent)?;

    let temp_path = temp_path_for(path);

    // Scoped so the handle is closed before the rename. Windows will not rename a
    // file that still has an open handle.
    {
        let mut file = fs::File::create(&temp_path).at_path(&temp_path)?;

        if let Err(e) = file.write_all(contents) {
            let _ = fs::remove_file(&temp_path);
            return Err(e).at_path(&temp_path);
        }

        // Flush the userspace buffer and then the OS buffer. Without the sync, a
        // power loss right after the rename can leave a correctly named but empty
        // file, which is the exact failure this module exists to prevent.
        if let Err(e) = file.flush().and_then(|_| file.sync_all()) {
            let _ = fs::remove_file(&temp_path);
            return Err(e).at_path(&temp_path);
        }
    }

    if let Err(e) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(e).at_path(path);
    }

    return Ok(());
}

/// Writes a string to `path` atomically. Convenience wrapper around [`write`].
pub fn write_string(path: &Path, contents: &str) -> Result<()> {
    return write(path, contents.as_bytes());
}

/// Builds a temporary path next to `path`, in the same directory.
///
/// Being in the same directory matters: a rename is only atomic within a single
/// filesystem, and the system temp directory is frequently on another volume.
///
/// The name mixes the process id with a monotonic counter so that two writers in the
/// same process, or two Luna instances, cannot collide on it.
fn temp_path_for(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();

    let stem = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "luna".to_string());

    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };

    return parent.join(format!(".{stem}.tmp.{pid}.{seq}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");

        write_string(&path, "hello").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn replaces_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");

        write_string(&path, "first").unwrap();
        write_string(&path, "second").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
    }

    #[test]
    fn creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a").join("b").join("c.txt");

        write_string(&path, "nested").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "nested");
    }

    #[test]
    fn leaves_no_temp_files_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");

        write_string(&path, "hello").unwrap();

        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(entries, vec!["hello.txt".to_string()]);
    }

    #[test]
    fn temp_paths_are_unique_and_local() {
        let target = Path::new("/some/dir/file.toml");

        let a = temp_path_for(target);
        let b = temp_path_for(target);

        assert_ne!(a, b);
        assert_eq!(a.parent(), target.parent());
        assert_eq!(b.parent(), target.parent());
    }

    #[test]
    fn handles_empty_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");

        write(&path, b"").unwrap();

        assert_eq!(fs::read(&path).unwrap(), Vec::<u8>::new());
    }
}
