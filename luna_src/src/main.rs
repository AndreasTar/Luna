#![allow(unused, dead_code, non_snake_case, non_camel_case_types)]
// No console window in release. Kept in debug, where the startup lines and the
// scheduler's output are worth seeing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::Cell;
use std::rc::Rc;

use luna::palette::{Resolved, Role};
use luna_core::instance::{self, InstanceCheck};
use luna_core::lifecycle::{self, RebuildCapability, ShutdownIntent};
use luna_core::shutdown::ShutdownReport;
use luna_core::Scheduler;
use luna_core::{Host, SidebarEntry as CoreSidebarEntry, ToolConfig};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

mod tools;
pub mod helpers;

use tools::ToolView;

slint::include_modules!();

// TODO add tests for each ui page (tools should have doctests ready)
// TODO add example and info etc page for each tool (maybe widget it or something)
// TODO also all calls to luna_lib need to be asyncronous, so we can use them in the UI without blocking the main thread

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let intent = run()?;

    // The launcher reads this to decide whether to relaunch, rebuild, or stop. It has
    // to be a process exit code, because it must survive the app dying.
    std::process::exit(intent.exit_code());
}

fn run() -> Result<ShutdownIntent, Box<dyn std::error::Error>> {
    // Before anything touches the database or the config: two Lunas sharing one
    // install would quietly disagree about everything.
    let paths = luna_core::AppPaths::discover()?;

    let _instance = match instance::acquire(paths.install_dir())? {
        InstanceCheck::Acquired(lock) => lock,
        InstanceCheck::AlreadyRunning { pid } => {
            // Not an error. The user clicked the shortcut because they wanted Luna,
            // and one is already there.
            match pid {
                Some(pid) => eprintln!("Luna is already running (process {pid})."),
                None => eprintln!("Luna is already running."),
            }
            return Ok(ShutdownIntent::Exit);
        }
    };

    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    // Storage, settings and the registry come up before any window exists. Background
    // services do not need a UI, and eventually the window will be destroyed while
    // the app keeps running.
    let mut host = Host::bootstrap_at(paths)?;

    for manifest in tools::manifests() {
        // No service factories yet: neither tool declares `background`. The argument
        // is where a tool's background half gets registered once one needs it.
        host.register_tool(manifest, None)?;
    }

    host.start_tools();

    for notice in &host.notices {
        eprintln!("startup: {notice}");
    }

    // Luna spends most of its life with no window, so what it loaded is worth stating
    // rather than leaving to be inferred from the sidebar.
    let enabled: Vec<&str> = host.registry.enabled().map(|m| m.id.as_str()).collect();
    eprintln!(
        "startup: {} tools compiled in, {} enabled: {}",
        host.registry.len(),
        enabled.len(),
        enabled.join(", ")
    );

    let luna_app_ui = LunaAppUi::new()?;

    // Bound once, because callbacks live on Slint globals which outlive any page.
    // See the note on `tools::ToolView` for what changes when pages own their own
    // callbacks. Held for the life of the window.
    let _bound_tools = tools::bind_all(&luna_app_ui.as_weak());

    let intent = Rc::new(Cell::new(ShutdownIntent::Exit));
    let host = Rc::new(std::cell::RefCell::new(host));

    populate_sidebar(&luna_app_ui, &host.borrow());

    wire_tool_change_banner(&luna_app_ui, &host.borrow(), &intent);
    wire_theme(&luna_app_ui, &host.borrow());
    wire_palette_picker(&luna_app_ui, &host);

    // The scheduler runs whether or not a window exists, which is the point of it.
    // Nothing registers jobs yet, so it idles; surfacing what it fires is UI work for
    // whichever tool owns the job.
    let _scheduler = match luna_core::scheduler::spawn(
        host.borrow().paths.database_file(),
        std::time::Duration::from_secs(30),
        |fires| {
            for fire in fires {
                eprintln!(
                    "scheduler: {} fired (due {}{})",
                    fire.rule_id,
                    fire.due_at,
                    if fire.late { ", late" } else { "" }
                );
            }
        },
    ) {
        Ok(handle) => Some(handle),
        Err(e) => {
            eprintln!("startup: the scheduler could not start: {e}");
            None
        }
    };

    let tray = wire_shutdown(&luna_app_ui, &host, &intent);

    let _tray = tray;

    luna_app_ui.run()?;

    shut_down(&luna_app_ui, &mut host.borrow_mut());

    return Ok(intent.get());
}

