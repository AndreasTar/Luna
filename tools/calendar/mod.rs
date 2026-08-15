//! The calendar.
//!
//! Split three ways. [`dates`] does the arithmetic and knows nothing about storage or
//! Slint. [`store`] does the storage and knows nothing about Slint. This file owns the
//! state the page is looking at, turns the other two into models, and puts them in the
//! global the page binds to.
//!
//! Models go through a global rather than through properties on the page because a tool
//! page is instantiated by the generated page chain, so Rust has no handle on it. See
//! the note in `helpers/global_callbacks.slint`.

mod dates;
mod store;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::{Datelike, Duration, Local, NaiveDate, Timelike, Utc};
use luna::rules::{Recurrence, Schedule};
use luna_core::{ScheduledJob, Scheduler, ServiceContext, ServiceFactory, ToolManifest, ToolService};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};

use crate::tools::{BoundTool, ToolView, ViewContext};
use crate::{
    DayCell, EventCell, Global_Calendar_Callback, Global_Calendar_Data, HourCell, LunaAppUi,
    MonthCell, UpcomingEventsCell,
};

use store::{CalendarEvent, Store};

pub const VERSION: luna::Version = luna::Version::new(0, 2, 0);

const TOOL_ID: &str = "luna.calendar";

/// How many entries the upcoming list shows.
const UPCOMING_LIMIT: usize = 40;

/// What the page is looking at.
///
/// `anchor` is the month on screen, `selected` the day. They are separate because
/// stepping the month must not move the selection, and selecting a day in the greyed-out
/// edge of a grid must move the month.
struct State {
    anchor: NaiveDate,
    selected: NaiveDate,
    store: Store,
    database: PathBuf,
    /// The note as edited but not yet written.
    pending_note: Option<String>,
}

impl State {
    /// The day grid currently on screen, which selection indices point into.
    fn grid(&self) -> Vec<dates::GridDay> {
        return dates::month_grid(self.anchor);
    }
}

pub struct Tool {
    ui_handle: Weak<LunaAppUi>,
}

impl BoundTool for Tool {
    fn tool_id(&self) -> &'static str {
        return TOOL_ID;
    }
}

impl ToolView for Tool {
    fn manifest() -> ToolManifest {
        return ToolManifest::from_toml(include_str!("manifest.toml"))
            .expect("calendar manifest.toml is malformed");
    }

    fn service() -> Option<ServiceFactory> {
        // Reminders have to fire with the page closed, which is the whole point of them,
        // so the calendar has a background half.
        return Some(Box::new(|| Box::new(CalendarService::default())));
    }

    fn bind(ui_handle: Weak<LunaAppUi>, ctx: &ViewContext<'_>) -> Self {
        let calendar = Tool { ui_handle };

        let database = ctx.paths.database_file();
        let today = Local::now().date_naive();

        let store = match Store::open(&database) {
            Ok(store) => store,
            Err(e) => {
                // A calendar that cannot reach its storage is still worth showing: the
                // grid, the navigation and the clock all work without it. Refusing to
                // bind would take the page away entirely.
                eprintln!("calendar: storage is unavailable, entries will not load: {e}");
                return calendar;
            }
        };

        let state = Rc::new(RefCell::new(State {
            anchor: today,
            selected: today,
            store,
            database,
            pending_note: None,
        }));

        let Some(ui) = calendar.ui_handle.upgrade() else {
            return calendar;
        };

        wire(&ui, &state);
        refresh(&ui, &state.borrow());

        return calendar;
    }
}

