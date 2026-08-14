//! Rules for scheduled work: conditions over past events, and deadlines that escalate.
//!
//! Two shapes of scheduled thing, which need different machinery.
//!
//! **Instants** fire at a moment. Each one is a temporal generator plus an optional
//! [`Guard`]: the generator proposes candidate moments from the clock alone, and the
//! guard decides whether a given candidate actually fires by looking at what has
//! happened before. So *"if reminder A fired yesterday, remind me today at 12am"* is a
//! daily generator plus `Guard::Fired { rule: "A", within: Lookback::Yesterday }`.
//!
//! **Windows** ([`WindowShape`]) have no single moment. They describe work that should
//! happen *somewhere within a period*, growing more urgent as the period elapses:
//! *"check the oil every 3 months"*. Their state is a function of the current time, so
//! it can be recomputed at any point rather than fired once.
//!
//! This module is pure. It has no clock and no storage: callers pass in the current
//! time and something implementing [`EventHistory`]. That keeps every rule testable at
//! an arbitrary instant, which matters enormously for anything calendar-shaped.
//!
//! # Why guards cannot loop
//!
//! A guard only ever reads events strictly in the past, and is only evaluated at a
//! candidate instant. Two rules that guard on each other therefore cannot trigger each
//! other endlessly: each evaluation looks backwards at an already-settled history.
//! Lookbacks are additionally capped at [`MAX_LOOKBACK_DAYS`] so a mistaken rule cannot
//! make evaluation arbitrarily expensive.

use chrono::{DateTime, Datelike, Duration as ChronoDuration, Local, TimeZone, Utc};

pub const VERSION: crate::Version = crate::Version::new(0, 1, 0);

/// The furthest back a guard may look.
///
/// A cap rather than a limitation: no sensible rule asks "did this fire in the last
/// three years", and without a bound a typo could make every evaluation scan the whole
/// event log.
pub const MAX_LOOKBACK_DAYS: i64 = 366;

/// Something that happened to a scheduled rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKind {
    /// The rule fired and the user was notified.
    Fired,
    /// The user acknowledged it.
    Acknowledged,
    /// The user pushed it back.
    Snoozed,
    /// The underlying task was marked done. Rolls the next window.
    Completed,
    /// The user dismissed it without acting.
    Dismissed,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        return match self {
            EventKind::Fired => "fired",
            EventKind::Acknowledged => "acknowledged",
            EventKind::Snoozed => "snoozed",
            EventKind::Completed => "completed",
            EventKind::Dismissed => "dismissed",
        };
    }
}

impl std::str::FromStr for EventKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s {
            "fired" => Ok(EventKind::Fired),
            "acknowledged" => Ok(EventKind::Acknowledged),
            "snoozed" => Ok(EventKind::Snoozed),
            "completed" => Ok(EventKind::Completed),
            "dismissed" => Ok(EventKind::Dismissed),
            other => Err(format!("{other:?} is not a known event kind")),
        };
    }
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.as_str());
    }
}

/// A half-open span of time, `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeWindow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeWindow {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        // A reversed span is a caller mistake that would otherwise silently match
        // nothing, so it is normalised rather than trusted.
        return if start <= end {
            Self { start, end }
        } else {
            Self { start: end, end: start }
        };
    }

    pub fn contains(&self, at: DateTime<Utc>) -> bool {
        return at >= self.start && at < self.end;
    }

    pub fn duration(&self) -> ChronoDuration {
        return self.end - self.start;
    }
}

/// How far back a guard looks from the instant being evaluated.
///
/// Calendar variants exist because "yesterday" is not "24 hours ago": a reminder that
/// fired at 23:00 yesterday is still *yesterday* when the candidate runs at 00:00
/// today, and a plain duration would miss most of the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookback {
    /// A fixed span ending at the evaluation instant.
    LastMinutes(i64),
    LastHours(i64),
    LastDays(i64),
    /// The local calendar day containing the evaluation instant, from midnight.
    Today,
    /// The whole local calendar day before it.
    Yesterday,
    /// The local calendar month containing the evaluation instant.
    ThisMonth,
}

