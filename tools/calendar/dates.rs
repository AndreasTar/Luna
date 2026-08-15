//! Calendar arithmetic.
//!
//! Every date question the calendar asks is answered here, in Rust, and the answers are
//! handed to Slint as prepared arrays. Slint never computes a date.
//!
//! That is a deliberate split rather than tidiness. Slint's expression language has no
//! date type, so the page used to work months out with a chain of ternaries: it got
//! February right only because February fell through the default branch, and it laid
//! every month out starting on a Monday because there was nowhere to put the weekday
//! offset. Leap years, month lengths, weekday offsets and daylight saving are exactly
//! what `chrono` exists for, and the page is left doing what it is good at: drawing
//! cells and reporting which one was clicked.
//!
//! Dates here are *local* dates, the ones on the wall. Instants are UTC. The conversion
//! between them happens in [`day_bounds`] and nowhere else.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};

/// Cells in a month grid: six weeks of seven days.
///
/// Fixed rather than fitted to the month so the grid never changes height as the user
/// steps through the year, which would make the whole page jump. Six weeks is the most
/// any month can touch: 31 days starting on a Sunday spans 6 weeks when weeks start on
/// a Monday.
pub const MONTH_CELLS: usize = 42;

/// One cell of a month grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridDay {
    pub date: NaiveDate,
    /// False for the days either side that belong to the neighbouring months.
    pub in_month: bool,
}

/// The six-week grid containing `anchor`'s month, running Monday to Sunday.
///
/// Monday first because the page's header row reads M T W T F S S.
pub fn month_grid(anchor: NaiveDate) -> Vec<GridDay> {
    // Cannot fail: day 1 exists in every month.
    let first = anchor.with_day(1).unwrap_or(anchor);
    let offset = first.weekday().num_days_from_monday() as i64;
    let start = first - Duration::days(offset);

    return (0..MONTH_CELLS)
        .map(|i| {
            let date = start + Duration::days(i as i64);

            return GridDay {
                date,
                in_month: date.month() == anchor.month() && date.year() == anchor.year(),
            };
        })
        .collect();
}

/// The last day of `anchor`'s month.
pub fn last_day_of_month(year: i32, month: u32) -> u32 {
    // The first of next month, minus a day. Cheaper to be right this way than to write
    // out the lengths and the leap year rule again.
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    let first_of_next = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, month, 28).unwrap_or_default());

    return (first_of_next - Duration::days(1)).day();
}

/// Steps by whole months, keeping the day of the month where it can.
///
/// The 31st of January stepped forward a month is the 28th of February, not the 3rd of
/// March. Clamping is what a person means by "next month" when they are looking at a
/// grid.
pub fn add_months(anchor: NaiveDate, delta: i32) -> NaiveDate {
    let months = anchor.year() * 12 + (anchor.month() as i32 - 1) + delta;
    let year = months.div_euclid(12);
    let month = months.rem_euclid(12) as u32 + 1;
    let day = anchor.day().min(last_day_of_month(year, month));

    return NaiveDate::from_ymd_opt(year, month, day).unwrap_or(anchor);
}

/// Steps by whole years, clamping the 29th of February onto the 28th.
pub fn add_years(anchor: NaiveDate, delta: i32) -> NaiveDate {
    return add_months(anchor, delta * 12);
}

/// The instant a local day begins.
///
/// Local midnight does not always exist: where daylight saving springs forward at
/// midnight the day starts at 01:00, and asking for 00:00 gives nothing back. Rather
/// than unwrap into a panic on one day a year in one set of timezones, this walks
/// forward in quarter hours until it finds the first instant that day actually has.
fn local_start_of_day(day: NaiveDate) -> DateTime<Utc> {
    for quarter in 0..(4 * 24) {
        let Some(time) = NaiveTime::from_hms_opt(quarter / 4, (quarter % 4) * 15, 0) else {
            continue;
        };

        let naive = day.and_time(time);

        if let Some(local) = Local.from_local_datetime(&naive).earliest() {
            return local.with_timezone(&Utc);
        }
    }

    // A day with no valid local instant in it at all is not a thing that happens, but
    // returning something sane beats an unwrap that can never be reasoned about.
    return Utc
        .from_utc_datetime(&day.and_time(NaiveTime::MIN));
}