/// Hooks every callback the page can raise.
fn wire(ui: &LunaAppUi, state: &Rc<RefCell<State>>) {
    let callbacks = ui.global::<Global_Calendar_Callback>();

    macro_rules! handler {
        ($setter:ident, |$s:ident $(, $arg:ident : $ty:ty)*| $body:block) => {{
            let state = state.clone();
            let weak = ui.as_weak();

            callbacks.$setter(move |$($arg: $ty),*| {
                {
                    let mut $s = state.borrow_mut();
                    $body
                }

                if let Some(ui) = weak.upgrade() {
                    refresh(&ui, &state.borrow());
                }
            });
        }};
    }

    handler!(on_month_step, |s, by: i32| {
        s.anchor = dates::add_months(s.anchor, by);
    });

    handler!(on_year_step, |s, by: i32| {
        s.anchor = dates::add_years(s.anchor, by);
    });

    handler!(on_today_selected, |s| {
        let today = Local::now().date_naive();
        s.anchor = today;
        s.selected = today;
    });

    handler!(on_day_selected, |s, index: i32| {
        if let Some(cell) = s.grid().get(index.max(0) as usize) {
            s.selected = cell.date;
            // Clicking a greyed-out day at the edge of the grid moves to that month,
            // which is what the click meant.
            s.anchor = cell.date;
        }
    });

    handler!(on_year_month_selected, |s, month: i32| {
        let month = (month.clamp(0, 11) as u32) + 1;
        let day = s.selected.day().min(dates::last_day_of_month(s.anchor.year(), month));

        if let Some(date) = NaiveDate::from_ymd_opt(s.anchor.year(), month, day) {
            s.anchor = date;
            s.selected = date;
        }
    });

    handler!(on_year_day_selected, |s, index: i32| {
        let index = index.max(0) as usize;
        let month = index / dates::MONTH_CELLS;
        let cell = index % dates::MONTH_CELLS;

        if let Some(first) = NaiveDate::from_ymd_opt(s.anchor.year(), month as u32 + 1, 1) {
            if let Some(day) = dates::month_grid(first).get(cell) {
                // Only the month's own days, so clicking the greyed-out padding of a
                // month in the year view does nothing rather than jumping a month.
                if day.in_month {
                    s.selected = day.date;
                    s.anchor = day.date;
                }
            }
        }
    });

    handler!(on_hour_selected, |_s, _hour: i32| {
        // Nothing to select yet: an hour is a place to put an entry, and entries are
        // added through the editor below the column.
    });

    handler!(on_upcoming_selected, |s, index: i32| {
        let events = s
            .store
            .upcoming(Utc::now(), UPCOMING_LIMIT)
            .unwrap_or_default();

        if let Some(event) = events.get(index.max(0) as usize) {
            let day = dates::local_date_of(event.starts_at);
            s.selected = day;
            s.anchor = day;
        }
    });

    handler!(on_event_added, |s, title: SharedString, start: i32, end: i32, remind_days: i32| {
        add_event(&mut s, &title, start, end, remind_days);
    });

    handler!(on_event_removed, |s, id: i32| {
        remove_event(&mut s, id as i64);
    });

    handler!(on_note_saved, |s| {
        save_note(&mut s);
    });

    // Typing does not refresh: the models have not changed, and pushing the note back
    // into the box the user is typing in would move the cursor.
    let note_state = state.clone();
    let note_weak = ui.as_weak();
    callbacks.on_note_edited(move |text| {
        note_state.borrow_mut().pending_note = Some(text.to_string());

        if let Some(ui) = note_weak.upgrade() {
            ui.global::<Global_Calendar_Data>().set_note_unsaved(true);
        }
    });
}

/// Adds an entry to the selected day.
///
/// Hours are clamped rather than rejected: the editor takes free text, and a person
/// typing 25 means the end of the day, not an error dialog.
fn add_event(state: &mut State, title: &str, start: i32, end: i32, remind_days: i32) {
    let title = title.trim();

    if title.is_empty() {
        return;
    }

    let start_hour = start.clamp(0, 23);
    // At least an hour long, and never running more than a day.
    let end_hour = end.clamp(start_hour + 1, start_hour + 24);

    let (day_start, _) = dates::day_bounds(state.selected);

    let event = CalendarEvent {
        id: 0,
        title: title.to_string(),
        starts_at: day_start + Duration::hours(start_hour as i64),
        ends_at: day_start + Duration::hours(end_hour as i64),
        all_day: false,
        reminder_lead: Duration::days(remind_days.clamp(0, 365) as i64),
        notes: String::new(),
    };

    match state.store.insert(&event) {
        Ok(id) => sync_reminder(state, id, &event),
        Err(e) => eprintln!("calendar: the entry could not be saved: {e}"),
    }
}

