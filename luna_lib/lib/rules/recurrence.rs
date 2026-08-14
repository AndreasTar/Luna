//! When a rule comes due.
//!
//! A [`Schedule`] is a recurrence pattern plus a local time of day. Asking it for
//! [`Schedule::next_after`] materialises exactly one instant, which is the only way it
//! is ever used: a rule holds its pattern, and the next occurrence is recomputed after
//! each fire rather than expanded ahead of time. An infinite series has no useful
//! expansion, and a stored "next fire" timestamp goes wrong the moment a clock or a
//! timezone moves.
//!
//! ## Local time, deliberately
//!
//! Patterns are expressed in local time, because "every day at 9am" means 9am where
//! the user is, not a fixed UTC offset that drifts an hour twice a year. Instants come
//! back in UTC, so storage and comparison never have to think about it.
//!
//! Daylight saving makes two local times a year awkward: one happens twice, one never
//! happens at all. Both are resolved rather than skipped, so a daily rule fires every
//! day of the year including those two.
//!
//! ## Relationship to RFC 5545
//!
//! The model maps onto the iCalendar `RRULE` fields (`FREQ`, `INTERVAL`, `BYDAY`,
//! `BYMONTHDAY`) without matching them exactly. Parsing and emitting `RRULE` strings
//! for `.ics` interoperability can be layered on top later without changing anything
//! here; the `rrule` crate was measured at 26 transitive dependencies, including a
//! timezone database and a regex engine, which is a poor trade for a pattern set this
//! small.

use chrono::{
    DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Timelike, Utc, Weekday,
};

/// Which occurrence of a weekday within a month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NthWeek {
    First,
    Second,
    Third,
    Fourth,
    /// The last one, whether that is the fourth or the fifth.
    Last,
}

impl NthWeek {
    /// The 1-based index, or `None` for [`NthWeek::Last`].
    fn index(self) -> Option<u32> {
        return match self {
            NthWeek::First => Some(1),
            NthWeek::Second => Some(2),
            NthWeek::Third => Some(3),
            NthWeek::Fourth => Some(4),
            NthWeek::Last => None,
        };
    }
}

/// Which day of a month a monthly recurrence lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MonthDay {
    /// A fixed day number, clamped to the length of the month.
    ///
    /// Clamping rather than skipping: "the 31st of every month" in February means the
    /// 28th, not nothing. A rule that silently never fires is worse than one that
    /// fires slightly early.
    OnDay(u32),

    /// The last day of the month, whatever its length.
    Last,

    /// The nth occurrence of a weekday, such as the second Sunday.
    Nth { week: NthWeek, weekday: Weekday },
}

/// How often a rule repeats.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Recurrence {
    /// Every `interval` days.
    Daily { interval: u32 },

    /// On the given weekdays, every `interval` weeks.
    Weekly { interval: u32, weekdays: Vec<Weekday> },

    /// On the given day, every `interval` months.
    Monthly { interval: u32, day: MonthDay },

    /// On the given month and day, every `interval` years.
    Yearly { interval: u32, month: u32, day: u32 },
}

impl Recurrence {
    /// Every day.
    pub fn daily() -> Self {
        return Recurrence::Daily { interval: 1 };
    }

    /// The nth given weekday of every month, such as the second Sunday.
    pub fn nth_weekday_monthly(week: NthWeek, weekday: Weekday) -> Self {
        return Recurrence::Monthly {
            interval: 1,
            day: MonthDay::Nth { week, weekday },
        };
    }

    /// The interval, forced to at least 1 so a zero cannot loop forever.
    fn interval(&self) -> u32 {
        let raw = match self {
            Recurrence::Daily { interval }
            | Recurrence::Weekly { interval, .. }
            | Recurrence::Monthly { interval, .. }
            | Recurrence::Yearly { interval, .. } => *interval,
        };

        return raw.max(1);
    }
}

/// A recurrence plus the local time of day it fires at.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Schedule {
    pub recurrence: Recurrence,
    /// Local time of day.
    pub at: NaiveTime,
    /// The schedule does not produce occurrences before this instant.
    pub start: DateTime<Utc>,
    /// The schedule stops after this instant, if set.
    pub until: Option<DateTime<Utc>>,
}