impl Lookback {
    /// Resolves to a concrete span, given the instant being evaluated.
    ///
    /// Calendar variants are computed in local time, because that is what a user means
    /// by "yesterday", then converted back to UTC for querying.
    pub fn window(self, at: DateTime<Utc>) -> TimeWindow {
        let capped = |days: i64| days.clamp(0, MAX_LOOKBACK_DAYS);

        return match self {
            Lookback::LastMinutes(n) => {
                let n = n.clamp(0, MAX_LOOKBACK_DAYS * 24 * 60);
                TimeWindow::new(at - ChronoDuration::minutes(n), at)
            }
            Lookback::LastHours(n) => {
                let n = n.clamp(0, MAX_LOOKBACK_DAYS * 24);
                TimeWindow::new(at - ChronoDuration::hours(n), at)
            }
            Lookback::LastDays(n) => {
                TimeWindow::new(at - ChronoDuration::days(capped(n)), at)
            }
            Lookback::Today => {
                let start = local_midnight(at);
                TimeWindow::new(start, at)
            }
            Lookback::Yesterday => {
                let today = local_midnight(at);
                TimeWindow::new(today - ChronoDuration::days(1), today)
            }
            Lookback::ThisMonth => {
                let start = local_month_start(at);
                TimeWindow::new(start, at)
            }
        };
    }
}

/// Local midnight at the start of the day containing `at`.
fn local_midnight(at: DateTime<Utc>) -> DateTime<Utc> {
    let local = at.with_timezone(&Local);

    // A DST transition can make midnight ambiguous or nonexistent. Falling back to the
    // instant itself keeps the window valid rather than panicking on two days a year.
    return local
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(at);
}

/// Local midnight on the first day of the month containing `at`.
fn local_month_start(at: DateTime<Utc>) -> DateTime<Utc> {
    let local = at.with_timezone(&Local);

    return local
        .date_naive()
        .with_day(1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(at);
}

/// Read access to what has already happened.
///
/// Implemented by whatever owns the event log. Kept to a single counting method so
/// the storage side stays free to answer it however it likes, and so this module never
/// needs to know what an event record looks like.
pub trait EventHistory {
    /// How many events of `kind` the given rule recorded within `window`.
    fn count(&self, rule: &str, kind: EventKind, window: TimeWindow) -> usize;

    /// When the rule last recorded an event of `kind`, if ever.
    ///
    /// Unbounded by design: window anchoring needs the last completion however long
    /// ago it was, and capping that would silently restart a maintenance schedule.
    fn last(&self, rule: &str, kind: EventKind) -> Option<DateTime<Utc>>;
}

/// A condition on a candidate instant, evaluated against past events.
///
/// A small typed tree rather than an expression language: it has to be serialisable,
/// inspectable in a UI, and impossible to make expensive by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Guard {
    /// No condition. The candidate always fires.
    Always,

    /// The named rule recorded this event within the lookback.
    Occurred {
        rule: String,
        kind: EventKind,
        within: Lookback,
    },

    /// The named rule recorded at least `count` of this event within the lookback.
    OccurredAtLeast {
        rule: String,
        kind: EventKind,
        within: Lookback,
        count: usize,
    },

    /// Every inner guard holds. An empty list holds.
    All(Vec<Guard>),

    /// At least one inner guard holds. An empty list does not hold.
    Any(Vec<Guard>),

    /// The inner guard does not hold.
    Not(Box<Guard>),
}

impl Guard {
    /// `Guard::Occurred` for the common case of "did this fire".
    pub fn fired(rule: impl Into<String>, within: Lookback) -> Self {
        return Guard::Occurred {
            rule: rule.into(),
            kind: EventKind::Fired,
            within,
        };
    }

