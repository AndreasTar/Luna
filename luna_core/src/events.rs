//! The event log: what has happened to scheduled rules.
//!
//! This is the substrate guards query. A guard such as *"did reminder A fire
//! yesterday"* is a count over this table, and a window task anchored on
//! `RollingFromCompletion` finds its start here.
//!
//! [`EventLog`] implements [`luna::rules::EventHistory`], which is how the pure rule
//! engine reaches storage without knowing anything about SQLite.
//!
//! ## Errors during evaluation
//!
//! [`EventHistory`] is deliberately infallible, so that guard evaluation stays simple
//! and total. A query can still fail, though, if the database is corrupt or the disk
//! has gone. Rather than silently answering "nothing happened", which would make
//! guards quietly wrong, failures are counted and kept: a caller evaluating a batch of
//! rules checks [`EventLog::take_errors`] afterwards and can refuse to act on results
//! that were computed against a broken log.

use std::cell::RefCell;

use chrono::{DateTime, TimeZone, Utc};
use luna::rules::{EventHistory, EventKind, TimeWindow};
use rusqlite::Connection;

use crate::error::Result;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// One recorded event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleEvent {
    pub id: i64,
    pub rule_id: String,
    pub kind: EventKind,
    pub at: DateTime<Utc>,
    /// The tool that owns the rule, or `None` for host-owned rules.
    pub tool_id: Option<String>,
    pub note: Option<String>,
}

/// Append-only history of scheduled rules, backed by SQLite.
pub struct EventLog {
    conn: Connection,
    /// Query failures seen while answering [`EventHistory`], which cannot report them.
    errors: RefCell<Vec<String>>,
}

impl EventLog {
    /// Opens a log against an already-migrated database file.
    ///
    /// Takes its own connection rather than sharing one. `rusqlite::Connection` is
    /// `Send` but not `Sync`, and rules are evaluated off the UI thread, so a shared
    /// handle would need a lock every caller contends on. WAL mode is built for
    /// several connections to one file.
    pub fn open(database: &std::path::Path) -> Result<Self> {
        let db = crate::Database::open(database)?;
        return Ok(Self::from_database(db));
    }

    /// Wraps an open database.
    pub fn from_database(db: crate::Database) -> Self {
        return Self {
            conn: db.into_connection(),
            errors: RefCell::new(Vec::new()),
        };
    }

    /// Records an event.
    ///
    /// `at` is stored as an absolute UTC instant. Never a duration: a deadline stored
    /// as "in N seconds" does not survive the app being closed, which is the whole
    /// problem the scheduler exists to solve.
    pub fn record(
        &self,
        rule_id: &str,
        kind: EventKind,
        at: DateTime<Utc>,
        tool_id: Option<&str>,
        note: Option<&str>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO rule_events (rule_id, kind, at, tool_id, note)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![rule_id, kind.as_str(), at.timestamp(), tool_id, note],
        )?;