fn remove_event(state: &mut State, id: i64) {
    if let Err(e) = state.store.delete(id) {
        eprintln!("calendar: the entry could not be removed: {e}");
        return;
    }

    if let Ok(mut scheduler) = Scheduler::open(&state.database) {
        let _ = scheduler.remove(&reminder_rule_id(id));
    }
}

fn save_note(state: &mut State) {
    let Some(body) = state.pending_note.take() else {
        return;
    };

    if let Err(e) = state.store.set_note(state.selected, &body, Utc::now()) {
        eprintln!("calendar: the note could not be saved: {e}");
    }
}

/// The scheduler rule id for an entry's reminder.
fn reminder_rule_id(event_id: i64) -> String {
    return format!("luna.calendar.event.{event_id}");
}

/// Registers, or removes, the scheduled reminder for one entry.
///
/// Written through a scheduler connection of this tool's own. The scheduler thread
/// notices on its next tick, because writing a job bumps a revision the thread compares
/// against; without that the reminder would only start working after a restart.
/// The scheduled job that reminds about one entry.
///
/// One function, used both when an entry is added and when the background half sweeps
/// the database at startup. Built in one place because two descriptions of the same rule
/// is one more than can be kept in agreement.
///
/// The entry is described by a recurrence rather than by a stored instant, because that
/// is the shape the scheduler works in: an instant goes wrong the moment a clock, a
/// timezone or a daylight saving rule moves, and the recurrence is re-evaluated against
/// whatever the clock says now.
///
/// A one-off is a recurrence bounded to a single occurrence. It is written as a yearly
/// rule on its own date with `until` set just past it, so nothing produces a second one.
/// Leaving `until` unset would quietly turn every dentist appointment into an annual
/// tradition.
///
/// The lead time is what separates the reminder from the thing it is about: an entry on
/// the 4th of July with seven days of lead fires on the 27th of June, and the fire still
/// carries the date it is warning about.
fn reminder_job(id: i64, event: &CalendarEvent) -> Option<ScheduledJob> {
    let local = event.starts_at.with_timezone(&Local);
    let at = chrono::NaiveTime::from_hms_opt(local.hour(), local.minute(), 0)?;

    let mut schedule = Schedule::new(
        Recurrence::Yearly {
            interval: 1,
            month: local.month(),
            day: local.day(),
        },
        at,
        // A moment before the entry, because occurrences are searched over a half-open
        // span that excludes its start. Anchored on the entry itself, the entry's own
        // occurrence would be the one thing the schedule never produced.
        event.starts_at - Duration::seconds(1),
    );

    // Every entry is one-off for now. Repeating entries are the `recurrence` column,
    // which nothing writes yet: that is the editor's job, not this function's.
    schedule = schedule.until(event.starts_at + Duration::seconds(1));

    return Some(
        ScheduledJob::new(reminder_rule_id(id), schedule)
            .owned_by(TOOL_ID)
            .reminding_before(event.reminder_lead),
    );
}

fn sync_reminder(state: &State, id: i64, event: &CalendarEvent) {
    let Some(job) = reminder_job(id, event) else {
        return;
    };

    let mut scheduler = match Scheduler::open(&state.database) {
        Ok(scheduler) => scheduler,
        Err(e) => {
            eprintln!("calendar: the reminder could not be registered: {e}");
            return;
        }
    };

    if let Err(e) = scheduler.upsert(job) {
        eprintln!("calendar: the reminder could not be registered: {e}");
    }
}