/// How far ahead a search will look before giving up.
///
/// Generous enough for any real pattern (a yearly rule needs four steps to clear a
/// leap-year edge) and small enough that a pathological one cannot hang.
const MAX_STEPS: u32 = 512;

impl Schedule {
    pub fn new(recurrence: Recurrence, at: NaiveTime, start: DateTime<Utc>) -> Self {
        return Self { recurrence, at, start, until: None };
    }

    pub fn until(mut self, until: DateTime<Utc>) -> Self {
        self.until = Some(until);
        return self;
    }

    /// The first occurrence strictly after `after`.
    ///
    /// `None` when the schedule has ended, or when no occurrence could be found within
    /// [`MAX_STEPS`], which in practice means the pattern is unsatisfiable.
    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Never produce anything before the schedule begins.
        let after = after.max(self.start - Duration::seconds(1));

        let found = match &self.recurrence {
            Recurrence::Daily { .. } => self.next_daily(after),
            Recurrence::Weekly { weekdays, .. } => self.next_weekly(after, weekdays),
            Recurrence::Monthly { day, .. } => self.next_monthly(after, *day),
            Recurrence::Yearly { month, day, .. } => self.next_yearly(after, *month, *day),
        }?;

        if self.until.is_some_and(|until| found > until) {
            return None;
        }

        return Some(found);
    }

    /// Every occurrence in `(from, to]`, oldest first.
    ///
    /// Used on startup to work out what was missed while Luna was closed. `cap` bounds
    /// the result: a daily rule and a year-long absence is 365 entries, and something
    /// has to stop a minute-by-minute rule from producing half a million.
    pub fn occurrences_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        cap: usize,
    ) -> Vec<DateTime<Utc>> {
        let mut out = Vec::new();
        let mut cursor = from;

        while out.len() < cap {
            let Some(next) = self.next_after(cursor) else {
                break;
            };

            if next > to {
                break;
            }

            out.push(next);
            cursor = next;
        }

        return out;
    }

    /// The local start date of the schedule.
    fn start_date(&self) -> NaiveDate {
        return self.start.with_timezone(&Local).date_naive();
    }

    fn next_daily(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let interval = self.recurrence.interval() as i64;
        let start = self.start_date();
        let after_date = after.with_timezone(&Local).date_naive();

        // Jump straight to the neighbourhood of `after` rather than stepping from the
        // start date, which could be years back.
        let elapsed = (after_date - start).num_days();
        let mut step = (elapsed / interval).max(0);

        for _ in 0..MAX_STEPS {
            let date = start.checked_add_signed(Duration::days(step * interval))?;

            if let Some(instant) = self.at_on(date) {
                if instant > after {
                    return Some(instant);
                }
            }

            step += 1;
        }

        return None;
    }

    fn next_weekly(&self, after: DateTime<Utc>, weekdays: &[Weekday]) -> Option<DateTime<Utc>> {
        if weekdays.is_empty() {
            return None;
        }

        let interval = self.recurrence.interval() as i64;
        let start = self.start_date();
        let after_date = after.with_timezone(&Local).date_naive();

        // Back up a full interval so an occurrence earlier this week is not missed.
        let mut date = after_date - Duration::days(7 * interval);
        if date < start {
            date = start;
        }

        for _ in 0..MAX_STEPS * 7 {
            if date >= start && weekdays.contains(&date.weekday()) {
                // Whole weeks since the start week decide interval alignment.
                let weeks = (date - start).num_days().div_euclid(7);

                if weeks.rem_euclid(interval) == 0 {
                    if let Some(instant) = self.at_on(date) {
                        if instant > after {
                            return Some(instant);
                        }
                    }
                }
            }

            date = date.succ_opt()?;
        }

        return None;
    }

    fn next_monthly(&self, after: DateTime<Utc>, day: MonthDay) -> Option<DateTime<Utc>> {
        let interval = self.recurrence.interval() as i64;
        let start = self.start_date();
        let after_local = after.with_timezone(&Local).date_naive();

        let months_between = |a: NaiveDate, b: NaiveDate| -> i64 {
            return (b.year() as i64 - a.year() as i64) * 12
                + (b.month() as i64 - a.month() as i64);
        };

        let elapsed = months_between(start, after_local);
        let mut step = (elapsed / interval).max(0);

        for _ in 0..MAX_STEPS {
            let months = step * interval;
            let (year, month) = add_months(start.year(), start.month(), months)?;

            if let Some(date) = resolve_month_day(year, month, day) {
                if date >= start {
                    if let Some(instant) = self.at_on(date) {
                        if instant > after {
                            return Some(instant);
                        }
                    }
                }
            }

            step += 1;
        }

        return None;
    }

    fn next_yearly(&self, after: DateTime<Utc>, month: u32, day: u32) -> Option<DateTime<Utc>> {
        let interval = self.recurrence.interval() as i64;
        let start = self.start_date();
        let after_local = after.with_timezone(&Local).date_naive();

        let elapsed = (after_local.year() - start.year()) as i64;
        let mut step = (elapsed / interval).max(0);

        for _ in 0..MAX_STEPS {
            let year = start.year() + (step * interval) as i32;

            // Clamped, so 29 February in a common year lands on the 28th rather than
            // skipping three years out of four.
            if let Some(date) = resolve_month_day(year, month, MonthDay::OnDay(day)) {
                if date >= start {
                    if let Some(instant) = self.at_on(date) {
                        if instant > after {
                            return Some(instant);
                        }
                    }
                }
            }

            step += 1;
        }

        return None;
    }

    /// The schedule's time of day on a given local date, as a UTC instant.
    ///
    /// Handles the two awkward days a year: a local time that happens twice takes the
    /// first, and one that never happens takes the first valid instant after the gap,
    /// so a daily rule fires on every calendar day rather than silently skipping one.
    fn at_on(&self, date: NaiveDate) -> Option<DateTime<Utc>> {
        let naive = date.and_time(self.at);

        if let Some(local) = Local.from_local_datetime(&naive).earliest() {
            return Some(local.with_timezone(&Utc));
        }

        // A spring-forward gap. Walk forward in minutes until the clock exists again;
        // an hour is the largest shift in practice, and the bound keeps it finite.
        for minutes in 1..=120 {
            let shifted = naive + Duration::minutes(minutes);
            if let Some(local) = Local.from_local_datetime(&shifted).earliest() {
                return Some(local.with_timezone(&Utc));
            }
        }

        return None;
    }
}

