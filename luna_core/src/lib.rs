//! # Luna Core
//!
//! Host services for the Luna desktop app. This crate owns everything that exists
//! once for the whole application rather than once per tool: where files live, how
//! they are written, settings, and the database.
//!
//! It is deliberately separate from `luna_lib`, which is published and must stay pure
//! logic, and from `luna_src`, which owns the Slint UI. Nothing here depends on Slint,
//! so host services can run with no window allocated.
//!
//! ## What this crate guarantees
//!
//! * **Portability.** Every path is anchored to the executable's directory. Nothing is
//!   written to `%APPDATA%`, the registry or any other system location, except a
//!   `data_root` the user chose themselves. See [`paths`].
//! * **Crash safety.** Every file write is atomic, and the database runs in WAL mode.
//!   Luna is designed never to close, so it will be killed rather than shut down.
//!   See [`atomic`].
//! * **Startup always succeeds.** A missing, corrupt or future-version config is
//!   recovered from rather than treated as fatal, and reported through
//!   [`Host::notices`].
//!
//! ## Getting started
//!
//! ```no_run
//! # fn main() -> Result<(), luna_core::CoreError> {
//! let host = luna_core::Host::bootstrap()?;
//!
//! for notice in &host.notices {
//!     eprintln!("startup: {notice}");
//! }
//!
//! println!("data lives in {}", host.paths.data_root().display());
//! # Ok(())
//! # }
//! ```

pub mod atomic;
pub mod config;
pub mod db;
pub mod error;
pub mod paths;

pub use config::{AppConfig, LoadOutcome, ToolConfig, UiStateTtl};
pub use db::Database;
pub use error::{CoreError, Result};
pub use paths::AppPaths;

use std::path::PathBuf;

/// Version of the host service layer as a whole.
pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// Something that happened during startup which the user should be told about.
///
/// None of these are fatal. Luna recovers and carries on, but silently discarding a
/// user's settings or writing their data somewhere they did not ask for would be
/// worse than saying so.
#[derive(Debug, Clone, PartialEq)]
pub enum Notice {
    /// A settings file could not be parsed. It was moved aside and defaults are in
    /// use, so the user's settings have just reverted.
    ConfigQuarantined {
        /// The config that could not be read.
        original: PathBuf,
        /// Where the unreadable file was moved to.
        moved_to: PathBuf,
    },

    /// The configured `data_root` could not be used, so the default is in use instead.
    ///
    /// Reported rather than fixed, because writing the user's data somewhere other
    /// than where they asked is not a decision to make on their behalf.
    DataRootUnusable {
        /// The directory that was configured but is not usable.
        configured: PathBuf,
        /// The directory actually in use.
        using: PathBuf,
        /// Why the configured directory was rejected.
        reason: String,
    },
}

impl std::fmt::Display for Notice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Notice::ConfigQuarantined { original, moved_to } => write!(
                f,
                "{} could not be read and was moved to {}. Default settings are in use.",
                original.display(),
                moved_to.display()
            ),
            Notice::DataRootUnusable { configured, using, reason } => write!(
                f,
                "The configured data folder {} is not usable ({reason}). Using {} instead.",
                configured.display(),
                using.display()
            ),
        };
    }
}

/// The host services, resolved and ready to use.
///
/// Built by [`Host::bootstrap`]. Later steps of the roadmap add the tool registry,
/// the scheduler and the port bus alongside these fields.
pub struct Host {
    /// Where everything lives.
    pub paths: AppPaths,
    /// Application-wide settings.
    pub config: AppConfig,
    /// The open, migrated database.
    pub db: Database,
    /// Recoverable problems found during startup, for the UI to surface.
    pub notices: Vec<Notice>,
}

impl Host {
    /// Resolves paths, loads settings, and opens the database.
    ///
    /// Recoverable problems become [`Notice`]s rather than errors. The only failures
    /// that abort startup are ones with no safe recovery: an unwritable install
    /// directory, or a database that cannot be opened or migrated.
    pub fn bootstrap() -> Result<Self> {
        let paths = AppPaths::discover()?;
        return Self::bootstrap_at(paths);
    }

    /// Same as [`Host::bootstrap`] but with paths supplied, for tests and for the
    /// launcher, which already knows where it put things.
    pub fn bootstrap_at(paths: AppPaths) -> Result<Self> {
        let mut notices = Vec::new();

        let config_path = paths.app_config_file();
        let loaded = AppConfig::load(&config_path)?;

        if let Some(moved_to) = loaded.quarantined {
            notices.push(Notice::ConfigQuarantined {
                original: config_path,
                moved_to,
            });
        }

        let config = loaded.value;

        // Honour a user-chosen data root, but never let a bad one stop startup: fall
        // back to the default and say so.
        let paths = match &config.data_root {
            None => paths,
            Some(configured) => {
                let default_root = paths.data_root().to_path_buf();
                match paths.clone().with_data_root(configured) {
                    Ok(redirected) => redirected,
                    Err(e) => {
                        notices.push(Notice::DataRootUnusable {
                            configured: configured.clone(),
                            using: default_root,
                            reason: e.to_string(),
                        });
                        paths
                    }
                }
            }
        };

        let db = Database::open(&paths.database_file())?;

        return Ok(Self { paths, config, db, notices });
    }