/// Rebuilds every model the page reads.
fn refresh(ui: &LunaAppUi, state: &State) {
    let data = ui.global::<Global_Calendar_Data>();
    let today = Local::now().date_naive();

    let grid = state.grid();

    let marked = dates::grid_bounds(state.anchor);
    let month_marks = state
        .store
        .days_with_events(marked.0, marked.1)
        .unwrap_or_default();

    let month_cells: Vec<DayCell> = grid
        .iter()
        .map(|cell| DayCell {
            day_num: cell.date.day() as i32,
            in_month: cell.in_month,
            is_today: cell.date == today,
            is_selected: cell.date == state.selected,
            has_events: month_marks.contains(&cell.date),
        })
        .collect();

    data.set_month_grid(ModelRc::new(VecModel::from(month_cells)));
    data.set_month_label(dates::month_label(state.anchor).into());
    data.set_year_label(state.anchor.year().to_string().into());
    data.set_day_label(dates::long_date_label(state.selected).into());

    // The year view. One flat array of twelve grids, so month m's cell i is at
    // m * MONTH_CELLS + i.
    let year = state.anchor.year();
    let year_bounds = (
        dates::day_bounds(NaiveDate::from_ymd_opt(year, 1, 1).unwrap_or(state.anchor)).0,
        dates::day_bounds(NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap_or(state.anchor)).0,
    );
    let year_marks = state
        .store
        .days_with_events(year_bounds.0, year_bounds.1)
        .unwrap_or_default();

    let mut year_months = Vec::with_capacity(12);
    let mut year_days = Vec::with_capacity(12 * dates::MONTH_CELLS);

    for month in 1..=12u32 {
        let Some(first) = NaiveDate::from_ymd_opt(year, month, 1) else {
            continue;
        };

        year_months.push(MonthCell {
            month_num: month as i32,
            in_year: true,
            is_current: first.year() == today.year() && month == today.month(),
            is_selected: first.year() == state.selected.year() && month == state.selected.month(),
        });

        for cell in dates::month_grid(first) {
            year_days.push(DayCell {
                day_num: cell.date.day() as i32,
                in_month: cell.in_month,
                is_today: cell.date == today,
                is_selected: cell.in_month && cell.date == state.selected,
                has_events: cell.in_month && year_marks.contains(&cell.date),
            });
        }
    }

    data.set_year_months(ModelRc::new(VecModel::from(year_months)));
    data.set_year_days(ModelRc::new(VecModel::from(year_days)));

    // The day column.
    let now = Local::now();
    let showing_today = state.selected == today;

    let hours: Vec<HourCell> = (0..24)
        .map(|hour| HourCell {
            hour_num: hour,
            is_selected: false,
            is_current: showing_today && hour as u32 == now.hour(),
        })
        .collect();

    data.set_day_hours(ModelRc::new(VecModel::from(hours)));

    let (day_start, day_end) = dates::day_bounds(state.selected);
    let day_events: Vec<EventCell> = state
        .store
        .events_between(day_start, day_end)
        .unwrap_or_default()
        .iter()
        .map(|event| {
            // Measured from the start of the day being shown, so an entry that began
            // yesterday starts at hour 0 and one running past midnight ends past 24.
            let start = (event.starts_at - day_start).num_minutes() as f32 / 60.0;
            let end = (event.ends_at - day_start).num_minutes() as f32 / 60.0;

            return EventCell {
                id: event.id as i32,
                title: event.title.clone().into(),
                start_hour: start.floor().max(0.0) as i32,
                end_hour: end.ceil().min(48.0) as i32,
                all_day: event.all_day,
            };
        })
        .collect();

    data.set_day_events(ModelRc::new(VecModel::from(day_events)));

    // What is coming.
    let upcoming: Vec<UpcomingEventsCell> = state
        .store
        .upcoming(Utc::now(), UPCOMING_LIMIT)
        .unwrap_or_default()
        .iter()
        .map(|event| {
            let day = dates::local_date_of(event.starts_at);

            return UpcomingEventsCell {
                event_id: event.id as i32,
                event_date: dates::short_date_label(day).into(),
                event_title: event.title.clone().into(),
                is_today: day == today,
            };
        })
        .collect();

    data.set_upcoming(ModelRc::new(VecModel::from(upcoming)));

    // The note for the selected day. An unsaved edit survives a refresh, so stepping the
    // month with a half-written note does not throw it away.
    let note = match &state.pending_note {
        Some(pending) => pending.clone(),
        None => state.store.note(state.selected).unwrap_or_default(),
    };

    data.set_note_unsaved(state.pending_note.is_some());
    data.set_note_body(note.into());
}

