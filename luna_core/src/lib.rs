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
pub mod events;
pub mod manifest;
pub mod lifecycle;
pub mod palettes;
pub mod paths;
pub mod registry;
pub mod scheduler;

pub use config::{AppConfig, LoadOutcome, ToolConfig, UiStateTtl};
pub use db::Database;
pub use error::{CoreError, Result};
pub use events::{EventLog, RuleEvent};
pub use manifest::{PortType, ToolManifest};
pub use palettes::PaletteSet;
pub use paths::AppPaths;
pub use registry::{Registry, ServiceContext, ServiceFactory, SidebarEntry, ToolService};
pub use scheduler::{Fire, ScheduledJob, Scheduler, Upcoming};

use std::path::PathBuf;

/// Version of the host service layer as a whole.
pub const VERSION: luna::Version = luna::Version::new(0, 2, 0);

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

    /// A tool was enabled in its settings but its service refused to start.
    ///
    /// The tool is left disabled and the rest of the app comes up normally, because
    /// one broken tool should not stop Luna from starting.
    ToolFailedToStart {
        /// The tool that could not start.
        id: String,
        /// Why it could not start.
        reason: String,
    },

    /// A palette file could not be used.
    ///
    /// Surfaced rather than skipped: the user wrote that file and is waiting to see it
    /// in the picker, so silence would look like the app ignoring them.
    PaletteUnusable {
        /// The palette file that failed to load.
        path: PathBuf,
        /// What was wrong with it.
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
            Notice::ToolFailedToStart { id, reason } => write!(
                f,
                "The tool {id} could not start ({reason}) and has been disabled."
            ),
            Notice::PaletteUnusable { path, reason } => write!(
                f,
                "The palette {} {reason}, so it is not available.",
                path.display()
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
    /// Every compiled-in tool, and which of them are running.
    pub registry: Registry,
    /// Palettes loaded from `<install>/palettes`.
    pub palettes: PaletteSet,
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

        let palettes = PaletteSet::load_dir(paths.palettes_dir());

        for problem in &palettes.problems {
            notices.push(Notice::PaletteUnusable {
                path: problem.path.clone(),
                reason: problem.reason.clone(),
            });
        }

        let host = Self {
            paths,
            config,
            db,
            registry: Registry::new(),
            palettes,
            notices,
        };

        // Write the defaults out on first run rather than waiting for a clean exit.
        // Luna is meant to be killed rather than closed, and its config is meant to be
        // hand-editable, so an install with no app.toml to look at is a poor start.
        if !host.paths.app_config_file().exists() {
            host.save_config()?;
        }

        return Ok(host);
    }

    /// Adds a tool and loads its saved settings.
    ///
    /// Nothing starts yet. Register every tool first, then call [`Host::start_tools`]
    /// once, so a failure in one tool cannot leave the registry half-built.
    ///
    /// `factory` builds the tool's background service, and is required if the manifest
    /// declares `background = true`.
    pub fn register_tool(
        &mut self,
        manifest: ToolManifest,
        factory: Option<ServiceFactory>,
    ) -> Result<()> {
        let id = manifest.id.clone();

        self.registry.register(manifest, factory)?;

        let loaded = self.tool_config(&id)?;

        if let Some(moved_to) = loaded.quarantined {
            self.notices.push(Notice::ConfigQuarantined {
                original: self.paths.tool_config_file(&id)?,
                moved_to,
            });
        }

        self.registry.apply_config(&id, loaded.value)?;

        return Ok(());
    }

    /// Starts the services of every tool whose settings say it is enabled.
    ///
    /// Tools that fail to start are disabled and reported through [`Host::notices`]
    /// rather than aborting startup.
    pub fn start_tools(&mut self) {
        for (id, error) in self.registry.start_enabled(&self.paths) {
            self.notices.push(Notice::ToolFailedToStart {
                id,
                reason: error.to_string(),
            });
        }
    }

    /// Enables or disables a tool and persists the change.
    ///
    /// Starts or stops the tool's service as needed, so this is the whole of what
    /// toggling a tool means.
    pub fn set_tool_enabled(&mut self, tool_id: &str, enabled: bool) -> Result<()> {
        if enabled {
            self.registry.enable(tool_id, &self.paths)?;
        } else {
            self.registry.disable(tool_id)?;
        }

        let mut config = self.tool_config(tool_id)?.value;
        config.enabled = enabled;

        self.save_tool_config(tool_id, &config)?;
        self.registry.apply_config(tool_id, config)?;

        return Ok(());
    }

    /// Stops every running service, giving each a chance to flush.
    ///
    /// Called on the way out. Errors are collected rather than propagated, because one
    /// tool failing to flush must not prevent the others from being asked.
    pub fn stop_tools(&mut self) -> Vec<(String, CoreError)> {
        let running: Vec<String> = self
            .registry
            .enabled()
            .map(|m| m.id.clone())
            .collect();

        let mut failures = Vec::new();

        for id in running {
            // Disabling persists nothing here: this is shutdown, and the tool should
            // still be enabled the next time Luna starts.
            if let Err(e) = self.registry.disable(&id) {
                failures.push((id, e));
            }
        }

        return failures;
    }

    /// Writes the current application settings to disk.
    pub fn save_config(&self) -> Result<()> {
        return self.config.save(&self.paths.app_config_file());
    }

    /// The application palette named by the settings.
    ///
    /// Falls back rather than failing, so a typo in `app.toml` or an empty palettes
    /// folder produces a plain app rather than an unstyled one.
    pub fn app_palette(&self) -> luna::palette::Palette {
        return self.palettes.app_palette(&self.config.palette);
    }

    /// The finished palette for one tool, with its overrides applied.
    ///
    /// Tools with no settings of their own get the application palette unchanged.
    pub fn palette_for_tool(&self, tool_id: &str) -> luna::palette::Resolved {
        let app = self.app_palette();

        let config = self
            .registry
            .config(tool_id)
            .cloned()
            .unwrap_or_default();

        return self.palettes.resolve_for_tool(&app, &config);
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

        assert_eq!(top_level, vec!["config", "data", "logs", "palettes"]);
    }

    #[test]
    fn first_run_leaves_an_editable_config_behind() {
        let dir = tempfile::tempdir().unwrap();
        let host = host_in(dir.path());

        let path = host.paths.app_config_file();
        assert!(path.exists(), "app.toml should exist after the first bootstrap");

        // And it round-trips, so what the user opens is what Luna will read back.
        let reloaded = AppConfig::load(&path).unwrap();
        assert_eq!(reloaded.value, AppConfig::default());
        assert_eq!(reloaded.quarantined, None);
    }

    #[test]
    fn bootstrap_does_not_overwrite_an_existing_config() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut host = host_in(dir.path());
            host.config.palette = "daylight".to_string();
            host.save_config().unwrap();
        }

        let host = host_in(dir.path());
        assert_eq!(host.config.palette, "daylight");
    }

    #[test]
    fn settings_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut host = host_in(dir.path());
            host.config.last_active_tool = Some("luna.calendar".to_string());
            host.config.palette = "daylight".to_string();
            host.save_config().unwrap();
        }

        let host = host_in(dir.path());

        assert_eq!(host.config.last_active_tool, Some("luna.calendar".to_string()));
        assert_eq!(host.config.palette, "daylight");
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

    fn test_manifest(id: &str) -> ToolManifest {
        return ToolManifest::from_toml(&format!(
            "id = \"{id}\"\nname = \"Test\"\nversion = \"1.0.0\"\n"
        ))
        .unwrap();
    }

    #[test]
    fn newly_registered_tools_default_to_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = host_in(dir.path());

        host.register_tool(test_manifest("luna.fresh"), None).unwrap();
        host.start_tools();

        assert!(host.registry.is_enabled("luna.fresh").unwrap());
        assert!(host.notices.is_empty(), "{:?}", host.notices);
    }

    #[test]
    fn toggling_a_tool_persists_across_a_restart() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut host = host_in(dir.path());
            host.register_tool(test_manifest("luna.toggle"), None).unwrap();
            host.start_tools();

            host.set_tool_enabled("luna.toggle", false).unwrap();
            assert!(!host.registry.is_enabled("luna.toggle").unwrap());
        }

        let mut host = host_in(dir.path());
        host.register_tool(test_manifest("luna.toggle"), None).unwrap();
        host.start_tools();

        assert!(
            !host.registry.is_enabled("luna.toggle").unwrap(),
            "a disabled tool must stay disabled after a restart"
        );

        // And back on again.
        host.set_tool_enabled("luna.toggle", true).unwrap();
        assert!(host.registry.is_enabled("luna.toggle").unwrap());
    }

    #[test]
    fn a_disabled_tool_is_absent_from_the_sidebar() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = host_in(dir.path());

        host.register_tool(test_manifest("luna.shown"), None).unwrap();
        host.register_tool(test_manifest("luna.hidden"), None).unwrap();
        host.start_tools();

        host.set_tool_enabled("luna.hidden", false).unwrap();

        let ids: Vec<String> = host
            .registry
            .sidebar_entries()
            .into_iter()
            .map(|e| e.id)
            .collect();

        assert_eq!(ids, vec!["luna.shown".to_string()]);
    }

    #[test]
    fn registering_the_same_id_twice_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = host_in(dir.path());

        host.register_tool(test_manifest("luna.dup"), None).unwrap();

        assert!(matches!(
            host.register_tool(test_manifest("luna.dup"), None),
            Err(CoreError::DuplicateTool { .. })
        ));
    }
}
