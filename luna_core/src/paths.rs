//! Where Luna keeps its files.
//!
//! Luna is portable: everything lives next to the executable. Nothing is written to
//! `%APPDATA%`, the registry, or any other system location. The one exception is a
//! `data_root` the user chose themselves, which is recorded in `config/app.toml`.
//!
//! ```text
//! <install>/
//!   luna_app.exe
//!   config/                  anchor, always <install>/config, must be writable
//!     app.toml
//!     tools/<tool-id>.toml
//!   data/                    default data root, may be redirected
//!     luna.db
//!     tools/<tool-id>/
//!   logs/
//!   tools/<name>/            tool sources, developer mode only
//! ```
//!
//! `config/` is the anchor and is never redirected, because it is the file that would
//! record the redirection. If it cannot be written, Luna cannot run portably and says
//! so rather than silently relocating the user's data.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, IoResultExt, Result};

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// Environment variable that overrides the detected install directory.
///
/// This exists for development only. Under `cargo run` the executable sits in
/// `target/debug`, and writing runtime files there is rarely what you want.
pub const INSTALL_DIR_ENV: &str = "LUNA_INSTALL_DIR";

/// Resolved locations for everything Luna reads or writes.
///
/// Build one with [`AppPaths::discover`], or [`AppPaths::rooted_at`] in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    install_dir: PathBuf,
    config_dir: PathBuf,
    data_root: PathBuf,
    logs_dir: PathBuf,
    tools_dir: PathBuf,
}

impl AppPaths {
    /// Locates the install directory and derives every other path from it.
    ///
    /// The `data_root` override is not read here, because reading it requires parsing
    /// `config/app.toml`, which belongs to the config layer. Call
    /// [`AppPaths::with_data_root`] once the config has been loaded.
    ///
    /// ## Returns
    /// Paths anchored at the executable's directory, with `config/`, `data/` and
    /// `logs/` created and verified writable.
    ///
    /// ## Errors
    /// [`CoreError::InstallDirNotWritable`] if `config/` cannot be created or written,
    /// which is fatal: Luna does not fall back to a system location.
    pub fn discover() -> Result<Self> {
        let install_dir = detect_install_dir()?;
        return Self::rooted_at(install_dir);
    }

    /// Builds paths anchored at an explicit directory, creating and verifying them.
    ///
    /// Used by [`AppPaths::discover`] and directly by tests.
    pub fn rooted_at(install_dir: impl Into<PathBuf>) -> Result<Self> {
        let install_dir = install_dir.into();

        let paths = Self {
            config_dir: install_dir.join("config"),
            data_root: install_dir.join("data"),
            logs_dir: install_dir.join("logs"),
            tools_dir: install_dir.join("tools"),
            install_dir,
        };

        // config/ is the anchor. If this fails, nothing else can be recorded, so the
        // failure is reported as the install directory being unusable.
        ensure_writable_dir(&paths.config_dir).map_err(|source| {
            CoreError::InstallDirNotWritable {
                path: paths.install_dir.clone(),
                source,
            }
        })?;

        ensure_writable_dir(&paths.data_root).map_err(|source| {
            CoreError::DataRootNotWritable {
                path: paths.data_root.clone(),
                source,
            }
        })?;

        fs::create_dir_all(&paths.logs_dir).at_path(&paths.logs_dir)?;

        return Ok(paths);
    }

    /// Redirects the data root to a user-chosen directory, verifying it is usable.
    ///
    /// The config directory is deliberately left where it is: it holds the setting
    /// that points here, so it cannot itself be redirected.
    ///
    /// ## Errors
    /// [`CoreError::DataRootNotWritable`] if the directory cannot be created or
    /// written. The caller should surface this and keep using the default rather than
    /// starting up with an unwritable data root.
    pub fn with_data_root(mut self, data_root: impl Into<PathBuf>) -> Result<Self> {
        let data_root = data_root.into();

        ensure_writable_dir(&data_root).map_err(|source| CoreError::DataRootNotWritable {
            path: data_root.clone(),
            source,
        })?;

        self.data_root = data_root;
        return Ok(self);
    }