    /// The negation of [`Guard::fired`].
    pub fn not_fired(rule: impl Into<String>, within: Lookback) -> Self {
        return Guard::Not(Box::new(Guard::fired(rule, within)));
    }

    /// `Guard::Occurred` for completion, which is what maintenance rules key on.
    pub fn completed(rule: impl Into<String>, within: Lookback) -> Self {
        return Guard::Occurred {
            rule: rule.into(),
            kind: EventKind::Completed,
            within,
        };
    }

    /// Whether this candidate should fire.
    ///
    /// `at` is the candidate instant, not the current time: a rule being evaluated for
    /// a moment that has already passed, during catch-up, must see the history as it
    /// was relevant to that moment.
    pub fn evaluate(&self, at: DateTime<Utc>, history: &dyn EventHistory) -> bool {
        return match self {
            Guard::Always => true,

            Guard::Occurred { rule, kind, within } => {
                history.count(rule, *kind, within.window(at)) > 0
            }

            Guard::OccurredAtLeast { rule, kind, within, count } => {
                history.count(rule, *kind, within.window(at)) >= *count
            }

            Guard::All(guards) => guards.iter().all(|g| g.evaluate(at, history)),

            Guard::Any(guards) => guards.iter().any(|g| g.evaluate(at, history)),

            Guard::Not(inner) => !inner.evaluate(at, history),
        };
    }

    /// Every rule id this guard depends on.
    ///
    /// Used to warn about mutual dependencies. They cannot loop, since guards only read
    /// the past, but a rule that depends on itself is almost always a mistake.
    pub fn referenced_rules(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect_rules(&mut out);
        out.sort_unstable();
        out.dedup();
        return out;
    }

    fn collect_rules<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Guard::Always => {}
            Guard::Occurred { rule, .. } | Guard::OccurredAtLeast { rule, .. } => {
                out.push(rule.as_str())
            }
            Guard::All(guards) | Guard::Any(guards) => {
                for guard in guards {
                    guard.collect_rules(out);
                }
            }
            Guard::Not(inner) => inner.collect_rules(out),
        }
    }

    /// How deeply nested this guard is, for rejecting pathological rules.
    pub fn depth(&self) -> usize {
        return match self {
            Guard::Always | Guard::Occurred { .. } | Guard::OccurredAtLeast { .. } => 1,
            Guard::All(guards) | Guard::Any(guards) => {
                1 + guards.iter().map(|g| g.depth()).max().unwrap_or(0)
            }
            Guard::Not(inner) => 1 + inner.depth(),
        };
    }
}

/// How urgent a window task currently is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgency {
    /// Before the window opens. Not shown at all.
    Dormant,
    /// Open but not yet due. Listed quietly.
    Upcoming,
    /// At the target. Notified normally.
    Due,
    /// Past the target, before the hard end. Persistent and intensifying.
    Overdue,
    /// Past the hard end. Prominent and repeating.
    Critical,
}

impl Urgency {
    pub fn as_str(self) -> &'static str {
        return match self {
            Urgency::Dormant => "dormant",
            Urgency::Upcoming => "upcoming",
            Urgency::Due => "due",
            Urgency::Overdue => "overdue",
            Urgency::Critical => "critical",
        };
    }

    /// Whether this state should be visible to the user at all.
    pub fn is_visible(self) -> bool {
        return self != Urgency::Dormant;
    }

    /// Whether this state warrants interrupting the user.
    pub fn should_notify(self) -> bool {
        return self >= Urgency::Due;
    }
}

impl std::fmt::Display for Urgency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.as_str());
    }
}

/// Where a window task stands right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowState {
    pub urgency: Urgency,
    /// How far through the window, from `0.0` at the soft start to `1.0` at the hard
    /// end, clamped outside that.
    ///
    /// Meant for interpolating a colour between palette roles, so escalation reads at
    /// a glance rather than only in the label.
    pub intensity: f32,
}