        return Ok(self.conn.last_insert_rowid());
    }

    /// Every event for a rule, newest first.
    pub fn events_for(&self, rule_id: &str, limit: usize) -> Result<Vec<RuleEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, rule_id, kind, at, tool_id, note
             FROM rule_events
             WHERE rule_id = ?1
             ORDER BY at DESC, id DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(rusqlite::params![rule_id, limit as i64], row_to_event)?;

        let mut events = Vec::new();
        for row in rows {
            // A row with an unrecognised kind is skipped rather than failing the whole
            // query: it can only come from a newer version of Luna having written it.
            if let Some(event) = row? {
                events.push(event);
            }
        }

        return Ok(events);
    }

    /// Deletes events older than `cutoff`, returning how many went.
    ///
    /// Retention is the caller's policy. The log grows without bound otherwise, and a
    /// health tool sampling every minute would dominate the database within months.
    pub fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<usize> {
        let removed = self.conn.execute(
            "DELETE FROM rule_events WHERE at < ?1",
            rusqlite::params![cutoff.timestamp()],
        )?;

        return Ok(removed);
    }

    /// Deletes every event belonging to a tool.
    ///
    /// For when a tool is removed for good. Not called on disable: a disabled tool's
    /// history is exactly what it needs when it comes back.
    pub fn remove_tool(&self, tool_id: &str) -> Result<usize> {
        let removed = self.conn.execute(
            "DELETE FROM rule_events WHERE tool_id = ?1",
            rusqlite::params![tool_id],
        )?;

        return Ok(removed);
    }

    /// How many events are stored.
    pub fn len(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM rule_events", [], |row| row.get(0))?;

        return Ok(count as usize);
    }

    /// Query failures seen since the last call, clearing them.
    ///
    /// Empty is the normal case. Anything here means guard results computed since the
    /// last check were answered against an unreadable log and should not be trusted.
    pub fn take_errors(&self) -> Vec<String> {
        return std::mem::take(&mut self.errors.borrow_mut());
    }

    /// Whether any query has failed since the last [`EventLog::take_errors`].
    pub fn has_errors(&self) -> bool {
        return !self.errors.borrow().is_empty();
    }

    fn note_error(&self, context: &str, error: rusqlite::Error) {
        self.errors
            .borrow_mut()
            .push(format!("{context}: {error}"));
    }
}

impl EventHistory for EventLog {
    fn count(&self, rule: &str, kind: EventKind, window: TimeWindow) -> usize {
        // Half-open to match TimeWindow, so adjacent lookbacks such as Today and
        // Yesterday cannot both claim the same event.
        let result: rusqlite::Result<i64> = self.conn.query_row(
            "SELECT COUNT(*) FROM rule_events
             WHERE rule_id = ?1 AND kind = ?2 AND at >= ?3 AND at < ?4",
            rusqlite::params![
                rule,
                kind.as_str(),
                window.start.timestamp(),
                window.end.timestamp()
            ],
            |row| row.get(0),
        );

        return match result {
            Ok(count) => count as usize,
            Err(e) => {
                self.note_error(&format!("counting {kind} for {rule:?}"), e);
                0
            }
        };
    }

    fn last(&self, rule: &str, kind: EventKind) -> Option<DateTime<Utc>> {
        let result: rusqlite::Result<Option<i64>> = self.conn.query_row(
            "SELECT MAX(at) FROM rule_events WHERE rule_id = ?1 AND kind = ?2",
            rusqlite::params![rule, kind.as_str()],
            |row| row.get(0),
        );

        return match result {
            Ok(Some(seconds)) => Utc.timestamp_opt(seconds, 0).single(),
            Ok(None) => None,
            Err(e) => {
                self.note_error(&format!("finding the last {kind} for {rule:?}"), e);
                None
            }
        };
    }
}