    /// The directory holding the executable. Everything else derives from this.
    pub fn install_dir(&self) -> &Path {
        return &self.install_dir;
    }

    /// `<install>/config`. Never redirected.
    pub fn config_dir(&self) -> &Path {
        return &self.config_dir;
    }

    /// The data root, either `<install>/data` or the user's chosen directory.
    pub fn data_root(&self) -> &Path {
        return &self.data_root;
    }

    /// `<install>/logs`.
    pub fn logs_dir(&self) -> &Path {
        return &self.logs_dir;
    }

    /// `<install>/tools`, where tool sources live in developer mode.
    pub fn tools_dir(&self) -> &Path {
        return &self.tools_dir;
    }

    /// `config/app.toml`, the global settings file.
    pub fn app_config_file(&self) -> PathBuf {
        return self.config_dir.join("app.toml");
    }

    /// `<data_root>/luna.db`, the SQLite database.
    pub fn database_file(&self) -> PathBuf {
        return self.data_root.join("luna.db");
    }

    /// `config/tools/<tool-id>.toml`, a single tool's settings file.
    ///
    /// ## Errors
    /// [`CoreError::InvalidToolId`] if the id cannot be used as a filename.
    pub fn tool_config_file(&self, tool_id: &str) -> Result<PathBuf> {
        validate_tool_id(tool_id)?;
        return Ok(self.config_dir.join("tools").join(format!("{tool_id}.toml")));
    }

    /// `<data_root>/tools/<tool-id>/`, a single tool's private data directory.
    ///
    /// The directory is not created here; call [`AppPaths::ensure_tool_data_dir`] when
    /// the tool actually needs it, so disabled tools leave no directories behind.
    ///
    /// ## Errors
    /// [`CoreError::InvalidToolId`] if the id cannot be used as a directory name.
    pub fn tool_data_dir(&self, tool_id: &str) -> Result<PathBuf> {
        validate_tool_id(tool_id)?;
        return Ok(self.data_root.join("tools").join(tool_id));
    }

    /// Creates and returns a tool's data directory.
    pub fn ensure_tool_data_dir(&self, tool_id: &str) -> Result<PathBuf> {
        let dir = self.tool_data_dir(tool_id)?;
        fs::create_dir_all(&dir).at_path(&dir)?;
        return Ok(dir);
    }
}

/// Finds the directory the executable lives in, honouring [`INSTALL_DIR_ENV`].
fn detect_install_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os(INSTALL_DIR_ENV) {
        let dir = PathBuf::from(dir);
        if !dir.as_os_str().is_empty() {
            return Ok(dir);
        }
    }

    let exe = std::env::current_exe()
        .map_err(|e| CoreError::InstallDirUnknown(e.to_string()))?;

    return exe
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| CoreError::InstallDirUnknown(format!("{} has no parent", exe.display())));
}

/// Creates a directory and proves it is writable by round-tripping a probe file.
///
/// Checking the read-only attribute is not enough on Windows, where ACLs decide the
/// outcome. Actually writing is the only reliable test.
fn ensure_writable_dir(dir: &Path) -> std::result::Result<(), std::io::Error> {
    fs::create_dir_all(dir)?;

    let probe = dir.join(format!(".luna-write-probe.{}", std::process::id()));
    fs::write(&probe, b"")?;
    let removed = fs::remove_file(&probe);

    // A probe we could write but not delete still proves writability. Leaving it
    // behind is untidy but not a reason to refuse startup.
    if removed.is_err() {
        return Ok(());
    }

    return Ok(());
}

