#![allow(unused, dead_code, non_snake_case, non_camel_case_types)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use luna_core::{Host, SidebarEntry as CoreSidebarEntry};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

mod tools;
pub mod helpers;

use tools::ToolView;

slint::include_modules!();

// TODO add tests for each ui page (tools should have doctests ready)
// TODO add example and info etc page for each tool (maybe widget it or something)
// TODO also all calls to luna_lib need to be asyncronous, so we can use them in the UI without blocking the main thread

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    // Storage, settings and the registry come up before any window exists. Background
    // services do not need a UI, and eventually the window will be destroyed while
    // the app keeps running.
    let mut host = Host::bootstrap()?;

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

    populate_sidebar(&luna_app_ui, &host);

    luna_app_ui.run()?;

    shut_down(&luna_app_ui, &mut host);

    return Ok(());
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

    for (id, error) in host.stop_tools() {
        eprintln!("shutdown: tool {id} failed to stop cleanly: {error}");
    }

    if let Err(e) = host.save_config() {
        eprintln!("shutdown: could not save settings: {e}");
    }
}