/// Reads a row, returning `None` for an unrecognised event kind.
fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Option<RuleEvent>> {
    let kind_text: String = row.get(2)?;

    let Ok(kind) = kind_text.parse::<EventKind>() else {
        return Ok(None);
    };

    let seconds: i64 = row.get(3)?;
    let Some(at) = Utc.timestamp_opt(seconds, 0).single() else {
        return Ok(None);
    };

    return Ok(Some(RuleEvent {
        id: row.get(0)?,
        rule_id: row.get(1)?,
        kind,
        at,
        tool_id: row.get(4)?,
        note: row.get(5)?,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use luna::rules::{Guard, Lookback};

    fn log() -> EventLog {
        return EventLog::from_database(crate::Database::open_in_memory().unwrap());
    }

    fn utc(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        return Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap();
    }

    #[test]
    fn records_and_reads_back() {
        let log = log();
        let at = utc(2026, 8, 14, 12);

        let id = log
            .record("a", EventKind::Fired, at, Some("luna.reminders"), Some("hello"))
            .unwrap();

        assert!(id > 0);

        let events = log.events_for("a", 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].rule_id, "a");
        assert_eq!(events[0].kind, EventKind::Fired);
        assert_eq!(events[0].at, at);
        assert_eq!(events[0].tool_id.as_deref(), Some("luna.reminders"));
        assert_eq!(events[0].note.as_deref(), Some("hello"));
    }

    #[test]
    fn counting_respects_rule_kind_and_window() {
        let log = log();

        log.record("a", EventKind::Fired, utc(2026, 8, 14, 9), None, None).unwrap();
        log.record("a", EventKind::Snoozed, utc(2026, 8, 14, 10), None, None).unwrap();
        log.record("b", EventKind::Fired, utc(2026, 8, 14, 11), None, None).unwrap();

        let day = TimeWindow::new(utc(2026, 8, 14, 0), utc(2026, 8, 15, 0));

        assert_eq!(log.count("a", EventKind::Fired, day), 1);
        assert_eq!(log.count("a", EventKind::Snoozed, day), 1);
        assert_eq!(log.count("b", EventKind::Fired, day), 1);
        assert_eq!(log.count("c", EventKind::Fired, day), 0);

        let earlier = TimeWindow::new(utc(2026, 8, 13, 0), utc(2026, 8, 14, 0));
        assert_eq!(log.count("a", EventKind::Fired, earlier), 0);
    }

    #[test]
    fn counting_is_half_open_so_adjacent_windows_cannot_double_count() {
        let log = log();
        let boundary = utc(2026, 8, 14, 0);

        log.record("a", EventKind::Fired, boundary, None, None).unwrap();

        let before = TimeWindow::new(utc(2026, 8, 13, 0), boundary);
        let after = TimeWindow::new(boundary, utc(2026, 8, 15, 0));

        assert_eq!(log.count("a", EventKind::Fired, before), 0, "end is exclusive");
        assert_eq!(log.count("a", EventKind::Fired, after), 1, "start is inclusive");
    }

    #[test]
    fn last_finds_the_most_recent_of_that_kind() {
        let log = log();

        log.record("oil", EventKind::Completed, utc(2026, 1, 1, 0), None, None).unwrap();
        log.record("oil", EventKind::Completed, utc(2026, 6, 1, 0), None, None).unwrap();
        log.record("oil", EventKind::Fired, utc(2026, 8, 1, 0), None, None).unwrap();

        assert_eq!(log.last("oil", EventKind::Completed), Some(utc(2026, 6, 1, 0)));
        assert_eq!(log.last("oil", EventKind::Fired), Some(utc(2026, 8, 1, 0)));
        assert_eq!(log.last("oil", EventKind::Snoozed), None);
        assert_eq!(log.last("nothing", EventKind::Completed), None);
    }

    #[test]
    fn last_is_not_bounded_by_any_lookback() {
        let log = log();

        // A maintenance task done years ago must still anchor its window; capping this
        // would silently restart the schedule.
        let long_ago = utc(2020, 3, 4, 5);
        log.record("oil", EventKind::Completed, long_ago, None, None).unwrap();

        assert_eq!(log.last("oil", EventKind::Completed), Some(long_ago));
    }

    #[test]
    fn a_real_guard_evaluates_against_the_stored_log() {
        let log = log();

        // The motivating case, end to end through SQLite this time.
        let candidate = utc(2026, 8, 14, 12);
        let guard = Guard::fired("a", Lookback::Yesterday);

        assert!(!guard.evaluate(candidate, &log));

        // Placed inside yesterday by asking the lookback where that is, rather than
        // assuming UTC midnight is local midnight. It is not, in most of the world.
        let yesterday = Lookback::Yesterday.window(candidate);
        let inside = yesterday.start + yesterday.duration() / 2;

        log.record("a", EventKind::Fired, inside, None, None).unwrap();

        assert!(guard.evaluate(candidate, &log));
        assert!(!log.has_errors(), "{:?}", log.take_errors());
    }

    #[test]
    fn a_compound_guard_evaluates_against_the_stored_log() {
        let log = log();
        let at = utc(2026, 8, 14, 12);

        // Anchored to the lookback's own window so the test holds in any timezone.
        let today = Lookback::Today.window(at);
        let inside = today.start + today.duration() / 2;

        log.record("a", EventKind::Fired, inside, None, None).unwrap();

        let guard = Guard::All(vec![
            Guard::fired("a", Lookback::Today),
            Guard::not_fired("b", Lookback::Today),
        ]);

        assert!(guard.evaluate(at, &log));

        log.record("b", EventKind::Fired, inside, None, None).unwrap();

        assert!(!guard.evaluate(at, &log));
    }

    #[test]
    fn events_come_back_newest_first_and_respect_the_limit() {
        let log = log();

        for hour in [9, 10, 11, 12] {
            log.record("a", EventKind::Fired, utc(2026, 8, 14, hour), None, None).unwrap();
        }

        let events = log.events_for("a", 2).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].at, utc(2026, 8, 14, 12));
        assert_eq!(events[1].at, utc(2026, 8, 14, 11));
    }

    #[test]
    fn pruning_removes_only_what_is_older_than_the_cutoff() {
        let log = log();

        log.record("a", EventKind::Fired, utc(2026, 1, 1, 0), None, None).unwrap();
        log.record("a", EventKind::Fired, utc(2026, 6, 1, 0), None, None).unwrap();
        log.record("a", EventKind::Fired, utc(2026, 8, 1, 0), None, None).unwrap();

        let removed = log.prune_before(utc(2026, 5, 1, 0)).unwrap();

        assert_eq!(removed, 1);
        assert_eq!(log.len().unwrap(), 2);
    }

    #[test]
    fn removing_a_tool_takes_only_its_own_history() {
        let log = log();

        log.record("a", EventKind::Fired, utc(2026, 8, 1, 0), Some("luna.one"), None).unwrap();
        log.record("b", EventKind::Fired, utc(2026, 8, 1, 0), Some("luna.two"), None).unwrap();
        log.record("c", EventKind::Fired, utc(2026, 8, 1, 0), None, None).unwrap();

        let removed = log.remove_tool("luna.one").unwrap();

        assert_eq!(removed, 1);
        assert_eq!(log.len().unwrap(), 2);
        assert_eq!(log.events_for("a", 10).unwrap().len(), 0);
        assert_eq!(log.events_for("b", 10).unwrap().len(), 1);
        assert_eq!(log.events_for("c", 10).unwrap().len(), 1, "host rules are untouched");
    }

    #[test]
    fn an_unrecognised_kind_is_skipped_rather_than_failing_the_query() {
        let log = log();

        log.record("a", EventKind::Fired, utc(2026, 8, 14, 12), None, None).unwrap();

        // As a newer version of Luna might have written.
        log.conn
            .execute(
                "INSERT INTO rule_events (rule_id, kind, at) VALUES ('a', 'teleported', 1)",
                [],
            )
            .unwrap();

        let events = log.events_for("a", 10).unwrap();

        assert_eq!(events.len(), 1, "the readable row still comes back");
        assert_eq!(events[0].kind, EventKind::Fired);
    }

    #[test]
    fn history_survives_reopening_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("luna.db");

        {
            let log = EventLog::open(&path).unwrap();
            log.record("a", EventKind::Completed, utc(2026, 8, 14, 12), None, None)
                .unwrap();
        }

        let log = EventLog::open(&path).unwrap();

        assert_eq!(log.last("a", EventKind::Completed), Some(utc(2026, 8, 14, 12)));
    }

    #[test]
    fn a_failing_query_is_recorded_rather_than_answered_as_nothing_happened() {
        let log = log();

        // Drop the table out from under it, standing in for a corrupt log.
        log.conn.execute_batch("DROP TABLE rule_events;").unwrap();

        let day = TimeWindow::new(utc(2026, 8, 14, 0), utc(2026, 8, 15, 0));

        assert_eq!(log.count("a", EventKind::Fired, day), 0);
        assert_eq!(log.last("a", EventKind::Fired), None);

        // The zero above is indistinguishable from "nothing happened", which is
        // exactly why the failure has to be recoverable by the caller.
        assert!(log.has_errors());

        let errors = log.take_errors();
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(!log.has_errors(), "taking clears them");
    }
}