    /// Writes the current application settings to disk.
    pub fn save_config(&self) -> Result<()> {
        return self.config.save(&self.paths.app_config_file());
    }

    /// Loads one tool's settings, falling back to defaults if missing or broken.
    ///
    /// A quarantined tool config is reported in the returned [`LoadOutcome`] rather
    /// than pushed onto [`Host::notices`], because tool configs are loaded lazily and
    /// the caller is better placed to decide how to surface it.
    pub fn tool_config(&self, tool_id: &str) -> Result<LoadOutcome<ToolConfig>> {
        let path = self.paths.tool_config_file(tool_id)?;
        return ToolConfig::load(&path);
    }

    /// Writes one tool's settings to disk.
    pub fn save_tool_config(&self, tool_id: &str, config: &ToolConfig) -> Result<()> {
        let path = self.paths.tool_config_file(tool_id)?;
        return config.save(&path);
    }
}

/// Seconds since the Unix epoch, saturating at 0 if the system clock predates it.
///
/// Used for quarantine filenames and migration timestamps, neither of which needs
/// more precision or a calendar dependency.
pub(crate) fn epoch_seconds() -> i64 {
    return std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_in(dir: &std::path::Path) -> Host {
        let paths = AppPaths::rooted_at(dir).unwrap();
        return Host::bootstrap_at(paths).unwrap();
    }

    #[test]
    fn bootstraps_into_an_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let host = host_in(dir.path());

        assert!(host.notices.is_empty(), "unexpected notices: {:?}", host.notices);
        assert_eq!(host.config, AppConfig::default());
        assert!(host.paths.database_file().exists());
        assert_eq!(
            host.db.schema_version().unwrap(),
            Database::expected_schema_version()
        );
    }

    #[test]
    fn writes_nothing_outside_the_install_directory() {
        let dir = tempfile::tempdir().unwrap();
        let host = host_in(dir.path());
        host.save_config().unwrap();

        let mut top_level: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        top_level.sort();

        assert_eq!(top_level, vec!["config", "data", "logs"]);
    }

    #[test]
    fn settings_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut host = host_in(dir.path());
            host.config.last_active_tool = Some("luna.calendar".to_string());
            host.config.dark_mode = false;
            host.save_config().unwrap();
        }

        let host = host_in(dir.path());

        assert_eq!(host.config.last_active_tool, Some("luna.calendar".to_string()));
        assert_eq!(host.config.dark_mode, false);
    }

    #[test]
    fn database_contents_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();

        host_in(dir.path()).db.meta_set("survives", "yes").unwrap();

        let host = host_in(dir.path());
        assert_eq!(host.db.meta_get("survives").unwrap(), Some("yes".to_string()));
    }

    #[test]
    fn corrupt_app_config_is_reported_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::rooted_at(dir.path()).unwrap();
        std::fs::write(paths.app_config_file(), "= not toml =").unwrap();

        let host = Host::bootstrap_at(paths).unwrap();

        assert_eq!(host.config, AppConfig::default());
        assert!(matches!(
            host.notices.as_slice(),
            [Notice::ConfigQuarantined { .. }]
        ));
    }

    #[test]
    fn honours_a_configured_data_root() {
        let install = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();

        {
            let mut host = host_in(install.path());
            host.config.data_root = Some(elsewhere.path().to_path_buf());
            host.save_config().unwrap();
        }

        let host = host_in(install.path());

        assert!(host.notices.is_empty(), "unexpected notices: {:?}", host.notices);
        assert_eq!(host.paths.data_root(), elsewhere.path());
        assert!(elsewhere.path().join("luna.db").exists());

        // The config anchor stays in the install directory regardless.
        assert_eq!(host.paths.config_dir(), install.path().join("config"));
    }

    #[test]
    fn falls_back_and_reports_when_the_data_root_is_unusable() {
        let install = tempfile::tempdir().unwrap();

        // A path whose parent is a file cannot become a directory.
        let blocker = install.path().join("not-a-dir");
        std::fs::write(&blocker, b"").unwrap();
        let unusable = blocker.join("data");

        {
            let mut host = host_in(install.path());
            host.config.data_root = Some(unusable.clone());
            host.save_config().unwrap();
        }

        let host = host_in(install.path());

        assert_eq!(host.paths.data_root(), install.path().join("data"));
        assert!(
            matches!(
                host.notices.as_slice(),
                [Notice::DataRootUnusable { configured, .. }] if configured == &unusable
            ),
            "expected a DataRootUnusable notice, got {:?}",
            host.notices
        );
    }

    #[test]
    fn tool_config_round_trips_through_the_host() {
        let dir = tempfile::tempdir().unwrap();
        let host = host_in(dir.path());

        assert_eq!(host.tool_config("luna.calendar").unwrap().value, ToolConfig::default());

        let mut cfg = ToolConfig::default();
        cfg.enabled = false;
        cfg.ui_state_ttl = UiStateTtl::Never;
        host.save_tool_config("luna.calendar", &cfg).unwrap();

        assert_eq!(host.tool_config("luna.calendar").unwrap().value, cfg);
    }

    #[test]
    fn rejects_tool_ids_that_would_escape_the_config_directory() {
        let dir = tempfile::tempdir().unwrap();
        let host = host_in(dir.path());

        assert!(matches!(
            host.tool_config("../escape"),
            Err(CoreError::InvalidToolId { .. })
        ));
    }
}
