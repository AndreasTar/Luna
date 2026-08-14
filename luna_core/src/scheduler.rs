//! The scheduler: what is due, and what to do about what was missed.
//!
//! One scheduler for the whole app, used by every tool. A health log sampling hourly
//! and a backup running at 02:00 register the same way a reminder does, which is what
//! lets the quit prompt say what will stop happening while Luna is closed.
//!
//! ## Only the next occurrence is ever materialised
//!
//! Jobs store their [`Schedule`], never a computed "next fire" timestamp. A stored
//! instant goes wrong the moment a clock, a timezone or a daylight saving rule moves,
//! and an infinite series has no useful expansion. [`Scheduler::tick`] asks each
//! schedule what falls in the interval since the last tick and recomputes from there.
//!
//! ## Being closed is the normal case
//!
//! Luna is meant to run continuously but will be killed, restarted and rebuilt. Every
//! tick therefore covers a span rather than an instant, and occurrences older than a
//! short grace period are treated as *missed* and put through the job's
//! [`CatchUp`] policy. During ordinary running that span is a few seconds and nothing
//! is ever missed; after a week closed it is a week, and the policy decides whether
//! the user gets one notification or a hundred.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use luna::rules::{missed_to_fire, CatchUp, EventKind, Guard, Schedule};
use rusqlite::Connection;

use crate::error::{CoreError, Result};
use crate::events::EventLog;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// Key under which the last tick is recorded, so a restart knows the gap it slept
/// through.
const LAST_TICK_KEY: &str = "scheduler.last_tick";

/// How recent an occurrence has to be to count as "now" rather than "missed".
///
/// Generous relative to the tick interval so that a slow tick, a busy machine or a
/// laptop lid closing for a moment does not turn an ordinary firing into a caught-up
/// one.
pub const DEFAULT_GRACE: Duration = Duration::minutes(5);

/// The most occurrences one job can produce from a single tick.
///
/// A minute-by-minute job and a year's absence is half a million instants, which
/// nothing downstream wants. The cap bounds the work; the catch-up policy then usually
/// discards most of what survives it anyway.
const MAX_OCCURRENCES_PER_TICK: usize = 500;

/// A registered piece of scheduled work.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledJob {
    /// Identifies the job, and keys its events in the log.
    pub rule_id: String,
    /// The tool that owns it, or `None` for host-owned jobs.
    pub tool_id: Option<String>,
    /// When it comes due.
    pub schedule: Schedule,
    /// A condition checked at each candidate instant. [`Guard::Always`] to fire
    /// unconditionally.
    pub guard: Guard,
    /// How far *before* each occurrence this job fires.
    ///
    /// Zero for something that happens at its scheduled time. Non-zero separates the
    /// alert from the thing it is about: a birthday on the 4th of July with a lead
    /// time of a week fires on the 27th of June, saying what it is for.
    ///
    /// Several alerts for one event are several jobs sharing a schedule with different
    /// lead times, rather than one job with a list. That keeps acknowledging or
    /// snoozing the week-before reminder from touching the day-of one, which is
    /// almost always what is wanted.
    pub lead_time: Duration,

    /// What to do about occurrences missed while Luna was closed.
    pub catch_up: CatchUp,
    /// Whether the job runs at all. A disabled job keeps its definition and history.
    pub enabled: bool,
}

impl ScheduledJob {
    /// A job that fires unconditionally on its schedule.
    pub fn new(rule_id: impl Into<String>, schedule: Schedule) -> Self {
        return Self {
            rule_id: rule_id.into(),
            tool_id: None,
            schedule,
            guard: Guard::Always,
            lead_time: Duration::zero(),
            catch_up: CatchUp::default(),
            enabled: true,
        };
    }

    pub fn owned_by(mut self, tool_id: impl Into<String>) -> Self {
        self.tool_id = Some(tool_id.into());
        return self;
    }

    pub fn guarded_by(mut self, guard: Guard) -> Self {
        self.guard = guard;
        return self;
    }