/// Wires the tray, the close button and the quit prompt together.
///
/// These are one feature, not three. Hiding the window only makes sense if there is a
/// tray to bring it back from, and quitting only makes sense after the user has been
/// told what stops. Returns the tray, which must be kept alive to stay in the tray.
fn wire_shutdown(
    ui: &LunaAppUi,
    host: &Rc<std::cell::RefCell<Host>>,
    intent: &Rc<Cell<ShutdownIntent>>,
) -> Option<helpers::tray::Tray> {
    let tray = helpers::tray::build(ui, {
        let ui = ui.as_weak();
        let host = host.clone();

        move |action| {
            let Some(ui) = ui.upgrade() else {
                return;
            };

            match action {
                helpers::tray::TrayAction::Show => {
                    let _ = ui.show();
                    let _ = ui.window().set_minimized(false);
                }
                helpers::tray::TrayAction::Quit => request_quit(&ui, &host),
            }
        }
    });

    ui.set_has_tray(tray.is_some());

    // Closing the window hides it when there is a tray to get it back from, and asks
    // about quitting when there is not. Silently exiting on a close click would be the
    // one thing an always-running app must not do.
    let has_tray = tray.is_some();

    ui.window().on_close_requested({
        let ui = ui.as_weak();
        let host = host.clone();

        move || {
            let Some(ui) = ui.upgrade() else {
                return slint::CloseRequestResponse::HideWindow;
            };

            if has_tray {
                // Hidden, not quit. The scheduler and every service keep running.
                return slint::CloseRequestResponse::HideWindow;
            }

            request_quit(&ui, &host);

            return slint::CloseRequestResponse::KeepWindowShown;
        }
    });

    ui.on_cancel_quit({
        let ui = ui.as_weak();
        move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_showing_quit_prompt(false);
            }
        }
    });

    ui.on_minimise_to_tray({
        let ui = ui.as_weak();
        move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_showing_quit_prompt(false);
                let _ = ui.hide();
            }
        }
    });

    ui.on_confirm_quit({
        let intent = intent.clone();
        move || {
            intent.set(ShutdownIntent::Exit);
            let _ = slint::quit_event_loop();
        }
    });

    return tray;
}

/// Decides whether quitting needs to be asked about, and asks if so.
///
/// An idle Luna closes without a prompt. One with reminders pending, or a tool mid
/// conversion, says so first: that is the piece of information that actually changes
/// the answer.
fn request_quit(ui: &LunaAppUi, host: &Rc<std::cell::RefCell<Host>>) {
    let report = build_shutdown_report(host);
    let now = chrono::Utc::now();

    if report.can_close_silently() {
        let _ = slint::quit_event_loop();
        return;
    }

    let lines: Vec<SharedString> = report
        .lines(now)
        .into_iter()
        .map(SharedString::from)
        .collect();

    ui.set_quit_headline(SharedString::from(report.headline(now)));
    ui.set_quit_lines(ModelRc::new(VecModel::from(lines)));
    ui.set_showing_quit_prompt(true);

    // The prompt is useless behind a hidden window, which is where Quit from the tray
    // leaves it.
    let _ = ui.show();
}

/// Asks the tools and the scheduler what closing would interrupt.
fn build_shutdown_report(host: &Rc<std::cell::RefCell<Host>>) -> ShutdownReport {
    // A short-lived connection rather than sharing the scheduler thread's: the thread
    // owns its own and is not `Sync`. Reading the job table is cheap.
    let upcoming = {
        let database = host.borrow().paths.database_file();

        match Scheduler::open(&database) {
            Ok(scheduler) => scheduler.upcoming(chrono::Utc::now()),
            Err(e) => {
                eprintln!("could not read scheduled work for the quit prompt: {e}");
                Vec::new()
            }
        }
    };

    return host.borrow_mut().shutdown_report(upcoming);
}

/// Fills the palette picker and applies a chosen palette immediately.
///
/// Choosing a palette rewrites the `Theme` global and saves the setting straight away.
/// A preview that only takes effect on restart would make the swatches the only
/// feedback, which is not enough to judge a palette by.
fn wire_palette_picker(ui: &LunaAppUi, host: &Rc<std::cell::RefCell<Host>>) {
    {
        let host_ref = host.borrow();
        ui.set_palettes(build_palette_model(&host_ref));
        ui.set_active_palette_id(SharedString::from(host_ref.app_palette().id.clone()));
        ui.set_palettes_folder(SharedString::from(
            host_ref.paths.palettes_dir().display().to_string(),
        ));
    }

    ui.on_palette_chosen({
        let ui = ui.as_weak();
        let host = host.clone();

        move |id| {
            let Some(ui) = ui.upgrade() else {
                return;
            };

            let mut host = host.borrow_mut();
            host.config.palette = id.to_string();

            if let Err(e) = host.save_config() {
                eprintln!("could not save the palette choice: {e}");
            }

            // Reapply to the tool currently on screen, so the change is visible the
            // moment it is made.
            let active = ui.get_current_tool_id();
            let resolved = host.palette_for_tool(active.as_str());

            push_theme(&ui, &resolved);
            ui.set_active_palette_id(id);
        }
    });
}

