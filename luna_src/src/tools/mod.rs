//! The tools compiled into this build.
//!
//! Each tool owns a folder holding its `manifest.toml`, its Rust binding and its
//! `.slint` UI. The list below is written by hand for now; the build script will
//! generate it once tools are discovered from their folders, at which point adding a
//! tool means dropping a folder in rather than editing this file.

use luna_core::ToolManifest;
use slint::Weak;

use crate::LunaAppUi;

pub(crate) mod base_converter;
pub(crate) mod calendar;

/// The UI half of a tool.
///
/// Implementors wire their Slint callbacks and hold whatever the page needs while it
/// is open. The background half is [`luna_core::ToolService`], which lives in the
/// registry and never touches Slint.
///
/// ## Lifecycle, and what it is not yet
///
/// The Slint page component is already created and destroyed on navigation, because
/// the page chain uses `if` and Slint instantiates conditionally. The Rust binding is
/// not: callbacks currently live on Slint globals, which exist for the life of the
/// window, so binding happens once at startup.
///
/// Moving the binding onto each page's own component instance, and with it true
/// per-open lifecycle, comes with the build-script work. The trait is shaped for that
/// now so tools do not need rewriting twice.
pub(crate) trait ToolView {
    /// The tool's manifest, parsed from the `manifest.toml` beside its source.
    ///
    /// Parsed rather than hand-built so the file stays the single source of truth,
    /// and so a malformed manifest fails loudly at startup instead of drifting out of
    /// sync with what the build script will later read.
    fn manifest() -> ToolManifest
    where
        Self: Sized;

    /// Wires the tool's callbacks to the window.
    fn bind(ui: Weak<LunaAppUi>) -> Self
    where
        Self: Sized;
}

/// Every tool's manifest, in registration order.
///
/// Kept separate from binding so the host can build its registry before a window
/// exists, which is what lets background services run with the UI closed.
pub(crate) fn manifests() -> Vec<ToolManifest> {
    return vec![
        base_converter::UI_BaseConverter::manifest(),
        calendar::UI_Calendar::manifest(),
    ];
}