    /// Fires this far ahead of the scheduled occurrence.
    ///
    /// ## Example
    /// ```ignore
    /// // "Alice's birthday, buy a gift", a week before the day itself.
    /// ScheduledJob::new("birthday.alice.reminder", birthday_schedule)
    ///     .reminding_before(Duration::days(7));
    /// ```
    pub fn reminding_before(mut self, lead_time: Duration) -> Self {
        self.lead_time = lead_time;
        return self;
    }

    pub fn catching_up(mut self, catch_up: CatchUp) -> Self {
        self.catch_up = catch_up;
        return self;
    }
}

/// A job that should fire now.
#[derive(Debug, Clone, PartialEq)]
pub struct Fire {
    pub rule_id: String,
    pub tool_id: Option<String>,
    /// When this alert was due to fire, which is not necessarily now.
    pub due_at: DateTime<Utc>,
    /// The occurrence it concerns.
    ///
    /// The same as `due_at` unless the job has a lead time, in which case this is the
    /// event being warned about and `due_at` is when the warning goes out. A reminder
    /// that cannot say what it is about is not much of a reminder.
    pub subject_at: DateTime<Utc>,
    /// Whether this is being delivered after the fact.
    ///
    /// Worth passing to the user: "this was due at 09:00 on Tuesday" reads very
    /// differently from a notification that appears to be about right now.
    pub late: bool,
}

/// When a job is next expected, for the quit prompt.
#[derive(Debug, Clone, PartialEq)]
pub struct Upcoming {
    pub rule_id: String,
    pub tool_id: Option<String>,
    /// When the alert fires. This is what the quit prompt counts down to.
    pub due_at: DateTime<Utc>,
    /// The occurrence it concerns, equal to `due_at` when there is no lead time.
    pub subject_at: DateTime<Utc>,
}

/// Every registered job, and the log they record into.
pub struct Scheduler {
    conn: Connection,
    log: EventLog,
    jobs: BTreeMap<String, ScheduledJob>,
    grace: Duration,
}

impl Scheduler {
    /// Opens the scheduler against a migrated database, loading its jobs.
    pub fn open(database: &Path) -> Result<Self> {
        let conn = crate::Database::open(database)?.into_connection();
        let log = EventLog::open(database)?;

        let mut scheduler = Self {
            conn,
            log,
            jobs: BTreeMap::new(),
            grace: DEFAULT_GRACE,
        };

        scheduler.reload()?;

        return Ok(scheduler);
    }

    /// Overrides how recent an occurrence must be to count as current.
    pub fn with_grace(mut self, grace: Duration) -> Self {
        self.grace = grace;
        return self;
    }