/// Builds the picker's model, including palettes that could not be loaded.
fn build_palette_model(host: &Host) -> ModelRc<PaletteEntry> {
    let brush = |color: luna::palette::Color| -> slint::Brush {
        return slint::Brush::SolidColor(slint::Color::from_argb_u8(
            color.a, color.r, color.g, color.b,
        ));
    };

    let mut entries: Vec<PaletteEntry> = host
        .palettes
        .all()
        .iter()
        .map(|p| PaletteEntry {
            id: SharedString::from(p.id.clone()),
            name: SharedString::from(p.name.clone()),
            description: SharedString::from(p.description.clone()),
            appearance: SharedString::from(p.appearance.as_str()),
            usable: true,
            problem: SharedString::new(),
            swatch_background: brush(p.get(Role::Background)),
            swatch_text: brush(p.get(Role::Text)),
            swatch_primary: brush(p.get(Role::Primary)),
            swatch_success: brush(p.get(Role::Success)),
            swatch_warning: brush(p.get(Role::Warning)),
            swatch_error: brush(p.get(Role::Error)),
        })
        .collect();

    // Listed rather than skipped: a file the user wrote and cannot find looks like
    // the app ignoring them.
    for problem in &host.palettes.problems {
        let name = problem
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| problem.path.display().to_string());

        entries.push(PaletteEntry {
            id: SharedString::new(),
            name: SharedString::from(name),
            description: SharedString::new(),
            appearance: SharedString::new(),
            usable: false,
            problem: SharedString::from(problem.reason.clone()),
            ..Default::default()
        });
    }

    return ModelRc::new(VecModel::from(entries));
}

/// Pushes the resolved palette into the `Theme` global, and keeps it current.
///
/// Resolution happens here rather than in Slint because a tool may use a different
/// palette or override individual roles, and the three-layer chain that works out is
/// Rust's job. Components only ever see a finished palette.
fn wire_theme(ui: &LunaAppUi, host: &Host) {
    // Palette problems are already reported through Host::notices; no need to repeat
    // them here.
    let app_palette = host.app_palette();
    eprintln!(
        "startup: palette {:?} ({} loaded)",
        app_palette.id,
        host.palettes.all().len()
    );

    // The active tool decides the palette, so it is reapplied on every page change.
    let apply = {
        let ui = ui.as_weak();
        let palettes = host.palettes.clone();
        let app_palette = app_palette.clone();

        move |tool_config: ToolConfig| {
            if let Some(ui) = ui.upgrade() {
                let resolved = palettes.resolve_for_tool(&app_palette, &tool_config);
                push_theme(&ui, &resolved);
            }
        }
    };

    apply(ToolConfig::default());

    // Reapply whenever the visible tool changes. Slint has no change callback on a
    // property, so this hangs off the sidebar selection the same way the page does.
    let configs: std::collections::BTreeMap<String, ToolConfig> = host
        .registry
        .all()
        .filter_map(|m| host.registry.config(&m.id).ok().map(|c| (m.id.clone(), c.clone())))
        .collect();

    ui.on_tool_changed({
        let apply = apply.clone();
        move |tool_id| {
            let config = configs.get(tool_id.as_str()).cloned().unwrap_or_default();
            apply(config);
        }
    });
}

/// Copies a resolved palette into the Slint `Theme` global.
fn push_theme(ui: &LunaAppUi, resolved: &Resolved) {
    let theme = ui.global::<Theme>();

    let c = |role: Role| -> slint::Brush {
        let color = resolved.get(role);
        return slint::Brush::SolidColor(slint::Color::from_argb_u8(
            color.a, color.r, color.g, color.b,
        ));
    };

    theme.set_primary(c(Role::Primary));
    theme.set_secondary(c(Role::Secondary));
    theme.set_tertiary(c(Role::Tertiary));
    theme.set_quaternary(c(Role::Quaternary));

    theme.set_text(c(Role::Text));
    theme.set_text_secondary(c(Role::TextSecondary));
    theme.set_text_tertiary(c(Role::TextTertiary));
    theme.set_text_quaternary(c(Role::TextQuaternary));

    theme.set_background(c(Role::Background));
    theme.set_background_secondary(c(Role::BackgroundSecondary));
    theme.set_background_tertiary(c(Role::BackgroundTertiary));
    theme.set_background_quaternary(c(Role::BackgroundQuaternary));

    theme.set_border(c(Role::Border));
    theme.set_border_secondary(c(Role::BorderSecondary));
    theme.set_border_tertiary(c(Role::BorderTertiary));
    theme.set_border_quaternary(c(Role::BorderQuaternary));

    theme.set_success(c(Role::Success));
    theme.set_warning(c(Role::Warning));
    theme.set_error(c(Role::Error));
    theme.set_info(c(Role::Info));
    theme.set_danger(c(Role::Danger));

    theme.set_inactive(c(Role::Inactive));
    theme.set_disabled(c(Role::Disabled));

    theme.set_highlight(c(Role::Highlight));
}