/// The shape of a window, as offsets from the moment it opens.
///
/// `soft_start` is the anchor itself, so offsets are measured from there: `target` is
/// when the work is due, and `hard_end` is when it becomes critical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowShape {
    /// How long after the anchor the task becomes visible.
    pub visible_after: ChronoDuration,
    /// How long after the anchor the task is due.
    pub due_after: ChronoDuration,
    /// How long after the anchor the task becomes critical.
    pub critical_after: ChronoDuration,
}

impl WindowShape {
    /// A window that opens partway through and goes critical after a grace period.
    ///
    /// The usual shape: quiet for most of the interval, visible near the end, due at
    /// the interval, critical once clearly overdue.
    ///
    /// ## Example
    /// ```
    /// # use luna::rules::WindowShape;
    /// # use chrono::Duration;
    /// // Oil change: due every 90 days, visible from day 60, critical after day 120.
    /// let shape = WindowShape::new(Duration::days(60), Duration::days(90), Duration::days(120));
    /// assert!(shape.is_ordered());
    /// ```
    pub fn new(
        visible_after: ChronoDuration,
        due_after: ChronoDuration,
        critical_after: ChronoDuration,
    ) -> Self {
        return Self { visible_after, due_after, critical_after };
    }

    /// A window derived from a single interval: visible at two thirds, due at the
    /// interval, critical at a third past it.
    pub fn from_interval(interval: ChronoDuration) -> Self {
        return Self {
            visible_after: interval * 2 / 3,
            due_after: interval,
            critical_after: interval + interval / 3,
        };
    }

    /// Whether the three thresholds are in a sensible order.
    pub fn is_ordered(&self) -> bool {
        return self.visible_after <= self.due_after && self.due_after <= self.critical_after;
    }

    /// The state of a window that opened at `anchor`, as of `now`.
    pub fn state_at(&self, anchor: DateTime<Utc>, now: DateTime<Utc>) -> WindowState {
        let visible = anchor + self.visible_after;
        let due = anchor + self.due_after;
        let critical = anchor + self.critical_after;

        let urgency = if now < visible {
            Urgency::Dormant
        } else if now < due {
            Urgency::Upcoming
        } else if now == due {
            Urgency::Due
        } else if now < critical {
            Urgency::Overdue
        } else {
            Urgency::Critical
        };

        // `Due` is a single instant above, which no poll would ever land on exactly.
        // Treat the moment of crossing the target as Due for as long as it takes the
        // caller to notice, by folding it into the Overdue branch's lower edge.
        let urgency = if urgency == Urgency::Overdue && now == due {
            Urgency::Due
        } else {
            urgency
        };

        let span = (critical - visible).num_seconds();
        let elapsed = (now - visible).num_seconds();

        let intensity = if span <= 0 {
            // A zero-width window is either dormant or already critical.
            if now < visible { 0.0 } else { 1.0 }
        } else {
            (elapsed as f32 / span as f32).clamp(0.0, 1.0)
        };

        return WindowState { urgency, intensity };
    }
}

/// What a window task measures its interval from.
#[derive(Debug, Clone, PartialEq)]
pub enum Anchor {
    /// The window restarts from when the work was last actually done.
    ///
    /// This is what makes maintenance schedules correct: an oil change done late
    /// should push the next one out, not keep a fixed calendar.
    RollingFromCompletion {
        /// Used only until the first completion is recorded.
        first_due: DateTime<Utc>,
    },

    /// The window opens at fixed calendar instants regardless of when it was done.
    ///
    /// Recurrence expansion is not implemented yet; this variant exists so the model
    /// is complete and callers can be written against it.
    FixedSchedule,
}

/// A task that should happen somewhere within a period, escalating as it elapses.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowTask {
    /// Identifies the task in the event log.
    pub rule: String,
    pub anchor: Anchor,
    pub shape: WindowShape,
}

