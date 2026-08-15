//! Reading and writing the calendar's own tables.
//!
//! Its own connection to the shared database, opened per the reasoning on
//! [`luna_core::ServiceContext::database`]: a connection is `Send` but not `Sync`, so
//! everything that wants one opens its own and WAL mode allows the overlap.
//!
//! Instants are stored as unix seconds UTC, including for all-day entries, which hold
//! the local day's midnight to the next midnight. A wall-clock string would make "what
//! is on this day" a string comparison that stops being true the moment the machine
//! changes timezone.

use std::collections::BTreeSet;
use std::path::Path;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, Timelike, Utc};
use luna::rules::{MonthDay, Recurrence, Schedule};
use luna_core::rusqlite::{self, Connection};
use luna_core::{Database, Result};

use super::dates;

/// How often an entry comes round.
///
/// Stored as its own short word in the `recurrence` column rather than as a serialised
/// rule, because this is the whole vocabulary the editor offers. A serialised
/// `luna::rules::Recurrence` would store intervals and weekday sets that nothing can
/// enter and nothing can display, and would need migrating the first time that type
/// changed shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    #[default]
    Never,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl Repeat {
    /// The stored word, or `None` for an entry that does not repeat.
    pub fn as_str(self) -> Option<&'static str> {
        return match self {
            Repeat::Never => None,
            Repeat::Daily => Some("daily"),
            Repeat::Weekly => Some("weekly"),
            Repeat::Monthly => Some("monthly"),
            Repeat::Yearly => Some("yearly"),
        };
    }

    /// Reads the stored word back. Anything unrecognised means it does not repeat,
    /// because a rule nobody can interpret must not become a rule that fires forever.
    pub fn from_stored(value: Option<&str>) -> Self {
        return match value {
            Some("daily") => Repeat::Daily,
            Some("weekly") => Repeat::Weekly,
            Some("monthly") => Repeat::Monthly,
            Some("yearly") => Repeat::Yearly,
            _ => Repeat::Never,
        };
    }

    /// The index the editor's dropdown uses, in the order it lists them.
    pub fn as_index(self) -> i32 {
        return match self {
            Repeat::Never => 0,
            Repeat::Daily => 1,
            Repeat::Weekly => 2,
            Repeat::Monthly => 3,
            Repeat::Yearly => 4,
        };
    }

    pub fn from_index(index: i32) -> Self {
        return match index {
            1 => Repeat::Daily,
            2 => Repeat::Weekly,
            3 => Repeat::Monthly,
            4 => Repeat::Yearly,
            _ => Repeat::Never,
        };
    }
}

/// One entry in the calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub id: i64,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub all_day: bool,
    pub repeat: Repeat,
    /// How long before the entry its reminder fires. Zero means no separate reminder.
    pub reminder_lead: Duration,
    pub notes: String,
}

/// The most occurrences one entry can contribute to a single query.
///
/// A daily entry across a year view is 365 of them, which is legitimate; the cap is only
/// here so a pattern that somehow produced instants without advancing cannot spin.
const MAX_OCCURRENCES: usize = 500;

impl CalendarEvent {
    /// How long the entry runs.
    pub fn duration(&self) -> Duration {
        return (self.ends_at - self.starts_at).max(Duration::zero());
    }

    /// The pattern this entry follows, or `None` when it happens once.
    ///
    /// Derived from the entry rather than stored alongside it: a weekly entry repeats on
    /// the weekday it starts on, a monthly one on its day number. Storing those
    /// separately would let them disagree with the date the entry actually has.
    pub fn recurrence(&self) -> Option<Recurrence> {
        let local = self.starts_at.with_timezone(&Local);

        return match self.repeat {
            Repeat::Never => None,
            Repeat::Daily => Some(Recurrence::Daily { interval: 1 }),
            Repeat::Weekly => Some(Recurrence::Weekly {
                interval: 1,
                weekdays: vec![local.weekday()],
            }),
            Repeat::Monthly => Some(Recurrence::Monthly {
                interval: 1,
                day: MonthDay::OnDay(local.day()),
            }),
            Repeat::Yearly => Some(Recurrence::Yearly {
                interval: 1,
                month: local.month(),
                day: local.day(),
            }),
        };
    }

