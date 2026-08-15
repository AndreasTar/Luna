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

use chrono::{DateTime, Duration, NaiveDate, Utc};
use luna_core::rusqlite::{self, Connection};
use luna_core::{Database, Result};

use super::dates;

/// One entry in the calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub id: i64,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub all_day: bool,
    /// How long before the entry its reminder fires. Zero means no separate reminder.
    pub reminder_lead: Duration,
    pub notes: String,
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

    /// Every entry overlapping `[from, to)`, earliest first.
    ///
    /// Overlap rather than containment, so a multi-day entry shows up on every day it
    /// runs through rather than only the one it started on.
    pub fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, starts_at, ends_at, all_day, reminder_lead_seconds, notes
             FROM calendar_event
             WHERE starts_at < ?2 AND ends_at > ?1
             ORDER BY starts_at, id",
        )?;

        let rows = stmt.query_map(
            rusqlite::params![from.timestamp(), to.timestamp()],
            row_to_event,
        )?;

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

    /// The next entries starting at or after `after`.
    pub fn upcoming(&self, after: DateTime<Utc>, limit: usize) -> Result<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, starts_at, ends_at, all_day, reminder_lead_seconds, notes
             FROM calendar_event
             WHERE ends_at > ?1
             ORDER BY starts_at, id
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(
            rusqlite::params![after.timestamp(), limit as i64],
            row_to_event,
        )?;

        return collect(rows);
    }

    /// Adds an entry, returning the id it was given.
    pub fn insert(&self, event: &CalendarEvent) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO calendar_event
                (title, starts_at, ends_at, all_day, reminder_lead_seconds, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                event.title,
                event.starts_at.timestamp(),
                event.ends_at.timestamp(),
                event.all_day as i64,
                event.reminder_lead.num_seconds(),
                event.notes,
            ],
        )?;

        return Ok(self.conn.last_insert_rowid());
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM calendar_event WHERE id = ?1", rusqlite::params![id])?;

        return Ok(());
    }

    /// Every entry that wants a reminder, for registering with the scheduler.
    pub fn events_with_reminders(&self) -> Result<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, starts_at, ends_at, all_day, reminder_lead_seconds, notes
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