    /// Reads every job back from the database.
    pub fn reload(&mut self) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "SELECT rule_id, tool_id, schedule, guard, catch_up, enabled, lead_seconds
             FROM scheduled_jobs",
        )?;

        let rows = stmt.query_map([], |row| {
            let rule_id: String = row.get(0)?;
            let tool_id: Option<String> = row.get(1)?;
            let schedule: String = row.get(2)?;
            let guard: String = row.get(3)?;
            let catch_up: String = row.get(4)?;
            let enabled: i64 = row.get(5)?;
            let lead_seconds: i64 = row.get(6)?;

            return Ok((rule_id, tool_id, schedule, guard, catch_up, enabled, lead_seconds));
        })?;

        let mut jobs = BTreeMap::new();

        for row in rows {
            let (rule_id, tool_id, schedule, guard, catch_up, enabled, lead_seconds) = row?;

            // A job that cannot be decoded is skipped rather than failing startup: it
            // can only come from a newer version of Luna, and refusing to start would
            // be a worse outcome than one silent job.
            let Ok(schedule) = serde_json::from_str::<Schedule>(&schedule) else {
                continue;
            };
            let Ok(guard) = serde_json::from_str::<Guard>(&guard) else {
                continue;
            };
            let catch_up = catch_up.parse().unwrap_or_default();

            jobs.insert(
                rule_id.clone(),
                ScheduledJob {
                    rule_id,
                    tool_id,
                    schedule,
                    guard,
                    lead_time: Duration::seconds(lead_seconds),
                    catch_up,
                    enabled: enabled != 0,
                },
            );
        }

        self.jobs = jobs;

        return Ok(());
    }

    /// Registers a job, replacing any existing one with the same id.
    pub fn upsert(&mut self, job: ScheduledJob) -> Result<()> {
        let schedule = serde_json::to_string(&job.schedule)
            .map_err(|e| CoreError::InvalidManifest {
                id: job.rule_id.clone(),
                reason: format!("schedule could not be stored: {e}"),
            })?;

        let guard = serde_json::to_string(&job.guard).map_err(|e| CoreError::InvalidManifest {
            id: job.rule_id.clone(),
            reason: format!("guard could not be stored: {e}"),
        })?;

        self.conn.execute(
            "INSERT INTO scheduled_jobs
                (rule_id, tool_id, schedule, guard, catch_up, enabled, lead_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(rule_id) DO UPDATE SET
                tool_id      = excluded.tool_id,
                schedule     = excluded.schedule,
                guard        = excluded.guard,
                catch_up     = excluded.catch_up,
                enabled      = excluded.enabled,
                lead_seconds = excluded.lead_seconds",
            rusqlite::params![
                job.rule_id,
                job.tool_id,
                schedule,
                guard,
                job.catch_up.as_str(),
                job.enabled as i64,
                job.lead_time.num_seconds()
            ],
        )?;

        self.jobs.insert(job.rule_id.clone(), job);

        return Ok(());
    }

    /// Removes a job. Its recorded history is left alone.
    pub fn remove(&mut self, rule_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM scheduled_jobs WHERE rule_id = ?1",
            rusqlite::params![rule_id],
        )?;

        self.jobs.remove(rule_id);

        return Ok(());
    }

    /// Removes every job belonging to a tool, for when it is uninstalled.
    pub fn remove_tool(&mut self, tool_id: &str) -> Result<usize> {
        let removed = self.conn.execute(
            "DELETE FROM scheduled_jobs WHERE tool_id = ?1",
            rusqlite::params![tool_id],
        )?;

        self.jobs
            .retain(|_, job| job.tool_id.as_deref() != Some(tool_id));

        return Ok(removed);
    }

    pub fn job(&self, rule_id: &str) -> Option<&ScheduledJob> {
        return self.jobs.get(rule_id);
    }

    pub fn jobs(&self) -> impl Iterator<Item = &ScheduledJob> {
        return self.jobs.values();
    }

    /// The event log, for recording completions and acknowledgements.
    pub fn log(&self) -> &EventLog {
        return &self.log;
    }

    /// What each enabled job is next expected to do, soonest first.
    ///
    /// This is what the quit prompt is built from: the user deciding whether to close
    /// Luna needs to know what stops happening if they do.
    pub fn upcoming(&self, after: DateTime<Utc>) -> Vec<Upcoming> {
        let mut out: Vec<Upcoming> = self
            .jobs
            .values()
            .filter(|job| job.enabled)
            .filter_map(|job| {
                // Shift the search by the lead time so the answer is when the alert
                // goes out, which is what a countdown should show.
                job.schedule
                    .next_after(after + job.lead_time)
                    .map(|subject_at| Upcoming {
                        rule_id: job.rule_id.clone(),
                        tool_id: job.tool_id.clone(),
                        due_at: subject_at - job.lead_time,
                        subject_at,
                    })
            })
            .collect();

        out.sort_by_key(|u| u.due_at);

        return out;
    }

    /// The soonest thing due after `after`, if anything.
    pub fn next_due(&self, after: DateTime<Utc>) -> Option<Upcoming> {
        return self.upcoming(after).into_iter().next();
    }

    /// Works out what should fire, records it, and advances the clock mark.
    ///
    /// Covers the whole span since the previous tick, so the same code path handles
    /// ordinary running and coming back after a week closed. Occurrences within the
    /// grace period fire normally; older ones go through the job's catch-up policy.
    ///
    /// Returns what fired, for the caller to notify.
    pub fn tick(&mut self, now: DateTime<Utc>) -> Result<Vec<Fire>> {
        let since = self.last_tick()?.unwrap_or(now);

        // A clock that moved backwards, or a first run. Nothing sensible to catch up
        // on, so start from here rather than replaying an arbitrary span.
        let since = if since > now { now } else { since };

        let mut fires = Vec::new();

        for job in self.jobs.values() {
            if !job.enabled {
                continue;
            }

            // A job with a lead time fires before its occurrence, so the span searched
            // is shifted forward by that much and the results shifted back.
            let occurrences = job.schedule.occurrences_between(
                since + job.lead_time,
                now + job.lead_time,
                MAX_OCCURRENCES_PER_TICK,
            );

            if occurrences.is_empty() {
                continue;
            }

            let subject_of: std::collections::BTreeMap<DateTime<Utc>, DateTime<Utc>> =
                occurrences
                    .iter()
                    .map(|&at| (at - job.lead_time, at))
                    .collect();

            let firing_times: Vec<DateTime<Utc>> = subject_of.keys().copied().collect();

            let cutoff = now - self.grace;
            let (missed, current): (Vec<_>, Vec<_>) =
                firing_times.into_iter().partition(|at| *at < cutoff);

            // Recent ones always fire: the policy is about what was missed, not about
            // suppressing ordinary work.
            for due_at in current {
                fires.push(Fire {
                    rule_id: job.rule_id.clone(),
                    tool_id: job.tool_id.clone(),
                    due_at,
                    subject_at: subject_of.get(&due_at).copied().unwrap_or(due_at),
                    late: false,
                });
            }

            for occurrence in missed_to_fire(&missed, job.catch_up) {
                fires.push(Fire {
                    rule_id: job.rule_id.clone(),
                    tool_id: job.tool_id.clone(),
                    due_at: occurrence.due_at,
                    subject_at: subject_of
                        .get(&occurrence.due_at)
                        .copied()
                        .unwrap_or(occurrence.due_at),
                    late: occurrence.late,
                });
            }
        }

        // Guards are evaluated at the candidate instant, not at `now`: a rule caught up
        // after the fact must see the history as it was relevant to when it was due.
        let mut fired = Vec::new();

        for fire in fires {
            let Some(job) = self.jobs.get(&fire.rule_id) else {
                continue;
            };

            if !job.guard.evaluate(fire.due_at, &self.log) {
                continue;
            }

            self.log.record(
                &fire.rule_id,
                EventKind::Fired,
                fire.due_at,
                fire.tool_id.as_deref(),
                None,
            )?;

            fired.push(fire);
        }

        // A log that could not answer makes every guard above unreliable, so it is
        // reported rather than left for the caller to notice.
        let errors = self.log.take_errors();
        if !errors.is_empty() {
            eprintln!(
                "scheduler: the event log failed {} time(s) during evaluation: {}",
                errors.len(),
                errors.join("; ")
            );
        }

        self.set_last_tick(now)?;

        fired.sort_by_key(|f| f.due_at);

        return Ok(fired);
    }

    /// When the scheduler last ran.
    pub fn last_tick(&self) -> Result<Option<DateTime<Utc>>> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM _luna_meta WHERE key = ?1",
                rusqlite::params![LAST_TICK_KEY],
                |row| row.get(0),
            )
            .ok();

        return Ok(raw
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|secs| chrono::TimeZone::timestamp_opt(&Utc, secs, 0).single()));
    }

    fn set_last_tick(&self, at: DateTime<Utc>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO _luna_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![LAST_TICK_KEY, at.timestamp().to_string()],
        )?;

        return Ok(());
    }
}