    /// The schedule its occurrences fall on, or `None` when it happens once.
    ///
    /// The same schedule type the scheduler uses, so what the day view draws and what
    /// the reminder fires on cannot drift apart: there is one description of when an
    /// entry happens, and both read it.
    pub fn schedule(&self) -> Option<Schedule> {
        let local = self.starts_at.with_timezone(&Local);
        let at = NaiveTime::from_hms_opt(local.hour(), local.minute(), 0)?;

        // Anchored a moment before the entry, because occurrences are searched over a
        // span that excludes its start. Anchored on the entry itself, the entry's own
        // first occurrence would be the one thing the schedule never produced.
        return Some(Schedule::new(
            self.recurrence()?,
            at,
            self.starts_at - Duration::seconds(1),
        ));
    }

    /// This entry as it falls on one particular occurrence, keeping its length.
    ///
    /// The id is kept, so selecting or deleting an occurrence acts on the entry it came
    /// from. There are no per-occurrence exceptions yet: every occurrence of an entry is
    /// the same entry.
    fn at_occurrence(&self, starts_at: DateTime<Utc>) -> Self {
        let span = self.duration();

        let mut occurrence = self.clone();
        occurrence.starts_at = starts_at;
        occurrence.ends_at = starts_at + span;

        return occurrence;
    }

    /// Every occurrence overlapping `[from, to)`.
    pub fn occurrences_in(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<Self> {
        let Some(schedule) = self.schedule() else {
            // Happens once: it is either in the window or it is not.
            return if self.starts_at < to && self.ends_at > from {
                vec![self.clone()]
            } else {
                Vec::new()
            };
        };

        // Reach back by the entry's own length, so an occurrence that began before the
        // window and is still running inside it is not missed.
        let search_from = from - self.duration() - Duration::seconds(1);

        return schedule
            .occurrences_between(search_from, to, MAX_OCCURRENCES)
            .into_iter()
            .map(|at| self.at_occurrence(at))
            .filter(|occurrence| occurrence.starts_at < to && occurrence.ends_at > from)
            .collect();
    }

    /// The first occurrence still running or yet to come at `after`.
    ///
    /// For the upcoming list, which shows one row per entry rather than every occurrence:
    /// a daily entry would otherwise fill the whole list with itself.
    pub fn next_occurrence(&self, after: DateTime<Utc>) -> Option<Self> {
        let Some(schedule) = self.schedule() else {
            return if self.ends_at > after {
                Some(self.clone())
            } else {
                None
            };
        };

        // Start the search a whole length back, so one that is part way through counts as
        // current rather than being skipped over.
        let mut cursor = after - self.duration() - Duration::seconds(1);

        // At most two steps: the first may still end before `after`, the next cannot.
        for _ in 0..2 {
            let at = schedule.next_after(cursor)?;
            let occurrence = self.at_occurrence(at);

            if occurrence.ends_at > after {
                return Some(occurrence);
            }

            cursor = at;
        }

        return None;
    }
}

/// The calendar's view of the database.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens the shared database, applying any outstanding migrations.
    pub fn open(database: &Path) -> Result<Self> {
        return Ok(Self {
            conn: Database::open(database)?.into_connection(),
        });
    }

    /// Every occurrence overlapping `[from, to)`, earliest first.
    ///
    /// Two queries rather than one, because the two kinds of entry are found in
    /// different ways. A one-off is a row that overlaps the window and SQL can test that
    /// directly. A repeating entry's row sits on its *first* occurrence, which is
    /// usually long before the window and would never match that test, so those are
    /// fetched whole and expanded here.
    ///
    /// What comes back are occurrences, not rows: a weekly entry in a month window
    /// arrives four or five times, each carrying its own date and the id of the entry it
    /// came from. Every view is built on this, so marking the month grid, drawing the
    /// day column and listing what is coming all follow a repeat without knowing it.
    pub fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>> {
        let mut events = self.one_off_events_between(from, to)?;

        for entry in self.repeating_entries(Some(to))? {
            events.extend(entry.occurrences_in(from, to));
        }

        events.sort_by(|a, b| a.starts_at.cmp(&b.starts_at).then(a.id.cmp(&b.id)));

        return Ok(events);
    }