/// The calendar's background half.
///
/// Registers a reminder for every entry that wants one, so they fire whether or not the
/// page has been opened this session. Reminders added while the page is open are
/// registered as they are made; this covers everything already in the database.
#[derive(Default)]
struct CalendarService {
    database: Option<PathBuf>,
}

impl ToolService for CalendarService {
    fn start(&mut self, ctx: &ServiceContext<'_>) -> luna_core::Result<()> {
        let database = ctx.database.to_path_buf();

        let store = Store::open(&database)?;
        let mut scheduler = Scheduler::open(&database)?;

        let mut registered = 0;

        for event in store.events_with_reminders()? {
            let Some(job) = reminder_job(event.id, &event) else {
                continue;
            };

            scheduler.upsert(job)?;
            registered += 1;
        }

        if registered > 0 {
            eprintln!("calendar: {registered} reminders registered");
        }

        self.database = Some(database);

        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn birthday(month: u32, day: u32, lead_days: i64) -> CalendarEvent {
        let starts = Local
            .with_ymd_and_hms(2026, month, day, 9, 0, 0)
            .single()
            .expect("a valid local instant")
            .with_timezone(&Utc);

        return CalendarEvent {
            id: 1,
            title: "Alice's birthday".to_string(),
            starts_at: starts,
            ends_at: starts + Duration::hours(1),
            all_day: false,
            reminder_lead: Duration::days(lead_days),
            notes: String::new(),
        };
    }

    #[test]
    fn a_reminder_fires_its_lead_time_before_the_entry() {
        // The case this feature exists for: a birthday on the 4th of July, and a
        // reminder to buy a gift a week earlier.
        let event = birthday(7, 4, 7);
        let job = reminder_job(event.id, &event).expect("the job should be buildable");

        assert_eq!(job.tool_id.as_deref(), Some(TOOL_ID));
        assert_eq!(job.lead_time, Duration::days(7));

        // What the scheduler will do with it: the occurrence is the 4th, the alert is
        // the 27th of June, and the alert still knows which day it is about.
        let from = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).unwrap();

        let occurrences = job.schedule.occurrences_between(from, to, 10);
        assert_eq!(occurrences.len(), 1, "one occurrence in the window");

        let occurrence = dates::local_date_of(occurrences[0]);
        let alert = dates::local_date_of(occurrences[0] - job.lead_time);

        assert_eq!(occurrence, NaiveDate::from_ymd_opt(2026, 7, 4).unwrap());
        assert_eq!(alert, NaiveDate::from_ymd_opt(2026, 6, 27).unwrap());
    }

    #[test]
    fn an_entry_without_a_lead_time_reminds_at_its_own_time() {
        let event = birthday(7, 4, 0);
        let job = reminder_job(event.id, &event).expect("the job should be buildable");

        assert_eq!(job.lead_time, Duration::zero());

        let from = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).unwrap();

        let occurrences = job.schedule.occurrences_between(from, to, 10);
        assert_eq!(dates::local_date_of(occurrences[0]), NaiveDate::from_ymd_opt(2026, 7, 4).unwrap());
        assert_eq!(dates::local_hour_of(occurrences[0]), 9);
    }

    #[test]
    fn a_one_off_entry_does_not_come_round_again_next_year() {
        // Written as a yearly rule because that is the shape the scheduler takes, so the
        // bound is the only thing stopping a single appointment reminding forever.
        let event = birthday(7, 4, 7);
        let job = reminder_job(event.id, &event).expect("the job should be buildable");

        let from = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2031, 1, 1, 0, 0, 0).unwrap();

        let occurrences = job.schedule.occurrences_between(from, to, 50);

        assert_eq!(
            occurrences.len(),
            1,
            "five years should still hold exactly one occurrence, got {occurrences:?}"
        );
    }

    #[test]
    fn a_rule_id_is_stable_for_an_entry() {
        // The id is how a removed entry finds its job again, so it has to be derived
        // rather than remembered.
        assert_eq!(reminder_rule_id(42), "luna.calendar.event.42");
        assert_ne!(reminder_rule_id(42), reminder_rule_id(43));
    }
}