/// The half-open span `[start, end)` of a local day, as UTC instants.
///
/// Half-open so an event starting exactly at midnight belongs to one day rather than
/// two, and so consecutive days tile without a gap or an overlap.
pub fn day_bounds(day: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
    return (
        local_start_of_day(day),
        local_start_of_day(day + Duration::days(1)),
    );
}

/// The span covering the whole grid a month is drawn in, neighbouring days included.
///
/// The month view marks which days carry events, and that includes the greyed-out days
/// either side, so the query has to cover the grid rather than the month.
pub fn grid_bounds(anchor: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
    let grid = month_grid(anchor);
    let first = grid.first().map(|d| d.date).unwrap_or(anchor);
    let last = grid.last().map(|d| d.date).unwrap_or(anchor);

    return (local_start_of_day(first), local_start_of_day(last + Duration::days(1)));
}

/// The local date an instant falls on.
pub fn local_date_of(at: DateTime<Utc>) -> NaiveDate {
    return at.with_timezone(&Local).date_naive();
}

/// The hour of the local day an instant falls in, 0 to 23.
pub fn local_hour_of(at: DateTime<Utc>) -> u32 {
    return at.with_timezone(&Local).hour();
}

/// Parses `YYYY-MM-DD`, the form the editor's date field holds.
///
/// Strict rather than forgiving: a field that silently accepted "2026-13-45" and landed
/// somewhere else entirely is worse than one that refuses and keeps the dialog open.
pub fn parse_day(text: &str) -> Option<NaiveDate> {
    return NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok();
}

/// Parses `HH:MM`, the form the editor's time fields hold.
///
/// A bare hour is accepted too, since typing 9 for nine o'clock is the obvious thing to
/// do and refusing it would be pedantry. It is handled here rather than by a second
/// format string because chrono will not build a time from an hour alone: with no minute
/// in the input there is nothing to construct one from.
pub fn parse_time(text: &str) -> Option<NaiveTime> {
    let text = text.trim();

    if let Ok(time) = NaiveTime::parse_from_str(text, "%H:%M") {
        return Some(time);
    }

    let hour: u32 = text.parse().ok()?;

    return NaiveTime::from_hms_opt(hour, 0, 0);
}

/// `YYYY-MM-DD`.
pub fn format_day(day: NaiveDate) -> String {
    return day.format("%Y-%m-%d").to_string();
}

/// `HH:MM`.
pub fn format_time(time: NaiveTime) -> String {
    return time.format("%H:%M").to_string();
}

/// A local date and time as a UTC instant.
///
/// Goes through the same spring-forward walk as [`day_bounds`], so an entry written at a
/// clock time that does not exist lands on the first one that does rather than vanishing.
pub fn local_instant(day: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    let naive = day.and_time(time);

    if let Some(local) = Local.from_local_datetime(&naive).earliest() {
        return local.with_timezone(&Utc);
    }

    for minutes in 1..=120 {
        let shifted = naive + Duration::minutes(minutes);

        if let Some(local) = Local.from_local_datetime(&shifted).earliest() {
            return local.with_timezone(&Utc);
        }
    }

    return local_start_of_day(day);
}

/// Where an instant falls within a local day, in hours, as a fraction.
///
/// 09:30 is 9.5. Measured from `day_start` rather than from the instant's own day, so an
/// entry that began yesterday reads as negative and one running past midnight reads past
/// 24, which is what the day column needs to draw it in the right place.
pub fn hours_from(day_start: DateTime<Utc>, at: DateTime<Utc>) -> f32 {
    return (at - day_start).num_minutes() as f32 / 60.0;
}

pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// "August 2026".
pub fn month_label(anchor: NaiveDate) -> String {
    let name = MONTH_NAMES
        .get(anchor.month0() as usize)
        .copied()
        .unwrap_or("");

    return format!("{name} {}", anchor.year());
}

/// "Friday 14 August 2026".
pub fn long_date_label(day: NaiveDate) -> String {
    let weekday = WEEKDAY_NAMES
        .get(day.weekday().num_days_from_monday() as usize)
        .copied()
        .unwrap_or("");
    let month = MONTH_NAMES.get(day.month0() as usize).copied().unwrap_or("");

    return format!("{weekday} {} {month} {}", day.day(), day.year());
}

/// "14 Aug", for the compact upcoming list.
pub fn short_date_label(day: NaiveDate) -> String {
    let month = MONTH_NAMES
        .get(day.month0() as usize)
        .copied()
        .unwrap_or("");

    // Three letters is the conventional abbreviation and every month name here is
    // longer than that, so slicing is safe. All ASCII, so byte slicing is char slicing.
    let short = if month.len() >= 3 { &month[..3] } else { month };

    return format!("{} {short}", day.day());
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        return NaiveDate::from_ymd_opt(y, m, d).expect("test date should be valid");
    }

    #[test]
    fn a_grid_is_always_six_weeks() {
        for month in 1..=12 {
            let grid = month_grid(date(2026, month, 1));
            assert_eq!(grid.len(), MONTH_CELLS, "month {month}");
        }
    }

    #[test]
    fn a_grid_starts_on_a_monday() {
        for month in 1..=12 {
            let grid = month_grid(date(2026, month, 1));
            assert_eq!(
                grid[0].date.weekday(),
                Weekday::Mon,
                "month {month} should start its grid on a Monday"
            );
        }
    }

    #[test]
    fn a_month_beginning_midweek_is_offset_by_its_weekday() {
        // The bug this whole module exists for: the first of the month used to land in
        // the first column whatever day it fell on.
        let first = date(2026, 2, 1);
        assert_eq!(first.weekday(), Weekday::Sun, "assumption about 2026");

        let grid = month_grid(first);
        let index = grid
            .iter()
            .position(|cell| cell.date == first)
            .expect("the first of the month should be in its own grid");

        // Sunday is the seventh column when weeks run Monday first.
        assert_eq!(index, 6);
        assert!(!grid[5].in_month, "the day before it belongs to January");
        assert_eq!(grid[5].date, date(2026, 1, 31));
    }

    #[test]
    fn a_month_beginning_on_a_monday_starts_in_the_first_cell() {
        let first = date(2026, 6, 1);
        assert_eq!(first.weekday(), Weekday::Mon, "assumption about 2026");

        let grid = month_grid(first);

        assert_eq!(grid[0].date, first);
        assert!(grid[0].in_month);
    }

    #[test]
    fn february_has_the_right_number_of_days() {
        assert_eq!(last_day_of_month(2026, 2), 28);
        assert_eq!(last_day_of_month(2024, 2), 29, "2024 is a leap year");
        assert_eq!(last_day_of_month(2000, 2), 29, "divisible by 400");
        assert_eq!(last_day_of_month(1900, 2), 28, "divisible by 100 but not 400");

        let in_month = month_grid(date(2024, 2, 1))
            .iter()
            .filter(|cell| cell.in_month)
            .count();
        assert_eq!(in_month, 29);
    }

    #[test]
    fn every_month_length_is_right() {
        let lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        for (index, expected) in lengths.iter().enumerate() {
            assert_eq!(
                last_day_of_month(2026, index as u32 + 1),
                *expected,
                "month {}",
                index + 1
            );
        }
    }

    #[test]
    fn stepping_a_month_clamps_the_day() {
        assert_eq!(add_months(date(2026, 1, 31), 1), date(2026, 2, 28));
        assert_eq!(add_months(date(2024, 1, 31), 1), date(2024, 2, 29));
        assert_eq!(add_months(date(2026, 3, 31), -1), date(2026, 2, 28));
    }

    #[test]
    fn stepping_a_month_crosses_the_year() {
        assert_eq!(add_months(date(2026, 12, 15), 1), date(2027, 1, 15));
        assert_eq!(add_months(date(2026, 1, 15), -1), date(2025, 12, 15));
        assert_eq!(add_months(date(2026, 1, 15), -13), date(2024, 12, 15));
    }

    #[test]
    fn stepping_a_year_clamps_a_leap_day() {
        assert_eq!(add_years(date(2024, 2, 29), 1), date(2025, 2, 28));
        assert_eq!(add_years(date(2024, 2, 29), 4), date(2028, 2, 29));
    }

    #[test]
    fn a_day_spans_to_the_start_of_the_next() {
        let (start, end) = day_bounds(date(2026, 8, 14));
        let (next_start, _) = day_bounds(date(2026, 8, 15));

        assert!(start < end);
        assert_eq!(end, next_start, "consecutive days must tile exactly");
        assert_eq!(local_date_of(start), date(2026, 8, 14));
    }

    #[test]
    fn a_grid_span_covers_the_neighbouring_days() {
        let anchor = date(2026, 2, 10);
        let (start, end) = grid_bounds(anchor);
        let grid = month_grid(anchor);

        assert_eq!(local_date_of(start), grid[0].date);
        assert!(local_date_of(end - Duration::seconds(1)) == grid[MONTH_CELLS - 1].date);
    }

    #[test]
    fn a_date_field_round_trips() {
        let day = date(2026, 8, 14);

        assert_eq!(format_day(day), "2026-08-14");
        assert_eq!(parse_day("2026-08-14"), Some(day));
        assert_eq!(parse_day(" 2026-08-14 "), Some(day), "surrounding space is fine");
        assert_eq!(parse_day("2026-13-45"), None, "an impossible date is refused");
        assert_eq!(parse_day("tomorrow"), None);
    }

    #[test]
    fn a_time_field_round_trips() {
        let at = NaiveTime::from_hms_opt(9, 30, 0).unwrap();

        assert_eq!(format_time(at), "09:30");
        assert_eq!(parse_time("09:30"), Some(at));
        assert_eq!(parse_time("9"), NaiveTime::from_hms_opt(9, 0, 0), "a bare hour works");
        assert_eq!(parse_time("25:00"), None);
    }

    #[test]
    fn half_past_is_half_way_through_the_hour() {
        // What the day column needs: 09:30 has to land between the 9 and 10 rows rather
        // than being rounded onto one of them.
        let day = date(2026, 8, 14);
        let (start, _) = day_bounds(day);

        let at = local_instant(day, NaiveTime::from_hms_opt(9, 30, 0).unwrap());
        assert_eq!(hours_from(start, at), 9.5);

        let quarter = local_instant(day, NaiveTime::from_hms_opt(14, 45, 0).unwrap());
        assert_eq!(hours_from(start, quarter), 14.75);
    }

    #[test]
    fn an_instant_past_midnight_reads_past_twenty_four() {
        let day = date(2026, 8, 14);
        let (start, _) = day_bounds(day);

        let at = local_instant(day + Duration::days(1), NaiveTime::from_hms_opt(2, 0, 0).unwrap());

        assert_eq!(hours_from(start, at), 26.0);
    }

    #[test]
    fn labels_read_the_way_a_person_writes_them() {
        assert_eq!(month_label(date(2026, 8, 1)), "August 2026");
        assert_eq!(long_date_label(date(2026, 8, 14)), "Friday 14 August 2026");
        assert_eq!(short_date_label(date(2026, 8, 14)), "14 Aug");
    }
}