    /// The non-repeating rows overlapping `[from, to)`.
    fn one_off_events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, starts_at, ends_at, all_day, reminder_lead_seconds, notes, recurrence
             FROM calendar_event
             WHERE recurrence IS NULL AND starts_at < ?2 AND ends_at > ?1
             ORDER BY starts_at, id",
        )?;

        let rows = stmt.query_map(
            rusqlite::params![from.timestamp(), to.timestamp()],
            row_to_event,
        )?;

        return collect(rows);
    }

    /// The repeating rows, as stored. `before` drops entries that had not begun yet.
    fn repeating_entries(&self, before: Option<DateTime<Utc>>) -> Result<Vec<CalendarEvent>> {
        let cutoff = before.map(|at| at.timestamp()).unwrap_or(i64::MAX);

        let mut stmt = self.conn.prepare(
            "SELECT id, title, starts_at, ends_at, all_day, reminder_lead_seconds, notes, recurrence
             FROM calendar_event
             WHERE recurrence IS NOT NULL AND starts_at < ?1
             ORDER BY starts_at, id",
        )?;

        let rows = stmt.query_map(rusqlite::params![cutoff], row_to_event)?;

        return collect(rows);
    }

    /// Which local days in `[from, to)` carry at least one entry.
    ///
    /// For the dot on a month cell. Returned as a set because the month grid asks the
    /// question 42 times and a scan per cell would be 42 queries.
    pub fn days_with_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<BTreeSet<NaiveDate>> {
        let mut days = BTreeSet::new();

        for event in self.events_between(from, to)? {
            // Walk the entry rather than taking its start date, so a run of days is all
            // marked. Bounded by the span asked for, so a mis-entered event ending in
            // the year 3000 cannot spin here.
            let mut day = dates::local_date_of(event.starts_at.max(from));
            let last = dates::local_date_of((event.ends_at.min(to)) - Duration::seconds(1));

            while day <= last {
                days.insert(day);
                day += Duration::days(1);
            }
        }

        return Ok(days);
    }

    /// What is coming, earliest first.
    ///
    /// A repeating entry contributes its next occurrence only, not every one it will ever
    /// have. Listing them all would mean a daily entry filling the panel with itself, and
    /// the question the list answers is what is coming up, not how often.
    pub fn upcoming(&self, after: DateTime<Utc>, limit: usize) -> Result<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, starts_at, ends_at, all_day, reminder_lead_seconds, notes, recurrence
             FROM calendar_event
             WHERE recurrence IS NULL AND ends_at > ?1
             ORDER BY starts_at, id
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(
            rusqlite::params![after.timestamp(), limit as i64],
            row_to_event,
        )?;

        let mut events = collect(rows)?;

        for entry in self.repeating_entries(None)? {
            if let Some(next) = entry.next_occurrence(after) {
                events.push(next);
            }
        }

        events.sort_by(|a, b| a.starts_at.cmp(&b.starts_at).then(a.id.cmp(&b.id)));
        events.truncate(limit);

        return Ok(events);
    }

    /// One entry by id, or `None` if it has gone.
    pub fn event(&self, id: i64) -> Result<Option<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, starts_at, ends_at, all_day, reminder_lead_seconds, notes, recurrence
             FROM calendar_event
             WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(rusqlite::params![id], row_to_event)?;

        return match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        };
    }

    /// Adds an entry, returning the id it was given.
    pub fn insert(&self, event: &CalendarEvent) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO calendar_event
                (title, starts_at, ends_at, all_day, reminder_lead_seconds, notes, recurrence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                event.title,
                event.starts_at.timestamp(),
                event.ends_at.timestamp(),
                event.all_day as i64,
                event.reminder_lead.num_seconds(),
                event.notes,
                event.repeat.as_str(),
            ],
        )?;

        return Ok(self.conn.last_insert_rowid());
    }

    /// Rewrites an existing entry, keeping its id and so its reminder.
    pub fn update(&self, event: &CalendarEvent) -> Result<()> {
        self.conn.execute(
            "UPDATE calendar_event SET
                title                 = ?2,
                starts_at             = ?3,
                ends_at               = ?4,
                all_day               = ?5,
                reminder_lead_seconds = ?6,
                notes                 = ?7,
                recurrence            = ?8
             WHERE id = ?1",
            rusqlite::params![
                event.id,
                event.title,
                event.starts_at.timestamp(),
                event.ends_at.timestamp(),
                event.all_day as i64,
                event.reminder_lead.num_seconds(),
                event.notes,
                event.repeat.as_str(),
            ],
        )?;

        return Ok(());
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM calendar_event WHERE id = ?1", rusqlite::params![id])?;

        return Ok(());
    }

    /// Every entry that wants a reminder, for registering with the scheduler.
    pub fn events_with_reminders(&self) -> Result<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, starts_at, ends_at, all_day, reminder_lead_seconds, notes, recurrence
             FROM calendar_event
             ORDER BY starts_at, id",
        )?;

        let rows = stmt.query_map([], row_to_event)?;

        return collect(rows);
    }

    /// The note written against a day, or empty if there is none.
    pub fn note(&self, day: NaiveDate) -> Result<String> {
        let body = self
            .conn
            .query_row(
                "SELECT body FROM calendar_note WHERE day = ?1",
                rusqlite::params![day.to_string()],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .unwrap_or_default();

        return Ok(body);
    }

    /// Writes a day's note, or removes it when the note is emptied.
    ///
    /// Emptying deletes rather than storing a blank, so a year of days the user typed
    /// into and cleared does not become a year of rows.
    pub fn set_note(&self, day: NaiveDate, body: &str, now: DateTime<Utc>) -> Result<()> {
        if body.trim().is_empty() {
            self.conn.execute(
                "DELETE FROM calendar_note WHERE day = ?1",
                rusqlite::params![day.to_string()],
            )?;

            return Ok(());
        }

        self.conn.execute(
            "INSERT INTO calendar_note (day, body, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(day) DO UPDATE SET
                body       = excluded.body,
                updated_at = excluded.updated_at",
            rusqlite::params![day.to_string(), body, now.timestamp()],
        )?;

        return Ok(());
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<CalendarEvent> {
    let starts: i64 = row.get(2)?;
    let ends: i64 = row.get(3)?;
    let all_day: i64 = row.get(4)?;
    let lead: i64 = row.get(5)?;

    return Ok(CalendarEvent {
        id: row.get(0)?,
        title: row.get(1)?,
        starts_at: DateTime::from_timestamp(starts, 0).unwrap_or_default(),
        ends_at: DateTime::from_timestamp(ends, 0).unwrap_or_default(),
        all_day: all_day != 0,
        reminder_lead: Duration::seconds(lead),
        repeat: Repeat::from_stored(row.get::<_, Option<String>>(7)?.as_deref()),
        notes: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
    });
}

fn collect<I>(rows: I) -> Result<Vec<CalendarEvent>>
where
    I: Iterator<Item = rusqlite::Result<CalendarEvent>>,
{
    let mut events = Vec::new();

    for row in rows {
        events.push(row?);
    }

    return Ok(events);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("a temp dir should be available");
        let store = Store::open(&dir.path().join("luna.db")).expect("the store should open");

        return (dir, store);
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        return NaiveDate::from_ymd_opt(y, m, d).expect("test date should be valid");
    }

    fn timed(title: &str, day: NaiveDate, from_hour: i64, to_hour: i64) -> CalendarEvent {
        let (start, _) = dates::day_bounds(day);

        return CalendarEvent {
            id: 0,
            title: title.to_string(),
            starts_at: start + Duration::hours(from_hour),
            ends_at: start + Duration::hours(to_hour),
            all_day: false,
            repeat: Repeat::Never,
            reminder_lead: Duration::zero(),
            notes: String::new(),
        };
    }

    #[test]
    fn an_event_survives_a_round_trip() {
        let (_dir, store) = store();
        let day = date(2026, 8, 14);

        let id = store.insert(&timed("Dentist", day, 9, 10)).unwrap();
        assert!(id > 0);

        let (from, to) = dates::day_bounds(day);
        let found = store.events_between(from, to).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Dentist");
        assert_eq!(found[0].id, id);
    }

    #[test]
    fn a_day_query_ignores_the_days_either_side() {
        let (_dir, store) = store();
        let day = date(2026, 8, 14);

        store.insert(&timed("Today", day, 9, 10)).unwrap();
        store
            .insert(&timed("Yesterday", day - Duration::days(1), 9, 10))
            .unwrap();
        store
            .insert(&timed("Tomorrow", day + Duration::days(1), 9, 10))
            .unwrap();

        let (from, to) = dates::day_bounds(day);
        let found = store.events_between(from, to).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Today");
    }

    #[test]
    fn an_event_running_past_midnight_belongs_to_both_days() {
        let (_dir, store) = store();
        let day = date(2026, 8, 14);

        // 22:00 to 02:00 the next morning.
        store.insert(&timed("Night shift", day, 22, 26)).unwrap();

        for (which, when) in [("first", day), ("second", day + Duration::days(1))] {
            let (from, to) = dates::day_bounds(when);
            let found = store.events_between(from, to).unwrap();

            assert_eq!(found.len(), 1, "should appear on the {which} day");
        }
    }

    #[test]
    fn an_event_ending_exactly_at_midnight_belongs_to_one_day() {
        let (_dir, store) = store();
        let day = date(2026, 8, 14);

        store.insert(&timed("Evening", day, 20, 24)).unwrap();

        let (from, to) = dates::day_bounds(day + Duration::days(1));
        let found = store.events_between(from, to).unwrap();

        assert!(found.is_empty(), "a half-open span must not bleed into the next day");
    }

    #[test]
    fn marked_days_cover_every_day_an_event_runs_through() {
        let (_dir, store) = store();
        let start = date(2026, 8, 14);

        store.insert(&timed("Holiday", start, 0, 72)).unwrap();

        let (from, to) = dates::grid_bounds(start);
        let days = store.days_with_events(from, to).unwrap();

        assert!(days.contains(&start));
        assert!(days.contains(&(start + Duration::days(1))));
        assert!(days.contains(&(start + Duration::days(2))));
        assert!(!days.contains(&(start + Duration::days(3))));
    }

    #[test]
    fn upcoming_is_in_order_and_skips_what_has_finished() {
        let (_dir, store) = store();
        let day = date(2026, 8, 14);

        store.insert(&timed("Later", day, 15, 16)).unwrap();
        store.insert(&timed("Sooner", day, 12, 13)).unwrap();
        store.insert(&timed("Done", day, 6, 7)).unwrap();

        let (start, _) = dates::day_bounds(day);
        let found = store.upcoming(start + Duration::hours(9), 10).unwrap();

        let titles: Vec<&str> = found.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, vec!["Sooner", "Later"]);
    }

    #[test]
    fn a_note_round_trips_and_clearing_it_removes_it() {
        let (_dir, store) = store();
        let day = date(2026, 8, 14);
        let now = Utc::now();

        assert_eq!(store.note(day).unwrap(), "");

        store.set_note(day, "buy a gift", now).unwrap();
        assert_eq!(store.note(day).unwrap(), "buy a gift");

        store.set_note(day, "  ", now).unwrap();
        assert_eq!(store.note(day).unwrap(), "", "a blank note should be removed");
    }

    fn repeating(title: &str, day: NaiveDate, from_hour: i64, repeat: Repeat) -> CalendarEvent {
        let mut event = timed(title, day, from_hour, from_hour + 1);
        event.repeat = repeat;

        return event;
    }

    #[test]
    fn a_weekly_entry_shows_on_every_one_of_its_weekdays() {
        // The point of the whole expansion: the row sits on one date, but the month it
        // starts in has to show it four or five times.
        let (_dir, store) = store();
        let first = date(2026, 8, 4);

        store
            .insert(&repeating("Standup", first, 9, Repeat::Weekly))
            .unwrap();

        let (from, _) = dates::day_bounds(date(2026, 8, 1));
        let (to, _) = dates::day_bounds(date(2026, 9, 1));

        let found = store.events_between(from, to).unwrap();
        let days: Vec<u32> = found
            .iter()
            .map(|e| dates::local_date_of(e.starts_at).day())
            .collect();

        assert_eq!(days, vec![4, 11, 18, 25], "every Tuesday from the 4th");

        for occurrence in &found {
            assert_eq!(occurrence.id, found[0].id, "all one entry");
            assert_eq!(
                occurrence.ends_at - occurrence.starts_at,
                Duration::hours(1),
                "an occurrence keeps the entry's length"
            );
        }
    }

    #[test]
    fn a_repeating_entry_shows_in_a_month_it_did_not_start_in() {
        // The case the old query could never answer: the stored row is in August and the
        // window is October, so nothing overlaps and the entry would simply vanish.
        let (_dir, store) = store();

        store
            .insert(&repeating("Rent", date(2026, 8, 1), 9, Repeat::Monthly))
            .unwrap();

        let (from, _) = dates::day_bounds(date(2026, 10, 1));
        let (to, _) = dates::day_bounds(date(2026, 11, 1));

        let found = store.events_between(from, to).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(
            dates::local_date_of(found[0].starts_at),
            date(2026, 10, 1),
            "on its day of the month, in the month being looked at"
        );
    }

    #[test]
    fn a_daily_entry_marks_every_day_of_the_month() {
        let (_dir, store) = store();

        store
            .insert(&repeating("Pills", date(2026, 8, 1), 8, Repeat::Daily))
            .unwrap();

        let (from, _) = dates::day_bounds(date(2026, 8, 1));
        let (to, _) = dates::day_bounds(date(2026, 9, 1));

        let days = store.days_with_events(from, to).unwrap();

        assert_eq!(days.len(), 31, "August has 31 of them");
        assert!(days.contains(&date(2026, 8, 31)));
    }

    #[test]
    fn a_yearly_entry_comes_back_the_next_year() {
        let (_dir, store) = store();

        store
            .insert(&repeating("Birthday", date(2026, 7, 4), 9, Repeat::Yearly))
            .unwrap();

        let (from, _) = dates::day_bounds(date(2029, 7, 1));
        let (to, _) = dates::day_bounds(date(2029, 8, 1));

        let found = store.events_between(from, to).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(dates::local_date_of(found[0].starts_at), date(2029, 7, 4));
    }

    #[test]
    fn a_repeating_entry_does_not_show_before_it_starts() {
        let (_dir, store) = store();

        store
            .insert(&repeating("Standup", date(2026, 8, 4), 9, Repeat::Weekly))
            .unwrap();

        let (from, _) = dates::day_bounds(date(2026, 7, 1));
        let (to, _) = dates::day_bounds(date(2026, 8, 1));

        assert!(
            store.events_between(from, to).unwrap().is_empty(),
            "a repeat runs forward from its entry, not in both directions"
        );
    }

    #[test]
    fn a_days_occurrence_keeps_its_time_of_day() {
        // What the day column draws. An occurrence three weeks along still starts at the
        // entry's own time, not at midnight or at the original date.
        let (_dir, store) = store();

        store
            .insert(&repeating("Standup", date(2026, 8, 4), 9, Repeat::Weekly))
            .unwrap();

        let day = date(2026, 8, 25);
        let (from, to) = dates::day_bounds(day);

        let found = store.events_between(from, to).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(dates::hours_from(from, found[0].starts_at), 9.0);
        assert_eq!(dates::hours_from(from, found[0].ends_at), 10.0);
    }

    #[test]
    fn upcoming_lists_a_repeating_entry_once() {
        // A daily entry has infinitely many occurrences ahead of it. The list answers
        // what is coming, so it takes the next one and stops.
        let (_dir, store) = store();
        let start = date(2026, 8, 1);

        store
            .insert(&repeating("Pills", start, 8, Repeat::Daily))
            .unwrap();
        store.insert(&timed("Dentist", date(2026, 8, 3), 9, 10)).unwrap();

        let (from, _) = dates::day_bounds(date(2026, 8, 2));
        let found = store.upcoming(from, 40).unwrap();

        let titles: Vec<&str> = found.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, vec!["Pills", "Dentist"]);

        assert_eq!(
            dates::local_date_of(found[0].starts_at),
            date(2026, 8, 2),
            "the next occurrence, not the first one ever"
        );
    }

    #[test]
    fn an_occurrence_part_way_through_still_counts_as_current() {
        let (_dir, store) = store();
        let start = date(2026, 8, 1);

        // 08:00 to 09:00 every day.
        store
            .insert(&repeating("Pills", start, 8, Repeat::Daily))
            .unwrap();

        let (day_start, _) = dates::day_bounds(date(2026, 8, 5));
        let mid = day_start + Duration::minutes(8 * 60 + 30);

        let found = store.upcoming(mid, 10).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(
            dates::local_date_of(found[0].starts_at),
            date(2026, 8, 5),
            "the one running right now, not tomorrow's"
        );
    }

    #[test]
    fn a_deleted_event_is_gone() {
        let (_dir, store) = store();
        let day = date(2026, 8, 14);

        let id = store.insert(&timed("Cancelled", day, 9, 10)).unwrap();
        store.delete(id).unwrap();

        let (from, to) = dates::day_bounds(day);
        assert!(store.events_between(from, to).unwrap().is_empty());
    }
}