/// Compares the tools on disk against the ones in this binary and offers a rebuild.
///
/// The check is a scan at startup rather than a filesystem watcher: a new tool folder
/// is not urgent, and a watcher would cost memory in an app that is always running.
fn wire_tool_change_banner(ui: &LunaAppUi, host: &Host, intent: &Rc<Cell<ShutdownIntent>>) {
    let known: Vec<String> = host.registry.all().map(|m| m.id.clone()).collect();
    let scan = lifecycle::scan_tools_dir(host.paths.tools_dir(), &known);

    for broken in &scan.unreadable {
        eprintln!(
            "startup: {} has a manifest that could not be read, so it was ignored",
            broken.display()
        );
    }

    let Some(summary) = scan.summary() else {
        return;
    };

    let capability = lifecycle::rebuild_capability(host.paths.install_dir());
    eprintln!("startup: tool changes on disk: {summary} (rebuild: {capability:?})");

    let message = if capability.is_available() {
        format!("Tool changes found: {summary}. Luna needs to rebuild to apply them.")
    } else {
        // Say why rather than offering a button that cannot work.
        format!("Tool changes found: {summary}. {}", capability.explanation())
    };

    ui.set_tool_change_notice(SharedString::from(message));
    ui.set_can_rebuild(capability.is_available());

    ui.on_dismiss_notice({
        let ui = ui.as_weak();
        move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_tool_change_notice(SharedString::new());
            }
        }
    });

    ui.on_rebuild_requested({
        let intent = intent.clone();
        move || {
            // Recorded before quitting, so the exit code reflects what was asked for
            // even though the shutdown path is shared with an ordinary quit.
            intent.set(ShutdownIntent::Rebuild);
            let _ = slint::quit_event_loop();
        }
    });
}

/// Fills the sidebar from the registry and restores the tool that was open last time.
fn populate_sidebar(ui: &LunaAppUi, host: &Host) {
    let entries = host.registry.sidebar_entries();

    let restored = host
        .config
        .last_active_tool
        .as_deref()
        // Only if it is still there and still enabled. A tool can be disabled, or
        // stop being compiled in, between one run and the next.
        .and_then(|id| entries.iter().position(|e| e.id == id))
        .unwrap_or(0);

    let model: Vec<SidebarEntry> = entries.into_iter().map(to_slint_entry).collect();

    ui.set_tools(ModelRc::new(VecModel::from(model)));
    ui.set_current_tool_index(restored as i32);
}

fn to_slint_entry(entry: CoreSidebarEntry) -> SidebarEntry {
    return SidebarEntry {
        id: SharedString::from(entry.id),
        name: SharedString::from(entry.name),
        category: SharedString::from(entry.category),
    };
}

/// Records what was open, stops services, and flushes settings.
///
/// Failures here are reported rather than propagated: the window is already gone, and
/// returning an error from a shutdown path only turns a lost setting into a crash.
fn shut_down(ui: &LunaAppUi, host: &mut Host) {
    let active = ui.get_current_tool_id();
    if !active.is_empty() {
        host.config.last_active_tool = Some(active.to_string());
    }

    // Only on a clean exit. A tool set to keep state for the session keeps it through
    // a crash, which is the right way round: the alternative loses work every time the
    // app is killed.
    match host.discard_session_ui_state() {
        Ok(0) => {}
        Ok(n) => eprintln!("shutdown: cleared session interface state for {n} tool(s)"),
        Err(e) => eprintln!("shutdown: could not clear session interface state: {e}"),
    }

    for (id, error) in host.stop_tools() {
        eprintln!("shutdown: tool {id} failed to stop cleanly: {error}");
    }

    if let Err(e) = host.save_config() {
        eprintln!("shutdown: could not save settings: {e}");
    }
}