/// A running scheduler thread.
///
/// Dropping the handle stops the thread at its next wake.
pub struct SchedulerHandle {
    running: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SchedulerHandle {
    /// Asks the thread to stop and waits for it.
    pub fn stop(mut self) {
        self.signal_stop();

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    fn signal_stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Drop for SchedulerHandle {
    fn drop(&mut self) {
        self.signal_stop();
    }
}

/// Runs a scheduler on its own thread, calling `on_fire` for whatever comes due.
///
/// The thread owns the scheduler, because a `rusqlite::Connection` is `Send` but not
/// `Sync` and there is no reason for anything else to share it.
///
/// It wakes on a fixed interval rather than sleeping until the next occurrence. Timing
/// out repeatedly costs nothing and is immune to the things that break a long sleep:
/// the system suspending, the wall clock being corrected, a timezone changing, or a job
/// being added while the thread is parked.
pub fn spawn(
    database: PathBuf,
    tick_every: StdDuration,
    on_fire: impl Fn(Vec<Fire>) + Send + 'static,
) -> Result<SchedulerHandle> {
    // Opened here rather than on the thread so a failure is reported to the caller
    // instead of vanishing into a thread that immediately exits.
    let mut scheduler = Scheduler::open(&database)?;

    let running = Arc::new(AtomicBool::new(true));
    let flag = running.clone();

    let thread = std::thread::Builder::new()
        .name("luna-scheduler".to_string())
        .spawn(move || {
            // Never sleep for long, so a stop request is acted on promptly.
            let slice = tick_every.min(StdDuration::from_secs(1));
            let mut waited = StdDuration::ZERO;

            while flag.load(Ordering::SeqCst) {
                if waited >= tick_every {
                    waited = StdDuration::ZERO;

                    match scheduler.tick(Utc::now()) {
                        Ok(fires) if !fires.is_empty() => on_fire(fires),
                        Ok(_) => {}
                        Err(e) => eprintln!("scheduler: tick failed: {e}"),
                    }
                }

                std::thread::sleep(slice);
                waited += slice;
            }
        })
        .map_err(|e| CoreError::Io {
            path: database,
            source: e,
        })?;

    return Ok(SchedulerHandle {
        running,
        thread: Some(thread),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, NaiveTime, TimeZone};
    use luna::rules::{EventHistory, Lookback, Recurrence};

    fn scheduler(dir: &tempfile::TempDir) -> Scheduler {
        return Scheduler::open(&dir.path().join("luna.db")).unwrap();
    }

    fn utc(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        return Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap();
    }

    fn daily_at_noon(start: DateTime<Utc>) -> Schedule {
        return Schedule::new(
            Recurrence::daily(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            start,
        );
    }

    #[test]
    fn a_job_survives_reopening() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut s = scheduler(&dir);
            s.upsert(
                ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)))
                    .owned_by("luna.reminders")
                    .catching_up(CatchUp::FireLate),
            )
            .unwrap();
        }

        let s = scheduler(&dir);
        let job = s.job("daily").expect("job should have been reloaded");

        assert_eq!(job.tool_id.as_deref(), Some("luna.reminders"));
        assert_eq!(job.catch_up, CatchUp::FireLate);
        assert!(job.enabled);
    }