/// Rejects tool ids that would escape their directory or break as filenames.
///
/// Tool ids are first-party and compiled in, so this is not a security boundary. It
/// catches typos before they turn into a file written somewhere surprising.
fn validate_tool_id(id: &str) -> Result<()> {
    let invalid = |reason: &'static str| CoreError::InvalidToolId {
        id: id.to_string(),
        reason,
    };

    if id.is_empty() {
        return Err(invalid("must not be empty"));
    }

    if id.len() > 96 {
        return Err(invalid("must be 96 characters or fewer"));
    }

    if id == "." || id == ".." {
        return Err(invalid("must not be a relative path component"));
    }

    if id.starts_with('.') {
        return Err(invalid("must not start with a dot"));
    }

    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-';
    if !id.chars().all(allowed) {
        return Err(invalid(
            "may only contain ASCII letters, digits, dot, underscore and hyphen",
        ));
    }

    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_every_path_from_the_install_dir() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::rooted_at(dir.path()).unwrap();

        assert_eq!(paths.install_dir(), dir.path());
        assert_eq!(paths.config_dir(), dir.path().join("config"));
        assert_eq!(paths.data_root(), dir.path().join("data"));
        assert_eq!(paths.logs_dir(), dir.path().join("logs"));
        assert_eq!(paths.tools_dir(), dir.path().join("tools"));
        assert_eq!(paths.database_file(), dir.path().join("data").join("luna.db"));
    }

    #[test]
    fn creates_the_directories_it_needs() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::rooted_at(dir.path()).unwrap();

        assert!(paths.config_dir().is_dir());
        assert!(paths.data_root().is_dir());
        assert!(paths.logs_dir().is_dir());
    }

    #[test]
    fn leaves_no_probe_files_behind() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::rooted_at(dir.path()).unwrap();

        let leftover: Vec<_> = fs::read_dir(paths.config_dir())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();

        assert!(leftover.is_empty(), "probe file left behind: {leftover:?}");
    }

    #[test]
    fn redirects_the_data_root_but_not_the_config_dir() {
        let install = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();

        let paths = AppPaths::rooted_at(install.path())
            .unwrap()
            .with_data_root(elsewhere.path())
            .unwrap();

        assert_eq!(paths.data_root(), elsewhere.path());
        assert_eq!(paths.database_file(), elsewhere.path().join("luna.db"));

        // The config anchor stays put, because it holds the redirection itself.
        assert_eq!(paths.config_dir(), install.path().join("config"));
    }

    #[test]
    fn builds_per_tool_paths() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::rooted_at(dir.path()).unwrap();

        assert_eq!(
            paths.tool_config_file("luna.calendar").unwrap(),
            dir.path().join("config").join("tools").join("luna.calendar.toml")
        );
        assert_eq!(
            paths.tool_data_dir("luna.calendar").unwrap(),
            dir.path().join("data").join("tools").join("luna.calendar")
        );
    }

    #[test]
    fn tool_data_dir_is_created_only_on_demand() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::rooted_at(dir.path()).unwrap();

        assert!(!paths.tool_data_dir("luna.calendar").unwrap().exists());

        let created = paths.ensure_tool_data_dir("luna.calendar").unwrap();
        assert!(created.is_dir());
    }

    #[test]
    fn accepts_reasonable_tool_ids() {
        for id in ["luna.calendar", "a", "tool-1", "tool_1", "Luna.Img2Ascii"] {
            assert!(validate_tool_id(id).is_ok(), "rejected {id:?}");
        }
    }

    #[test]
    fn rejects_tool_ids_that_would_escape_their_directory() {
        for id in ["", ".", "..", "../evil", "a/b", "a\\b", ".hidden", "with space", "emoji.\u{1f600}"] {
            assert!(validate_tool_id(id).is_err(), "accepted {id:?}");
        }
    }

    #[test]
    fn rejects_overlong_tool_ids() {
        let long = "a".repeat(97);
        assert!(validate_tool_id(&long).is_err());
    }
}