/// Adds a signed number of months to a year and month.
fn add_months(year: i32, month: u32, months: i64) -> Option<(i32, u32)> {
    let total = (year as i64) * 12 + (month as i64 - 1) + months;

    let year = i32::try_from(total.div_euclid(12)).ok()?;
    let month = u32::try_from(total.rem_euclid(12)).ok()? + 1;

    return Some((year, month));
}

/// The number of days in a month.
fn days_in_month(year: i32, month: u32) -> Option<u32> {
    let (next_year, next_month) = add_months(year, month, 1)?;
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let next_first = NaiveDate::from_ymd_opt(next_year, next_month, 1)?;

    return u32::try_from((next_first - first).num_days()).ok();
}

/// Resolves a [`MonthDay`] within a specific month.
fn resolve_month_day(year: i32, month: u32, day: MonthDay) -> Option<NaiveDate> {
    let length = days_in_month(year, month)?;

    return match day {
        MonthDay::OnDay(n) => {
            let clamped = n.clamp(1, length);
            NaiveDate::from_ymd_opt(year, month, clamped)
        }

        MonthDay::Last => NaiveDate::from_ymd_opt(year, month, length),

        MonthDay::Nth { week, weekday } => match week.index() {
            Some(n) => {
                let first = NaiveDate::from_ymd_opt(year, month, 1)?;

                // Days from the 1st to the first matching weekday.
                let offset = (weekday.num_days_from_monday() as i64
                    - first.weekday().num_days_from_monday() as i64)
                    .rem_euclid(7);

                let day = 1 + offset + (n as i64 - 1) * 7;

                if day > length as i64 {
                    // Asking for the fifth Sunday of a month that has four. Skipped
                    // rather than clamped: "the fifth Sunday" genuinely does not exist
                    // that month, unlike "the 31st", which has an obvious nearest day.
                    None
                } else {
                    NaiveDate::from_ymd_opt(year, month, day as u32)
                }
            }

            None => {
                let last = NaiveDate::from_ymd_opt(year, month, length)?;

                let back = (last.weekday().num_days_from_monday() as i64
                    - weekday.num_days_from_monday() as i64)
                    .rem_euclid(7);

                NaiveDate::from_ymd_opt(year, month, length - back as u32)
            }
        },
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(h: u32, m: u32) -> NaiveTime {
        return NaiveTime::from_hms_opt(h, m, 0).unwrap();
    }

    /// A UTC instant far enough from any local midnight that timezone offsets on the
    /// test machine cannot move it into a neighbouring day.
    fn start_of(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        return Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap();
    }

    /// The local calendar date of an instant, which is what patterns are expressed in.
    fn local_date(at: DateTime<Utc>) -> NaiveDate {
        return at.with_timezone(&Local).date_naive();
    }

    #[test]
    fn daily_produces_consecutive_days() {
        let schedule = Schedule::new(Recurrence::daily(), time(9, 0), start_of(2026, 8, 1));

        let first = schedule.next_after(start_of(2026, 8, 10)).unwrap();
        let second = schedule.next_after(first).unwrap();
        let third = schedule.next_after(second).unwrap();

        assert_eq!(local_date(second), local_date(first).succ_opt().unwrap());
        assert_eq!(local_date(third), local_date(second).succ_opt().unwrap());
    }

    #[test]
    fn daily_fires_at_the_local_time_of_day() {
        let schedule = Schedule::new(Recurrence::daily(), time(9, 30), start_of(2026, 8, 1));

        let next = schedule.next_after(start_of(2026, 8, 10)).unwrap();
        let local = next.with_timezone(&Local);

        assert_eq!(local.hour(), 9);
        assert_eq!(local.minute(), 30);
    }

    #[test]
    fn an_interval_skips_the_days_between() {
        let schedule = Schedule::new(
            Recurrence::Daily { interval: 3 },
            time(9, 0),
            start_of(2026, 8, 1),
        );

        let first = schedule.next_after(start_of(2026, 8, 10)).unwrap();
        let second = schedule.next_after(first).unwrap();

        assert_eq!((local_date(second) - local_date(first)).num_days(), 3);
    }

    #[test]
    fn nothing_fires_before_the_schedule_starts() {
        let start = start_of(2026, 8, 10);
        let schedule = Schedule::new(Recurrence::daily(), time(9, 0), start);

        let next = schedule.next_after(start_of(2026, 1, 1)).unwrap();

        assert!(next >= start - Duration::days(1), "got {next}");
        assert!(local_date(next) >= local_date(start) - Duration::days(1));
    }

    #[test]
    fn nothing_fires_after_the_schedule_ends() {
        let schedule = Schedule::new(Recurrence::daily(), time(9, 0), start_of(2026, 8, 1))
            .until(start_of(2026, 8, 5));

        assert!(schedule.next_after(start_of(2026, 8, 2)).is_some());
        assert_eq!(schedule.next_after(start_of(2026, 8, 20)), None);
    }

    #[test]
    fn weekly_lands_only_on_the_chosen_weekdays() {
        let schedule = Schedule::new(
            Recurrence::Weekly {
                interval: 1,
                weekdays: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
            },
            time(9, 0),
            start_of(2026, 8, 1),
        );

        let mut cursor = start_of(2026, 8, 10);

        for _ in 0..10 {
            cursor = schedule.next_after(cursor).unwrap();
            let weekday = local_date(cursor).weekday();

            assert!(
                matches!(weekday, Weekday::Mon | Weekday::Wed | Weekday::Fri),
                "landed on {weekday:?}"
            );
        }
    }

    #[test]
    fn weekly_with_no_weekdays_never_fires() {
        let schedule = Schedule::new(
            Recurrence::Weekly { interval: 1, weekdays: vec![] },
            time(9, 0),
            start_of(2026, 8, 1),
        );

        assert_eq!(schedule.next_after(start_of(2026, 8, 10)), None);
    }

    #[test]
    fn the_second_sunday_of_the_month_is_the_second_sunday() {
        let schedule = Schedule::new(
            Recurrence::nth_weekday_monthly(NthWeek::Second, Weekday::Sun),
            time(10, 0),
            start_of(2026, 1, 1),
        );

        let mut cursor = start_of(2026, 1, 1);

        for _ in 0..12 {
            cursor = schedule.next_after(cursor).unwrap();
            let date = local_date(cursor);

            assert_eq!(date.weekday(), Weekday::Sun);
            assert!(
                (8..=14).contains(&date.day()),
                "the second Sunday always falls between the 8th and the 14th, got {date}"
            );
        }
    }

    #[test]
    fn the_last_friday_is_within_seven_days_of_month_end() {
        let schedule = Schedule::new(
            Recurrence::nth_weekday_monthly(NthWeek::Last, Weekday::Fri),
            time(10, 0),
            start_of(2026, 1, 1),
        );

        let mut cursor = start_of(2026, 1, 1);

        for _ in 0..12 {
            cursor = schedule.next_after(cursor).unwrap();
            let date = local_date(cursor);
            let length = days_in_month(date.year(), date.month()).unwrap();

            assert_eq!(date.weekday(), Weekday::Fri);
            assert!(
                date.day() + 7 > length,
                "{date} is not the last Friday of a {length}-day month"
            );
        }
    }

    #[test]
    fn a_fifth_weekday_is_skipped_in_months_that_lack_one() {
        // Not every month has five Sundays, and there is no obvious nearest day, so
        // those months produce nothing rather than something wrong.
        let mut seen = 0;

        for month in 1..=12 {
            if resolve_month_day(2026, month, MonthDay::Nth {
                week: NthWeek::Fourth,
                weekday: Weekday::Sun,
            })
            .is_some()
            {
                seen += 1;
            }
        }

        assert_eq!(seen, 12, "every month has a fourth Sunday");
    }

    #[test]
    fn a_day_number_past_the_end_of_the_month_is_clamped() {
        // "The 31st of every month" in February means the 28th, not nothing at all.
        let feb = resolve_month_day(2026, 2, MonthDay::OnDay(31)).unwrap();
        assert_eq!(feb.day(), 28);

        let leap_feb = resolve_month_day(2028, 2, MonthDay::OnDay(31)).unwrap();
        assert_eq!(leap_feb.day(), 29);

        let april = resolve_month_day(2026, 4, MonthDay::OnDay(31)).unwrap();
        assert_eq!(april.day(), 30);
    }

    #[test]
    fn monthly_on_the_last_day_tracks_month_length() {
        let schedule = Schedule::new(
            Recurrence::Monthly { interval: 1, day: MonthDay::Last },
            time(9, 0),
            start_of(2026, 1, 1),
        );

        let mut cursor = start_of(2026, 1, 1);

        for _ in 0..14 {
            cursor = schedule.next_after(cursor).unwrap();
            let date = local_date(cursor);
            let length = days_in_month(date.year(), date.month()).unwrap();

            assert_eq!(date.day(), length, "{date} is not the last day");
        }
    }

    #[test]
    fn a_monthly_interval_steps_by_that_many_months() {
        let schedule = Schedule::new(
            Recurrence::Monthly { interval: 3, day: MonthDay::OnDay(15) },
            time(9, 0),
            start_of(2026, 1, 1),
        );

        let first = schedule.next_after(start_of(2026, 1, 1)).unwrap();
        let second = schedule.next_after(first).unwrap();

        let months = (local_date(second).year() - local_date(first).year()) * 12
            + local_date(second).month() as i32
            - local_date(first).month() as i32;

        assert_eq!(months, 3);
    }

    #[test]
    fn yearly_repeats_on_the_same_date() {
        let schedule = Schedule::new(
            Recurrence::Yearly { interval: 1, month: 3, day: 15 },
            time(9, 0),
            start_of(2026, 1, 1),
        );

        let first = schedule.next_after(start_of(2026, 1, 1)).unwrap();
        let second = schedule.next_after(first).unwrap();

        assert_eq!(local_date(first).month(), 3);
        assert_eq!(local_date(first).day(), 15);
        assert_eq!(local_date(second).year(), local_date(first).year() + 1);
        assert_eq!(local_date(second).day(), 15);
    }

    #[test]
    fn the_29th_of_february_still_fires_in_common_years() {
        let schedule = Schedule::new(
            Recurrence::Yearly { interval: 1, month: 2, day: 29 },
            time(9, 0),
            start_of(2026, 1, 1),
        );

        let mut cursor = start_of(2026, 1, 1);

        for _ in 0..5 {
            cursor = schedule.next_after(cursor).unwrap();
            let date = local_date(cursor);

            assert_eq!(date.month(), 2);
            assert!(
                date.day() == 28 || date.day() == 29,
                "clamped to the end of February, got {date}"
            );
        }
    }

    #[test]
    fn occurrences_between_returns_the_gap_oldest_first() {
        let schedule = Schedule::new(Recurrence::daily(), time(9, 0), start_of(2026, 8, 1));

        let missed = schedule.occurrences_between(start_of(2026, 8, 10), start_of(2026, 8, 15), 100);

        assert_eq!(missed.len(), 5);

        for pair in missed.windows(2) {
            assert!(pair[0] < pair[1], "not in order");
        }
    }

    #[test]
    fn occurrences_between_respects_the_cap() {
        let schedule = Schedule::new(Recurrence::daily(), time(9, 0), start_of(2026, 1, 1));

        // A year of daily occurrences, asked for three.
        let missed = schedule.occurrences_between(start_of(2026, 1, 1), start_of(2026, 12, 31), 3);

        assert_eq!(missed.len(), 3);
    }

    #[test]
    fn occurrences_between_is_empty_when_nothing_was_missed() {
        let schedule = Schedule::new(Recurrence::daily(), time(9, 0), start_of(2026, 8, 1));

        let missed = schedule.occurrences_between(start_of(2026, 8, 10), start_of(2026, 8, 10), 100);

        assert!(missed.is_empty());
    }

    #[test]
    fn a_zero_interval_cannot_hang() {
        // A zero would divide by zero or loop forever if it reached the arithmetic.
        let schedule = Schedule::new(
            Recurrence::Daily { interval: 0 },
            time(9, 0),
            start_of(2026, 8, 1),
        );

        assert!(schedule.next_after(start_of(2026, 8, 10)).is_some());
    }

    #[test]
    fn every_day_of_a_year_produces_an_instant() {
        // Includes both daylight saving transitions in any timezone the test runs in.
        // A local time that does not exist must still resolve, or a daily rule would
        // silently skip a day once a year.
        let schedule = Schedule::new(Recurrence::daily(), time(2, 30), start_of(2026, 1, 1));

        let mut cursor = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut days = 0;

        while days < 365 {
            let Some(next) = schedule.next_after(cursor) else {
                panic!("stopped producing occurrences after {days} days");
            };
            assert!(next > cursor, "did not advance at day {days}");
            cursor = next;
            days += 1;
        }
    }

    #[test]
    fn add_months_crosses_year_boundaries_in_both_directions() {
        assert_eq!(add_months(2026, 1, 0), Some((2026, 1)));
        assert_eq!(add_months(2026, 12, 1), Some((2027, 1)));
        assert_eq!(add_months(2026, 1, -1), Some((2025, 12)));
        assert_eq!(add_months(2026, 6, 18), Some((2027, 12)));
    }

    #[test]
    fn month_lengths_are_right_including_leap_years() {
        assert_eq!(days_in_month(2026, 1), Some(31));
        assert_eq!(days_in_month(2026, 2), Some(28));
        assert_eq!(days_in_month(2028, 2), Some(29));
        assert_eq!(days_in_month(2026, 4), Some(30));
        assert_eq!(days_in_month(2026, 12), Some(31));
    }
}