    #[test]
    fn a_guard_survives_reopening() {
        let dir = tempfile::tempdir().unwrap();

        let guard = Guard::All(vec![
            Guard::fired("other", Lookback::Yesterday),
            Guard::not_fired("self", Lookback::Today),
        ]);

        {
            let mut s = scheduler(&dir);
            s.upsert(
                ScheduledJob::new("conditional", daily_at_noon(utc(2026, 8, 1, 0)))
                    .guarded_by(guard.clone()),
            )
            .unwrap();
        }

        let s = scheduler(&dir);

        assert_eq!(s.job("conditional").unwrap().guard, guard);
    }

    #[test]
    fn upsert_replaces_rather_than_duplicating() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(ScheduledJob::new("j", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

        let mut updated = ScheduledJob::new("j", daily_at_noon(utc(2026, 8, 1, 0)));
        updated.enabled = false;
        s.upsert(updated).unwrap();

        assert_eq!(s.jobs().count(), 1);
        assert!(!s.job("j").unwrap().enabled);
    }

    #[test]
    fn removing_a_tool_takes_its_jobs_and_leaves_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(ScheduledJob::new("a", daily_at_noon(utc(2026, 8, 1, 0))).owned_by("luna.one"))
            .unwrap();
        s.upsert(ScheduledJob::new("b", daily_at_noon(utc(2026, 8, 1, 0))).owned_by("luna.two"))
            .unwrap();
        s.upsert(ScheduledJob::new("c", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

        assert_eq!(s.remove_tool("luna.one").unwrap(), 1);

        assert!(s.job("a").is_none());
        assert!(s.job("b").is_some());
        assert!(s.job("c").is_some(), "host-owned jobs are untouched");
    }

    #[test]
    fn the_first_tick_does_not_replay_history() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        // A schedule that has been running, on paper, for months.
        s.upsert(ScheduledJob::new("daily", daily_at_noon(utc(2026, 1, 1, 0)))).unwrap();

        let fired = s.tick(utc(2026, 8, 14, 13)).unwrap();

        assert!(
            fired.is_empty(),
            "a first run must not fire months of backlog: {fired:?}"
        );
    }

    #[test]
    fn an_occurrence_within_the_grace_period_fires_normally() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

        // Establish a starting mark, then tick again a little later.
        s.tick(utc(2026, 8, 14, 0)).unwrap();

        let next = s.next_due(utc(2026, 8, 14, 0)).unwrap().due_at;
        let fired = s.tick(next + Duration::seconds(30)).unwrap();

        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].rule_id, "daily");
        assert!(!fired[0].late, "a fresh occurrence is not late");
    }

    #[test]
    fn a_week_of_absence_collapses_to_one_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

        s.tick(utc(2026, 8, 7, 0)).unwrap();

        // Closed for a week.
        let fired = s.tick(utc(2026, 8, 14, 13)).unwrap();

        assert_eq!(
            fired.len(),
            1,
            "the default must not produce a week of duplicates: {fired:?}"
        );
        assert!(fired[0].late);
    }

    #[test]
    fn fire_late_delivers_every_missed_occurrence() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(
            ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)))
                .catching_up(CatchUp::FireLate),
        )
        .unwrap();

        s.tick(utc(2026, 8, 7, 0)).unwrap();
        let fired = s.tick(utc(2026, 8, 14, 13)).unwrap();

        assert!(fired.len() >= 6, "expected a week of occurrences, got {}", fired.len());

        for pair in fired.windows(2) {
            assert!(pair[0].due_at <= pair[1].due_at, "not oldest first");
        }
    }

    #[test]
    fn skip_discards_what_was_missed() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(
            ScheduledJob::new("sample", daily_at_noon(utc(2026, 8, 1, 0)))
                .catching_up(CatchUp::Skip),
        )
        .unwrap();

        s.tick(utc(2026, 8, 7, 0)).unwrap();
        let fired = s.tick(utc(2026, 8, 14, 13)).unwrap();

        assert!(fired.is_empty(), "{fired:?}");
    }

    #[test]
    fn a_disabled_job_does_not_fire() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        let mut job = ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)));
        job.enabled = false;
        s.upsert(job).unwrap();

        s.tick(utc(2026, 8, 7, 0)).unwrap();

        assert!(s.tick(utc(2026, 8, 14, 13)).unwrap().is_empty());
    }

    #[test]
    fn a_guard_can_suppress_a_firing() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        // Only fire if something else fired yesterday, which nothing has.
        s.upsert(
            ScheduledJob::new("conditional", daily_at_noon(utc(2026, 8, 1, 0)))
                .guarded_by(Guard::fired("prerequisite", Lookback::Yesterday)),
        )
        .unwrap();

        s.tick(utc(2026, 8, 7, 0)).unwrap();

        assert!(s.tick(utc(2026, 8, 14, 13)).unwrap().is_empty());
    }

    #[test]
    fn a_satisfied_guard_lets_the_firing_through() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(
            ScheduledJob::new("conditional", daily_at_noon(utc(2026, 8, 1, 0)))
                .guarded_by(Guard::fired("prerequisite", Lookback::LastDays(30))),
        )
        .unwrap();

        s.tick(utc(2026, 8, 7, 0)).unwrap();

        s.log()
            .record("prerequisite", EventKind::Fired, utc(2026, 8, 10, 9), None, None)
            .unwrap();

        assert!(!s.tick(utc(2026, 8, 14, 13)).unwrap().is_empty());
    }

    #[test]
    fn firing_is_recorded_so_later_guards_can_see_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

        s.tick(utc(2026, 8, 7, 0)).unwrap();
        let fired = s.tick(utc(2026, 8, 14, 13)).unwrap();

        assert_eq!(fired.len(), 1);

        let recorded = s.log().last("daily", EventKind::Fired);
        assert_eq!(recorded, Some(fired[0].due_at));
    }

    #[test]
    fn a_backwards_clock_does_not_replay_everything() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

        s.tick(utc(2026, 8, 14, 12)).unwrap();

        // The wall clock is corrected backwards by a month.
        let fired = s.tick(utc(2026, 7, 14, 12)).unwrap();

        assert!(fired.is_empty(), "{fired:?}");
    }

    #[test]
    fn ticking_twice_over_the_same_span_does_not_fire_twice() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

        s.tick(utc(2026, 8, 7, 0)).unwrap();

        let first = s.tick(utc(2026, 8, 14, 13)).unwrap();
        let second = s.tick(utc(2026, 8, 14, 13)).unwrap();

        assert_eq!(first.len(), 1);
        assert!(second.is_empty(), "the mark must advance: {second:?}");
    }

    /// A yearly event, as a birthday would be.
    fn yearly(month: u32, day: u32) -> Schedule {
        return Schedule::new(
            Recurrence::Yearly { interval: 1, month, day },
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            utc(2026, 1, 1, 0),
        );
    }

    #[test]
    fn a_reminder_fires_ahead_of_the_event_and_says_what_it_is_about() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        // "Alice's birthday, buy a gift", a week before the 4th of July.
        s.upsert(
            ScheduledJob::new("birthday.alice.reminder", yearly(7, 4))
                .reminding_before(Duration::days(7)),
        )
        .unwrap();

        let upcoming = s.next_due(utc(2026, 6, 1, 0)).unwrap();

        let fires_on = upcoming.due_at.with_timezone(&chrono::Local).date_naive();
        let event_on = upcoming.subject_at.with_timezone(&chrono::Local).date_naive();

        assert_eq!(event_on.month(), 7);
        assert_eq!(event_on.day(), 4);
        assert_eq!(
            (event_on - fires_on).num_days(),
            7,
            "the alert should land a week before the day itself"
        );
    }

    #[test]
    fn a_lead_time_reminder_actually_fires_at_the_lead_time() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(
            ScheduledJob::new("birthday.alice.reminder", yearly(7, 4))
                .reminding_before(Duration::days(7)),
        )
        .unwrap();

        // Nothing yet in June, well before the lead time.
        s.tick(utc(2026, 6, 1, 0)).unwrap();
        assert!(s.tick(utc(2026, 6, 20, 0)).unwrap().is_empty());

        // Ticking past the 27th of June delivers it, a week ahead of the event.
        let fired = s.tick(utc(2026, 6, 28, 12)).unwrap();

        assert_eq!(fired.len(), 1, "{fired:?}");
        assert!(
            fired[0].subject_at > fired[0].due_at,
            "the event must be ahead of the alert"
        );
        assert_eq!((fired[0].subject_at - fired[0].due_at).num_days(), 7);
    }

    #[test]
    fn the_event_itself_and_its_reminder_are_independent_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        // Two alerts for one event: one a week out, one on the day.
        s.upsert(
            ScheduledJob::new("birthday.alice.week", yearly(7, 4))
                .reminding_before(Duration::days(7)),
        )
        .unwrap();
        s.upsert(ScheduledJob::new("birthday.alice.day", yearly(7, 4))).unwrap();

        let upcoming = s.upcoming(utc(2026, 6, 1, 0));

        assert_eq!(upcoming.len(), 2);
        assert_eq!(upcoming[0].rule_id, "birthday.alice.week", "the earlier alert first");
        assert_eq!(upcoming[1].rule_id, "birthday.alice.day");

        // Both concern the same moment, and each can be acknowledged on its own.
        assert_eq!(upcoming[0].subject_at, upcoming[1].subject_at);
    }

    #[test]
    fn a_lead_time_survives_reopening() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut s = scheduler(&dir);
            s.upsert(
                ScheduledJob::new("birthday.alice.week", yearly(7, 4))
                    .reminding_before(Duration::days(7)),
            )
            .unwrap();
        }

        let s = scheduler(&dir);

        assert_eq!(
            s.job("birthday.alice.week").unwrap().lead_time,
            Duration::days(7)
        );
    }

    #[test]
    fn without_a_lead_time_the_alert_and_the_event_are_the_same_moment() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

        let upcoming = s.next_due(utc(2026, 8, 14, 0)).unwrap();

        assert_eq!(upcoming.due_at, upcoming.subject_at);
    }

    #[test]
    fn upcoming_lists_enabled_jobs_soonest_first() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        let at = |h: u32| {
            Schedule::new(
                Recurrence::daily(),
                NaiveTime::from_hms_opt(h, 0, 0).unwrap(),
                utc(2026, 8, 1, 0),
            )
        };

        s.upsert(ScheduledJob::new("late", at(23))).unwrap();
        s.upsert(ScheduledJob::new("early", at(1))).unwrap();

        let mut off = ScheduledJob::new("disabled", at(2));
        off.enabled = false;
        s.upsert(off).unwrap();

        let upcoming = s.upcoming(utc(2026, 8, 14, 0));

        let ids: Vec<&str> = upcoming.iter().map(|u| u.rule_id.as_str()).collect();

        assert!(!ids.contains(&"disabled"), "disabled jobs are not upcoming");
        assert_eq!(ids.len(), 2);

        for pair in upcoming.windows(2) {
            assert!(pair[0].due_at <= pair[1].due_at, "not soonest first");
        }
    }

    #[test]
    fn next_due_is_the_soonest_upcoming() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = scheduler(&dir);

        s.upsert(ScheduledJob::new("daily", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

        let next = s.next_due(utc(2026, 8, 14, 0)).unwrap();

        assert_eq!(next.rule_id, "daily");
        assert!(next.due_at > utc(2026, 8, 14, 0));
    }

    #[test]
    fn next_due_is_none_when_nothing_is_scheduled() {
        let dir = tempfile::tempdir().unwrap();
        let s = scheduler(&dir);

        assert_eq!(s.next_due(utc(2026, 8, 14, 0)), None);
    }

    #[test]
    fn an_undecodable_job_is_skipped_rather_than_failing_startup() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut s = scheduler(&dir);
            s.upsert(ScheduledJob::new("good", daily_at_noon(utc(2026, 8, 1, 0)))).unwrap();

            // As a newer version of Luna might have written.
            s.conn
                .execute(
                    "INSERT INTO scheduled_jobs (rule_id, schedule, guard, catch_up, enabled)
                     VALUES ('future', '{\"unknown\":1}', 'null', 'skip', 1)",
                    [],
                )
                .unwrap();
        }

        let s = scheduler(&dir);

        assert!(s.job("good").is_some(), "the readable job still loads");
        assert!(s.job("future").is_none());
    }

    #[test]
    fn the_background_thread_runs_and_stops() {
        use std::sync::mpsc;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("luna.db");

        {
            let mut s = Scheduler::open(&path).unwrap();
            // Every minute, starting well in the past, so a tick has something to do.
            s.upsert(
                ScheduledJob::new("often", daily_at_noon(utc(2020, 1, 1, 0)))
                    .catching_up(CatchUp::CollapseToOne),
            )
            .unwrap();
            // Leave a mark far enough back that the next tick sees a gap.
            s.tick(Utc::now() - Duration::days(2)).unwrap();
        }

        let (tx, rx) = mpsc::channel();

        let handle = spawn(path, StdDuration::from_millis(50), move |fires| {
            let _ = tx.send(fires.len());
        })
        .unwrap();

        let got = rx.recv_timeout(StdDuration::from_secs(5));

        handle.stop();

        assert!(got.is_ok(), "the thread should have ticked and fired something");
    }
}
