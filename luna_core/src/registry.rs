//! The tool registry.
//!
//! Holds every tool the binary was compiled with, its manifest, its settings, and its
//! background service if it has one running. Tools are enabled and disabled here at
//! runtime, which is separate from being compiled in: everything compiled is present,
//! and enabling decides whether a service runs and whether the tool appears in the
//! sidebar and in port lookups.
//!
//! The registry is pure in-memory state and performs no IO. Persisting the enabled
//! flag is [`crate::Host`]'s job, so that everything which touches the filesystem
//! stays in one place.
//!
//! ## Services and views
//!
//! A tool has up to two halves. The service half lives here: it starts when the tool
//! is enabled, stops when it is disabled, and never touches the UI, which is what lets
//! a reminder fire while a different tool's page is on screen.
//!
//! The view half is deliberately **not** here. Views are Slint components, and this
//! crate stays free of a Slint dependency so host services can run with no window
//! allocated. The UI layer keeps its own map from tool id to view, and asks the
//! registry only what to show and in what order.

use std::collections::BTreeMap;

use crate::config::ToolConfig;
use crate::error::{CoreError, Result};
use crate::manifest::{PortType, ToolManifest};
use crate::paths::AppPaths;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// The background half of a tool.
///
/// Created when the tool is enabled and dropped when it is disabled. Implementors own
/// the tool's state and, once the scheduler lands, will handle its scheduled work and
/// inbound messages.
///
/// `Send` is required because services run off the UI thread.
pub trait ToolService: Send {
    /// Called once, just after the service is created.
    ///
    /// Returning an error leaves the tool disabled rather than half-started.
    fn start(&mut self, ctx: &ServiceContext<'_>) -> Result<()> {
        let _ = ctx;
        return Ok(());
    }

    /// Asked whether Luna may close.
    ///
    /// The default allows it. Override when the tool can be mid-flight in a way that
    /// stopping would corrupt, such as a conversion writing its output.
    ///
    /// A tool cannot refuse outright: the strongest answer holds the door briefly, so
    /// a wedged tool can never trap the user in the app.
    fn on_shutdown_request(&mut self) -> crate::shutdown::ShutdownVote {
        return crate::shutdown::ShutdownVote::Allow;
    }

    /// Called once, before the service is dropped.
    ///
    /// The last chance to flush state. Errors are reported but do not prevent the
    /// service being dropped, since the alternative is a tool that cannot be turned
    /// off.
    fn stop(&mut self) -> Result<()> {
        return Ok(());
    }
}

/// What a service is given when it starts.
pub struct ServiceContext<'a> {
    /// The id of the tool this service belongs to.
    pub tool_id: &'a str,
    /// Where the tool may read and write. Use [`AppPaths::ensure_tool_data_dir`].
    pub paths: &'a AppPaths,
    /// The tool's current settings.
    pub config: &'a ToolConfig,
    /// Where the shared database lives.
    ///
    /// A path rather than an open connection, deliberately. `rusqlite::Connection` is
    /// `Send` but not `Sync`, and services run off the UI thread and eventually on
    /// threads of their own, so a shared handle would have to be behind a lock that
    /// every tool contends on. Each service opens its own connection to the same file
    /// instead; WAL mode is built for exactly that, allowing concurrent readers
    /// alongside one writer.
    ///
    /// Open one with [`crate::Database::open`], which applies the same pragmas and
    /// runs any outstanding migrations.
    pub database: &'a std::path::Path,
}

/// Builds a fresh service for a tool.
///
/// A factory rather than an instance, because disabling a tool drops its service
/// entirely and re-enabling has to build a new one.
pub type ServiceFactory = Box<dyn Fn() -> Box<dyn ToolService> + Send + Sync>;

/// One tool as the registry sees it.
struct ToolEntry {
    manifest: ToolManifest,
    config: ToolConfig,
    factory: Option<ServiceFactory>,
    service: Option<Box<dyn ToolService>>,
}

/// Everything the sidebar needs to draw one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarEntry {
    pub id: String,
    pub name: String,
    pub category: String,
}

/// Every compiled-in tool, and which of them are running.
#[derive(Default)]
pub struct Registry {
    /// Keyed by tool id. A `BTreeMap` keeps iteration order stable and independent of
    /// registration order, which matters because the sidebar is built from it.
    entries: BTreeMap<String, ToolEntry>,
}

impl Registry {
    pub fn new() -> Self {
        return Self::default();
    }