impl WindowTask {
    /// When the current window opened.
    ///
    /// For a rolling anchor this is the last completion, or `first_due` if it has
    /// never been done.
    pub fn current_anchor(&self, history: &dyn EventHistory) -> Option<DateTime<Utc>> {
        return match &self.anchor {
            Anchor::RollingFromCompletion { first_due } => Some(
                history
                    .last(&self.rule, EventKind::Completed)
                    .unwrap_or(*first_due),
            ),
            Anchor::FixedSchedule => None,
        };
    }

    /// Where this task stands as of `now`.
    ///
    /// `None` when the anchor cannot be determined, which currently means a fixed
    /// schedule awaiting recurrence support.
    pub fn state_at(&self, now: DateTime<Utc>, history: &dyn EventHistory) -> Option<WindowState> {
        let anchor = self.current_anchor(history)?;
        return Some(self.shape.state_at(anchor, now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// An in-memory history, so guard behaviour can be tested at any instant.
    #[derive(Default)]
    struct Log {
        events: Vec<(String, EventKind, DateTime<Utc>)>,
    }

    impl Log {
        fn record(&mut self, rule: &str, kind: EventKind, at: DateTime<Utc>) -> &mut Self {
            self.events.push((rule.to_string(), kind, at));
            return self;
        }
    }

    impl EventHistory for Log {
        fn count(&self, rule: &str, kind: EventKind, window: TimeWindow) -> usize {
            return self
                .events
                .iter()
                .filter(|(r, k, at)| r == rule && *k == kind && window.contains(*at))
                .count();
        }

        fn last(&self, rule: &str, kind: EventKind) -> Option<DateTime<Utc>> {
            return self
                .events
                .iter()
                .filter(|(r, k, _)| r == rule && *k == kind)
                .map(|(_, _, at)| *at)
                .max();
        }
    }

    fn utc(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        return Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap();
    }

    #[test]
    fn always_fires() {
        let log = Log::default();
        assert!(Guard::Always.evaluate(utc(2026, 8, 14, 12), &log));
    }

    #[test]
    fn a_guard_sees_an_event_inside_its_lookback() {
        let mut log = Log::default();
        log.record("a", EventKind::Fired, utc(2026, 8, 14, 9));

        let guard = Guard::fired("a", Lookback::LastHours(6));

        assert!(guard.evaluate(utc(2026, 8, 14, 12), &log));
    }

    #[test]
    fn a_guard_does_not_see_an_event_outside_its_lookback() {
        let mut log = Log::default();
        log.record("a", EventKind::Fired, utc(2026, 8, 14, 1));

        let guard = Guard::fired("a", Lookback::LastHours(6));

        assert!(!guard.evaluate(utc(2026, 8, 14, 12), &log));
    }

    #[test]
    fn a_guard_ignores_other_rules_and_other_kinds() {
        let mut log = Log::default();
        log.record("other", EventKind::Fired, utc(2026, 8, 14, 11));
        log.record("a", EventKind::Snoozed, utc(2026, 8, 14, 11));

        let guard = Guard::fired("a", Lookback::LastHours(6));

        assert!(!guard.evaluate(utc(2026, 8, 14, 12), &log));
    }

    #[test]
    fn not_fired_is_the_negation() {
        let mut log = Log::default();
        let at = utc(2026, 8, 14, 12);

        let guard = Guard::not_fired("a", Lookback::LastHours(6));
        assert!(guard.evaluate(at, &log), "nothing recorded, so it did not fire");

        log.record("a", EventKind::Fired, utc(2026, 8, 14, 11));
        assert!(!guard.evaluate(at, &log));
    }

    #[test]
    fn count_thresholds_are_inclusive() {
        let mut log = Log::default();
        log.record("a", EventKind::Snoozed, utc(2026, 8, 14, 9));
        log.record("a", EventKind::Snoozed, utc(2026, 8, 14, 10));

        let guard = |count| Guard::OccurredAtLeast {
            rule: "a".to_string(),
            kind: EventKind::Snoozed,
            within: Lookback::LastHours(12),
            count,
        };

        let at = utc(2026, 8, 14, 12);

        assert!(guard(2).evaluate(at, &log));
        assert!(!guard(3).evaluate(at, &log));
    }

    #[test]
    fn all_and_any_combine_as_expected() {
        let mut log = Log::default();
        log.record("a", EventKind::Fired, utc(2026, 8, 14, 11));

        let at = utc(2026, 8, 14, 12);
        let a = Guard::fired("a", Lookback::LastHours(6));
        let b = Guard::fired("b", Lookback::LastHours(6));

        assert!(Guard::All(vec![a.clone()]).evaluate(at, &log));
        assert!(!Guard::All(vec![a.clone(), b.clone()]).evaluate(at, &log));
        assert!(Guard::Any(vec![a.clone(), b.clone()]).evaluate(at, &log));
        assert!(!Guard::Any(vec![b.clone()]).evaluate(at, &log));
    }

    #[test]
    fn empty_combinators_follow_the_usual_convention() {
        let log = Log::default();
        let at = utc(2026, 8, 14, 12);

        assert!(Guard::All(vec![]).evaluate(at, &log), "vacuously true");
        assert!(!Guard::Any(vec![]).evaluate(at, &log), "vacuously false");
    }

    #[test]
    fn the_motivating_case_works() {
        // "If reminder A fired yesterday, remind me today at 12am."
        let guard = Guard::fired("a", Lookback::Yesterday);

        // The candidate instant is local midnight today, which is what the daily
        // generator would produce.
        let candidate = local_midnight(utc(2026, 8, 14, 12));

        let mut log = Log::default();

        assert!(!guard.evaluate(candidate, &log), "nothing fired yet");

        // Fired late yesterday evening, local time. A plain 24h lookback from midnight
        // would be borderline; the calendar variant must catch it.
        log.record("a", EventKind::Fired, candidate - ChronoDuration::hours(1));

        assert!(guard.evaluate(candidate, &log));
    }

    #[test]
    fn yesterday_excludes_today() {
        let now = utc(2026, 8, 14, 12);
        let candidate = now;

        let mut log = Log::default();
        // Recorded after local midnight today.
        log.record("a", EventKind::Fired, local_midnight(now) + ChronoDuration::hours(1));

        assert!(
            !Guard::fired("a", Lookback::Yesterday).evaluate(candidate, &log),
            "an event today is not an event yesterday"
        );
        assert!(Guard::fired("a", Lookback::Today).evaluate(candidate, &log));
    }

    #[test]
    fn today_and_yesterday_do_not_overlap() {
        let at = utc(2026, 8, 14, 15);

        let today = Lookback::Today.window(at);
        let yesterday = Lookback::Yesterday.window(at);

        assert_eq!(yesterday.end, today.start, "the two must meet exactly once");
        assert_eq!(yesterday.duration(), ChronoDuration::days(1));
    }

    #[test]
    fn lookbacks_are_capped() {
        let at = utc(2026, 8, 14, 12);

        let absurd = Lookback::LastDays(100_000).window(at);

        assert_eq!(absurd.duration().num_days(), MAX_LOOKBACK_DAYS);
    }

    #[test]
    fn a_negative_lookback_is_empty_rather_than_reversed() {
        let at = utc(2026, 8, 14, 12);
        let window = Lookback::LastDays(-5).window(at);

        assert_eq!(window.duration(), ChronoDuration::zero());
        assert!(!window.contains(at - ChronoDuration::hours(1)));
    }

    #[test]
    fn a_reversed_window_is_normalised() {
        let a = utc(2026, 8, 14, 12);
        let b = utc(2026, 8, 14, 18);

        assert_eq!(TimeWindow::new(b, a), TimeWindow::new(a, b));
    }

    #[test]
    fn windows_are_half_open() {
        let start = utc(2026, 8, 14, 0);
        let end = utc(2026, 8, 15, 0);
        let window = TimeWindow::new(start, end);

        assert!(window.contains(start), "the start is included");
        assert!(!window.contains(end), "the end is not");
    }

    #[test]
    fn referenced_rules_are_collected_and_deduplicated() {
        let guard = Guard::All(vec![
            Guard::fired("a", Lookback::Today),
            Guard::Any(vec![
                Guard::fired("b", Lookback::Today),
                Guard::Not(Box::new(Guard::fired("a", Lookback::Yesterday))),
            ]),
        ]);

        assert_eq!(guard.referenced_rules(), vec!["a", "b"]);
    }

    #[test]
    fn depth_reflects_nesting() {
        assert_eq!(Guard::Always.depth(), 1);
        assert_eq!(Guard::Not(Box::new(Guard::Always)).depth(), 2);
        assert_eq!(
            Guard::All(vec![Guard::Not(Box::new(Guard::Always))]).depth(),
            3
        );
    }

    #[test]
    fn event_kinds_round_trip() {
        for kind in [
            EventKind::Fired,
            EventKind::Acknowledged,
            EventKind::Snoozed,
            EventKind::Completed,
            EventKind::Dismissed,
        ] {
            assert_eq!(kind.as_str().parse(), Ok(kind));
        }

        assert!("nonsense".parse::<EventKind>().is_err());
    }

    #[test]
    fn a_window_escalates_through_every_state() {
        let anchor = utc(2026, 1, 1, 0);
        let shape = WindowShape::new(
            ChronoDuration::days(60),
            ChronoDuration::days(90),
            ChronoDuration::days(120),
        );

        let urgency_at = |days: i64| shape.state_at(anchor, anchor + ChronoDuration::days(days)).urgency;

        assert_eq!(urgency_at(0), Urgency::Dormant);
        assert_eq!(urgency_at(59), Urgency::Dormant);
        assert_eq!(urgency_at(60), Urgency::Upcoming);
        assert_eq!(urgency_at(89), Urgency::Upcoming);
        assert_eq!(urgency_at(90), Urgency::Due);
        assert_eq!(urgency_at(91), Urgency::Overdue);
        assert_eq!(urgency_at(119), Urgency::Overdue);
        assert_eq!(urgency_at(120), Urgency::Critical);
        assert_eq!(urgency_at(365), Urgency::Critical);
    }

    #[test]
    fn intensity_runs_from_the_visible_point_to_critical() {
        let anchor = utc(2026, 1, 1, 0);
        let shape = WindowShape::new(
            ChronoDuration::days(60),
            ChronoDuration::days(90),
            ChronoDuration::days(120),
        );

        let intensity = |days: i64| shape.state_at(anchor, anchor + ChronoDuration::days(days)).intensity;

        assert_eq!(intensity(0), 0.0, "clamped before the window opens");
        assert_eq!(intensity(60), 0.0);
        assert!((intensity(90) - 0.5).abs() < 0.01, "halfway at the due date");
        assert_eq!(intensity(120), 1.0);
        assert_eq!(intensity(365), 1.0, "clamped after critical");
    }

    #[test]
    fn urgency_drives_visibility_and_notification() {
        assert!(!Urgency::Dormant.is_visible());
        assert!(Urgency::Upcoming.is_visible());

        assert!(!Urgency::Upcoming.should_notify(), "upcoming is listed, not announced");
        assert!(Urgency::Due.should_notify());
        assert!(Urgency::Critical.should_notify());
    }

    #[test]
    fn a_shape_from_an_interval_is_ordered() {
        let shape = WindowShape::from_interval(ChronoDuration::days(90));

        assert!(shape.is_ordered());
        assert_eq!(shape.due_after, ChronoDuration::days(90));
        assert!(shape.visible_after < shape.due_after);
        assert!(shape.critical_after > shape.due_after);
    }

    #[test]
    fn a_rolling_task_starts_from_its_first_due_date_until_it_is_done() {
        let first_due = utc(2026, 1, 1, 0);
        let task = WindowTask {
            rule: "oil".to_string(),
            anchor: Anchor::RollingFromCompletion { first_due },
            shape: WindowShape::from_interval(ChronoDuration::days(90)),
        };

        let log = Log::default();

        assert_eq!(task.current_anchor(&log), Some(first_due));
    }

    #[test]
    fn completing_a_rolling_task_pushes_the_next_window_out() {
        let first_due = utc(2026, 1, 1, 0);
        let task = WindowTask {
            rule: "oil".to_string(),
            anchor: Anchor::RollingFromCompletion { first_due },
            shape: WindowShape::new(
                ChronoDuration::days(60),
                ChronoDuration::days(90),
                ChronoDuration::days(120),
            ),
        };

        let mut log = Log::default();

        // Done late, on day 100.
        let done_at = first_due + ChronoDuration::days(100);
        log.record("oil", EventKind::Completed, done_at);

        assert_eq!(task.current_anchor(&log), Some(done_at));

        // The next window is measured from the completion, not from the calendar. Day
        // 130 overall is only day 30 of the new window, so it is dormant again.
        let state = task
            .state_at(first_due + ChronoDuration::days(130), &log)
            .unwrap();

        assert_eq!(
            state.urgency,
            Urgency::Dormant,
            "a late completion must push the next window out, not keep the old schedule"
        );
    }

    #[test]
    fn a_rolling_task_uses_the_most_recent_completion() {
        let first_due = utc(2026, 1, 1, 0);
        let task = WindowTask {
            rule: "oil".to_string(),
            anchor: Anchor::RollingFromCompletion { first_due },
            shape: WindowShape::from_interval(ChronoDuration::days(90)),
        };

        let mut log = Log::default();
        let older = first_due + ChronoDuration::days(90);
        let newer = first_due + ChronoDuration::days(200);

        log.record("oil", EventKind::Completed, older);
        log.record("oil", EventKind::Completed, newer);

        assert_eq!(task.current_anchor(&log), Some(newer));
    }

    #[test]
    fn a_fixed_schedule_has_no_state_until_recurrence_lands() {
        let task = WindowTask {
            rule: "monthly".to_string(),
            anchor: Anchor::FixedSchedule,
            shape: WindowShape::from_interval(ChronoDuration::days(30)),
        };

        let log = Log::default();

        assert_eq!(task.current_anchor(&log), None);
        assert_eq!(task.state_at(utc(2026, 8, 14, 12), &log), None);
    }

    #[test]
    fn a_degenerate_window_does_not_divide_by_zero() {
        let anchor = utc(2026, 1, 1, 0);
        let shape = WindowShape::new(
            ChronoDuration::zero(),
            ChronoDuration::zero(),
            ChronoDuration::zero(),
        );

        let before = shape.state_at(anchor, anchor - ChronoDuration::hours(1));
        let after = shape.state_at(anchor, anchor + ChronoDuration::hours(1));

        assert_eq!(before.intensity, 0.0);
        assert_eq!(after.intensity, 1.0);
        assert_eq!(after.urgency, Urgency::Critical);
    }

    #[test]
    fn guards_only_read_the_past_so_mutual_references_terminate() {
        // A guards on B, B guards on A. Evaluating either must simply answer, because
        // each only ever consults events that already happened.
        let mut log = Log::default();
        log.record("a", EventKind::Fired, utc(2026, 8, 13, 12));

        let a_on_b = Guard::fired("b", Lookback::Yesterday);
        let b_on_a = Guard::fired("a", Lookback::Yesterday);

        let at = utc(2026, 8, 14, 12);

        assert!(!a_on_b.evaluate(at, &log));
        assert!(b_on_a.evaluate(at, &log));

        // And the dependency is discoverable, so a UI can warn about the pairing.
        let mut refs: HashMap<&str, Vec<&str>> = HashMap::new();
        refs.insert("a", a_on_b.referenced_rules());
        refs.insert("b", b_on_a.referenced_rules());

        assert_eq!(refs["a"], vec!["b"]);
        assert_eq!(refs["b"], vec!["a"]);
    }
}
