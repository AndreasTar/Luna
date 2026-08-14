
use crate::tools::{ BoundTool, ToolView };
use crate::{ Global_Calendar_Callback, LunaAppUi };
use luna_core::ToolManifest;
use slint::{ ComponentHandle, Model, ModelRc, SharedString, Weak };

pub const VERSION: luna::Version = luna::Version::new(0, 0, 1);

pub struct Tool {
    ui_handle: Weak<LunaAppUi>,
}

impl BoundTool for Tool {
    fn tool_id(&self) -> &'static str {
        return "luna.calendar";
    }
}

impl ToolView for Tool {

    fn manifest() -> ToolManifest {
        return ToolManifest::from_toml(include_str!("manifest.toml"))
            .expect("calendar manifest.toml is malformed");
    }

    fn bind(ui_handle: Weak<LunaAppUi>) -> Self {

        let calendar = Tool {
            ui_handle
        };

        // The calendar's globals were previously unreachable from Rust: a global is
        // only exposed to Rust if the root .slint file re-exports it, and this one was
        // not. Now that it is, every click has somewhere to land.
        //
        // These are stubs. The calendar has no backing data yet, so they exist to
        // prove the wiring and to give the real handlers somewhere to go.
        let ui = calendar.ui_handle.unwrap();
        let callbacks = ui.global::<Global_Calendar_Callback>();

        callbacks.on_month_date_clicked(|| {});
        callbacks.on_month_next_button_clicked(|| {});
        callbacks.on_month_prev_button_clicked(|| {});
        callbacks.on_year_month_clicked(|| {});
        callbacks.on_year_date_clicked(|| {});
        callbacks.on_year_next_button_clicked(|| {});
        callbacks.on_year_prev_button_clicked(|| {});
        callbacks.on_daily_hour_clicked(|| {});
        callbacks.on_notes_save_button_clicked(|| {});
        callbacks.on_upcoming_event_clicked(|| {});

        return calendar;
    }
}