    /// Adds a tool.
    ///
    /// The tool starts disabled regardless of its settings. Call
    /// [`Registry::apply_config`] to load its saved state, then
    /// [`Registry::start_enabled`] to bring services up, so that a failure to start
    /// one tool cannot leave the registry half-built.
    ///
    /// ## Errors
    /// [`CoreError::DuplicateTool`] if the id is already registered, and whatever
    /// [`ToolManifest::validate`] rejects.
    pub fn register(
        &mut self,
        manifest: ToolManifest,
        factory: Option<ServiceFactory>,
    ) -> Result<()> {
        manifest.validate()?;

        if self.entries.contains_key(&manifest.id) {
            return Err(CoreError::DuplicateTool { id: manifest.id });
        }

        if manifest.background && factory.is_none() {
            return Err(CoreError::InvalidManifest {
                id: manifest.id,
                reason: "declares background = true but registered no service factory"
                    .to_string(),
            });
        }

        let mut config = ToolConfig::default();
        // Nothing is running yet, so the saved enabled flag is applied later by
        // apply_config. Starting disabled keeps registration free of side effects.
        config.enabled = false;

        self.entries.insert(
            manifest.id.clone(),
            ToolEntry { manifest, config, factory, service: None },
        );

        return Ok(());
    }

    /// Loads a tool's saved settings without starting or stopping anything.
    pub fn apply_config(&mut self, tool_id: &str, config: ToolConfig) -> Result<()> {
        let entry = self.entry_mut(tool_id)?;
        entry.config = config;
        return Ok(());
    }

    /// Starts services for every tool whose settings say it is enabled.
    ///
    /// Returns the ids that failed to start, paired with the error. A tool that fails
    /// to start is left disabled and the rest still come up, because one broken tool
    /// should not stop the app.
    pub fn start_enabled(&mut self, paths: &AppPaths) -> Vec<(String, CoreError)> {
        let wanted: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| e.config.enabled && e.service.is_none())
            .map(|(id, _)| id.clone())
            .collect();

        let mut failures = Vec::new();

        for id in wanted {
            if let Err(e) = self.start_service(&id, paths) {
                if let Ok(entry) = self.entry_mut(&id) {
                    entry.config.enabled = false;
                }
                failures.push((id, e));
            }
        }

