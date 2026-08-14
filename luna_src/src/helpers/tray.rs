//! The tray icon, and the loop that notices clicks on it.
//!
//! Luna is meant to stay running with no window most of the time. Without somewhere to
//! click, hiding the window would strand the user, so the tray and hide-on-close are
//! one feature rather than two: neither ships without the other.
//!
//! ## Why the events are polled
//!
//! `tray-icon` delivers menu clicks through a global channel rather than through the
//! windowing event loop. Slint owns that loop and does not expose a hook to drain
//! someone else's queue, so a [`slint::Timer`] drains it instead. Polling four times a
//! second costs nothing measurable and avoids reaching into Slint's backend, which
//! would tie Luna to a specific version of it.

use std::rc::Rc;

use slint::ComponentHandle;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::LunaAppUi;

/// How often the tray's event queue is drained.
///
/// Fast enough that a click feels immediate, slow enough to be invisible in a profile.
const POLL: std::time::Duration = std::time::Duration::from_millis(250);

/// What the user picked from the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// Bring the window back.
    Show,
    /// Close Luna for good, subject to the quit prompt.
    Quit,
}

/// A live tray icon.
///
/// Must be kept alive: dropping it removes the icon from the tray.
pub struct Tray {
    _icon: TrayIcon,
    _timer: slint::Timer,
}

/// Builds the tray icon and starts draining its events.
///
/// `on_action` runs on the UI thread, so it may touch the window directly.
///
/// Returns `None` if the tray could not be created, which happens on systems with no
/// tray at all. Luna carries on with an ordinary window in that case rather than
/// refusing to start, so `on_close` should check whether a tray exists before deciding
/// to hide rather than quit.
pub fn build(
    ui: &LunaAppUi,
    on_action: impl Fn(TrayAction) + 'static,
) -> Option<Tray> {
    let show = MenuItem::new("Show Luna", true, None);
    let quit = MenuItem::new("Quit Luna", true, None);

    let show_id = show.id().clone();
    let quit_id = quit.id().clone();

    let menu = Menu::new();
    menu.append(&show).ok()?;
    menu.append(&tray_icon::menu::PredefinedMenuItem::separator()).ok()?;
    menu.append(&quit).ok()?;

    let icon = TrayIconBuilder::new()
        .with_tooltip("Luna")
        .with_menu(Box::new(menu))
        .with_icon(placeholder_icon())
        .build()
        .ok()?;

    let on_action = Rc::new(on_action);

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, POLL, {
        let on_action = on_action.clone();
        let ui = ui.as_weak();

        move || {
            // Drain everything queued since the last tick. `try_recv` never blocks, so
            // a burst of clicks is handled in one pass rather than one per tick.
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                let action = if event.id == show_id {
                    TrayAction::Show
                } else if event.id == quit_id {
                    TrayAction::Quit
                } else {
                    continue;
                };

                if ui.upgrade().is_some() {
                    on_action(action);
                }
            }
        }
    });

    return Some(Tray {
        _icon: icon,
        _timer: timer,
    });
}

/// A plain generated icon.
///
/// Luna has no icon asset yet, and a tray entry with no image is invisible on Windows,
/// which would make the tray useless rather than merely unstyled. Replaced by the real
/// icon when there is one.
fn placeholder_icon() -> Icon {
    const SIZE: u32 = 16;

    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);

    let centre = (SIZE as f32 - 1.0) / 2.0;
    let radius = centre - 0.5;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - centre;
            let dy = y as f32 - centre;
            let inside = (dx * dx + dy * dy).sqrt() <= radius;

            if inside {
                rgba.extend_from_slice(&[0xdb, 0xc5, 0xfc, 0xff]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    // The dimensions match the buffer by construction, so this cannot fail.
    return Icon::from_rgba(rgba, SIZE, SIZE).expect("generated icon is well formed");
}