        return failures;
    }

    /// Enables a tool, starting its service if it has one.
    ///
    /// Idempotent. On failure the tool is left disabled with no service.
    pub fn enable(&mut self, tool_id: &str, paths: &AppPaths) -> Result<()> {
        if self.is_enabled(tool_id)? {
            return Ok(());
        }

        self.start_service(tool_id, paths)?;
        self.entry_mut(tool_id)?.config.enabled = true;

        return Ok(());
    }

    /// Disables a tool, stopping and dropping its service.
    ///
    /// Idempotent. A service that errors on stop is still dropped and the tool is
    /// still disabled, since refusing to turn a tool off is worse than a failed flush;
    /// the error is returned so the caller can report it.
    pub fn disable(&mut self, tool_id: &str) -> Result<()> {
        let entry = self.entry_mut(tool_id)?;
        entry.config.enabled = false;

        let stop_result = match entry.service.as_mut() {
            Some(service) => service.stop(),
            None => Ok(()),
        };

        entry.service = None;

        return stop_result;
    }

    /// Builds and starts a service, leaving the enabled flag alone.
    fn start_service(&mut self, tool_id: &str, paths: &AppPaths) -> Result<()> {
        let entry = self.entry_mut(tool_id)?;

        let factory = match entry.factory.as_ref() {
            Some(f) => f,
            // Tools without a background half are enabled by having their config say
            // so; there is nothing to run.
            None => return Ok(()),
        };

        let mut service = factory();

        let database = paths.database_file();

        let ctx = ServiceContext {
            tool_id: &entry.manifest.id,
            paths,
            config: &entry.config,
            database: &database,
        };

        service.start(&ctx)?;
        entry.service = Some(service);

        return Ok(());
    }

    /// Whether the tool is currently enabled.
    pub fn is_enabled(&self, tool_id: &str) -> Result<bool> {
        return Ok(self.entry(tool_id)?.config.enabled);
    }

    /// Whether the tool currently has a running service.
    ///
    /// Not the same as being enabled: a tool with no background half is enabled and
    /// has no service.
    pub fn has_running_service(&self, tool_id: &str) -> Result<bool> {
        return Ok(self.entry(tool_id)?.service.is_some());
    }

    /// A tool's manifest.
    pub fn manifest(&self, tool_id: &str) -> Result<&ToolManifest> {
        return Ok(&self.entry(tool_id)?.manifest);
    }

    /// A tool's current settings.
    pub fn config(&self, tool_id: &str) -> Result<&ToolConfig> {
        return Ok(&self.entry(tool_id)?.config);
    }

    /// Whether a tool with this id is compiled in, enabled or not.
    pub fn contains(&self, tool_id: &str) -> bool {
        return self.entries.contains_key(tool_id);
    }

    /// How many tools are compiled in.
    pub fn len(&self) -> usize {
        return self.entries.len();
    }

    pub fn is_empty(&self) -> bool {
        return self.entries.is_empty();
    }

    /// Every registered tool, enabled or not, in id order.
    pub fn all(&self) -> impl Iterator<Item = &ToolManifest> {
        return self.entries.values().map(|e| &e.manifest);
    }

    /// Only the enabled tools, in id order.
    pub fn enabled(&self) -> impl Iterator<Item = &ToolManifest> {
        return self
            .entries
            .values()
            .filter(|e| e.config.enabled)
            .map(|e| &e.manifest);
    }

    /// The sidebar contents: enabled tools grouped by category, alphabetical within
    /// each group.
    ///
    /// Ordering is derived rather than configurable. A user-defined order is worth
    /// having eventually, but it needs somewhere to live in config and a way to
    /// reorder in the UI, neither of which exists yet.
    pub fn sidebar_entries(&self) -> Vec<SidebarEntry> {
        let mut entries: Vec<SidebarEntry> = self
            .enabled()
            .map(|m| SidebarEntry {
                id: m.id.clone(),
                name: m.name.clone(),
                category: m.category.clone(),
            })
            .collect();

        entries.sort_by(|a, b| {
            a.category
                .cmp(&b.category)
                .then_with(|| a.name.cmp(&b.name))
        });

        return entries;
    }

    /// Enabled tools that accept the given payload type.
    ///
    /// This is the lookup behind "Send to": a sender asks which tools accept a type
    /// and never names a receiver, so a tool that is absent or disabled simply does
    /// not appear.
    pub fn accepting(&self, port: &PortType) -> Vec<&ToolManifest> {
        return self.enabled().filter(|m| m.accepts(port)).collect();
    }

    /// Enabled tools that can produce the given payload type.
    pub fn offering(&self, port: &PortType) -> Vec<&ToolManifest> {
        return self.enabled().filter(|m| m.offers(port)).collect();
    }

    /// Asks every running service whether Luna may close.
    ///
    /// Tools with no service, or that are disabled, have nothing in flight and are not
    /// asked.
    pub fn poll_shutdown(&mut self) -> Vec<crate::shutdown::ToolVote> {
        let mut votes = Vec::new();

        for (id, entry) in self.entries.iter_mut() {
            let Some(service) = entry.service.as_mut() else {
                continue;
            };

            let vote = service.on_shutdown_request();

            if !vote.is_allow() {
                votes.push(crate::shutdown::ToolVote {
                    tool_id: id.clone(),
                    vote,
                });
            }
        }

        return votes;
    }

    fn entry(&self, tool_id: &str) -> Result<&ToolEntry> {
        return self
            .entries
            .get(tool_id)
            .ok_or_else(|| CoreError::UnknownTool { id: tool_id.to_string() });
    }

    fn entry_mut(&mut self, tool_id: &str) -> Result<&mut ToolEntry> {
        return self
            .entries
            .get_mut(tool_id)
            .ok_or_else(|| CoreError::UnknownTool { id: tool_id.to_string() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn manifest(id: &str, name: &str, background: bool) -> ToolManifest {
        return ToolManifest::from_toml(&format!(
            "id = \"{id}\"\nname = \"{name}\"\nversion = \"1.0.0\"\nbackground = {background}\n"
        ))
        .unwrap();
    }

    /// Records start and stop calls so lifecycle can be asserted from the outside.
    #[derive(Default)]
    struct Counters {
        started: AtomicUsize,
        stopped: AtomicUsize,
    }

    struct SpyService {
        counters: Arc<Counters>,
        fail_on_start: bool,
    }

    impl ToolService for SpyService {
        fn start(&mut self, ctx: &ServiceContext<'_>) -> Result<()> {
            if self.fail_on_start {
                return Err(CoreError::UnknownTool { id: "boom".to_string() });
            }

            // A service must be able to reach its own storage and the database from
            // what it is handed, without going looking for either.
            assert!(!ctx.tool_id.is_empty());
            assert!(ctx.paths.tool_data_dir(ctx.tool_id).is_ok());
            assert!(ctx.database.ends_with("luna.db"), "got {:?}", ctx.database);

            self.counters.started.fetch_add(1, Ordering::SeqCst);
            return Ok(());
        }

        fn stop(&mut self) -> Result<()> {
            self.counters.stopped.fetch_add(1, Ordering::SeqCst);
            return Ok(());
        }
    }

    fn spy(counters: Arc<Counters>, fail_on_start: bool) -> Option<ServiceFactory> {
        return Some(Box::new(move || {
            Box::new(SpyService {
                counters: counters.clone(),
                fail_on_start,
            })
        }));
    }

    fn paths() -> (tempfile::TempDir, AppPaths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::rooted_at(dir.path()).unwrap();
        return (dir, paths);
    }

    #[test]
    fn registers_tools_and_starts_them_disabled() {
        let mut reg = Registry::new();
        reg.register(manifest("luna.a", "A", false), None).unwrap();

        assert_eq!(reg.len(), 1);
        assert!(reg.contains("luna.a"));
        assert_eq!(reg.is_enabled("luna.a").unwrap(), false);
        assert_eq!(reg.enabled().count(), 0);
    }

    #[test]
    fn rejects_duplicate_ids() {
        let mut reg = Registry::new();
        reg.register(manifest("luna.a", "A", false), None).unwrap();

        let err = reg.register(manifest("luna.a", "Another", false), None);
        assert!(matches!(err, Err(CoreError::DuplicateTool { .. })), "{err:?}");
    }

    #[test]
    fn rejects_background_tools_with_no_factory() {
        let mut reg = Registry::new();
        let err = reg.register(manifest("luna.a", "A", true), None);

        assert!(matches!(err, Err(CoreError::InvalidManifest { .. })), "{err:?}");
    }

    #[test]
    fn unknown_tools_are_reported_not_panicked_on() {
        let reg = Registry::new();
        assert!(matches!(
            reg.manifest("luna.nope"),
            Err(CoreError::UnknownTool { .. })
        ));
    }

    #[test]
    fn enabling_starts_the_service_and_disabling_stops_it() {
        let (_dir, paths) = paths();
        let counters = Arc::new(Counters::default());

        let mut reg = Registry::new();
        reg.register(manifest("luna.a", "A", true), spy(counters.clone(), false))
            .unwrap();

        reg.enable("luna.a", &paths).unwrap();
        assert!(reg.is_enabled("luna.a").unwrap());
        assert!(reg.has_running_service("luna.a").unwrap());
        assert_eq!(counters.started.load(Ordering::SeqCst), 1);

        reg.disable("luna.a").unwrap();
        assert!(!reg.is_enabled("luna.a").unwrap());
        assert!(!reg.has_running_service("luna.a").unwrap());
        assert_eq!(counters.stopped.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn enable_and_disable_are_idempotent() {
        let (_dir, paths) = paths();
        let counters = Arc::new(Counters::default());

        let mut reg = Registry::new();
        reg.register(manifest("luna.a", "A", true), spy(counters.clone(), false))
            .unwrap();

        reg.enable("luna.a", &paths).unwrap();
        reg.enable("luna.a", &paths).unwrap();
        assert_eq!(counters.started.load(Ordering::SeqCst), 1, "started twice");

        reg.disable("luna.a").unwrap();
        reg.disable("luna.a").unwrap();
        assert_eq!(counters.stopped.load(Ordering::SeqCst), 1, "stopped twice");
    }

    #[test]
    fn re_enabling_builds_a_fresh_service() {
        let (_dir, paths) = paths();
        let counters = Arc::new(Counters::default());

        let mut reg = Registry::new();
        reg.register(manifest("luna.a", "A", true), spy(counters.clone(), false))
            .unwrap();

        reg.enable("luna.a", &paths).unwrap();
        reg.disable("luna.a").unwrap();
        reg.enable("luna.a", &paths).unwrap();

        assert_eq!(counters.started.load(Ordering::SeqCst), 2);
        assert_eq!(counters.stopped.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_tool_without_a_service_still_enables() {
        let (_dir, paths) = paths();

        let mut reg = Registry::new();
        reg.register(manifest("luna.a", "A", false), None).unwrap();

        reg.enable("luna.a", &paths).unwrap();

        assert!(reg.is_enabled("luna.a").unwrap());
        assert!(!reg.has_running_service("luna.a").unwrap(), "nothing to run");
    }

    #[test]
    fn a_service_that_fails_to_start_leaves_the_tool_disabled() {
        let (_dir, paths) = paths();
        let counters = Arc::new(Counters::default());

        let mut reg = Registry::new();
        reg.register(manifest("luna.a", "A", true), spy(counters.clone(), true))
            .unwrap();

        assert!(reg.enable("luna.a", &paths).is_err());
        assert!(!reg.is_enabled("luna.a").unwrap());
        assert!(!reg.has_running_service("luna.a").unwrap());
    }

    #[test]
    fn one_broken_tool_does_not_stop_the_others() {
        let (_dir, paths) = paths();
        let good = Arc::new(Counters::default());
        let bad = Arc::new(Counters::default());

        let mut reg = Registry::new();
        reg.register(manifest("luna.good", "Good", true), spy(good.clone(), false))
            .unwrap();
        reg.register(manifest("luna.bad", "Bad", true), spy(bad.clone(), true))
            .unwrap();

        for id in ["luna.good", "luna.bad"] {
            let mut cfg = ToolConfig::default();
            cfg.enabled = true;
            reg.apply_config(id, cfg).unwrap();
        }

        let failures = reg.start_enabled(&paths);

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].0, "luna.bad");
        assert!(reg.is_enabled("luna.good").unwrap());
        assert!(!reg.is_enabled("luna.bad").unwrap(), "failed tool must be left off");
        assert_eq!(good.started.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn start_enabled_honours_saved_settings() {
        let (_dir, paths) = paths();

        let mut reg = Registry::new();
        reg.register(manifest("luna.on", "On", false), None).unwrap();
        reg.register(manifest("luna.off", "Off", false), None).unwrap();

        let mut on = ToolConfig::default();
        on.enabled = true;
        reg.apply_config("luna.on", on).unwrap();

        let mut off = ToolConfig::default();
        off.enabled = false;
        reg.apply_config("luna.off", off).unwrap();

        assert!(reg.start_enabled(&paths).is_empty());

        assert!(reg.is_enabled("luna.on").unwrap());
        assert!(!reg.is_enabled("luna.off").unwrap());
    }

    #[test]
    fn sidebar_lists_only_enabled_tools_grouped_by_category() {
        let (_dir, paths) = paths();
        let mut reg = Registry::new();

        let with_category = |id: &str, name: &str, category: &str| {
            ToolManifest::from_toml(&format!(
                "id = \"{id}\"\nname = \"{name}\"\nversion = \"1.0.0\"\ncategory = \"{category}\"\n"
            ))
            .unwrap()
        };

        reg.register(with_category("luna.zebra", "Zebra", "alpha"), None).unwrap();
        reg.register(with_category("luna.apple", "Apple", "beta"), None).unwrap();
        reg.register(with_category("luna.mango", "Mango", "alpha"), None).unwrap();
        reg.register(with_category("luna.hidden", "Hidden", "alpha"), None).unwrap();

        for id in ["luna.zebra", "luna.apple", "luna.mango"] {
            reg.enable(id, &paths).unwrap();
        }

        let names: Vec<String> = reg.sidebar_entries().into_iter().map(|e| e.name).collect();

        // alpha before beta, alphabetical within each, disabled tool absent.
        assert_eq!(names, vec!["Mango", "Zebra", "Apple"]);
    }

    #[test]
    fn port_lookups_ignore_disabled_tools() {
        let (_dir, paths) = paths();
        let mut reg = Registry::new();

        let ascii = ToolManifest::from_toml(
            r#"
            id      = "luna.img_to_ascii"
            name    = "Image to Ascii"
            version = "1.0.0"
            accepts = ["luna/image"]
            offers  = ["luna/text"]
            "#,
        )
        .unwrap();
        reg.register(ascii, None).unwrap();

        let image: PortType = "luna/image".parse().unwrap();

        // Disabled: the sender must not see it at all.
        assert!(reg.accepting(&image).is_empty());

        reg.enable("luna.img_to_ascii", &paths).unwrap();
        assert_eq!(reg.accepting(&image).len(), 1);
        assert_eq!(reg.accepting(&image)[0].id, "luna.img_to_ascii");

        reg.disable("luna.img_to_ascii").unwrap();
        assert!(reg.accepting(&image).is_empty(), "disabling must remove it again");
    }

    #[test]
    fn iteration_order_is_independent_of_registration_order() {
        let mut forwards = Registry::new();
        forwards.register(manifest("luna.a", "A", false), None).unwrap();
        forwards.register(manifest("luna.b", "B", false), None).unwrap();

        let mut backwards = Registry::new();
        backwards.register(manifest("luna.b", "B", false), None).unwrap();
        backwards.register(manifest("luna.a", "A", false), None).unwrap();

        let ids = |r: &Registry| r.all().map(|m| m.id.clone()).collect::<Vec<_>>();

        assert_eq!(ids(&forwards), ids(&backwards));
    }
}
