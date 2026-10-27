//! Time axes: calendar-aligned ticks and date formatting, in UTC with an
//! optional fixed offset. Times are seconds since the Unix epoch.

use crate::locale::Locale;

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = month as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The (year, month, day) of a day number since 1970-01-01.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Seconds since the Unix epoch of a date and time (UTC).
pub fn unix_seconds(year: i64, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> f64 {
    (days_from_civil(year, month, day) * 86_400) as f64
        + (hour * 3600 + minute * 60 + second) as f64
}

/// A calendar-aware tick interval.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Unit {
    /// A fixed number of seconds.
    Seconds(f64),
    Months(u32),
    Years(u32),
}

const DAY: f64 = 86_400.0;
const UNITS: &[Unit] = &[
    Unit::Seconds(1.0),
    Unit::Seconds(2.0),
    Unit::Seconds(5.0),
    Unit::Seconds(10.0),
    Unit::Seconds(15.0),
    Unit::Seconds(30.0),
    Unit::Seconds(60.0),
    Unit::Seconds(120.0),
    Unit::Seconds(300.0),
    Unit::Seconds(600.0),
    Unit::Seconds(900.0),
    Unit::Seconds(1800.0),
    Unit::Seconds(3600.0),
    Unit::Seconds(7200.0),
    Unit::Seconds(10_800.0),
    Unit::Seconds(21_600.0),
    Unit::Seconds(43_200.0),
    Unit::Seconds(DAY),
    Unit::Seconds(2.0 * DAY),
    Unit::Seconds(7.0 * DAY),
    Unit::Months(1),
    Unit::Months(3),
    Unit::Months(6),
    Unit::Years(1),
    Unit::Years(2),
    Unit::Years(5),
    Unit::Years(10),
    Unit::Years(25),
    Unit::Years(50),
    Unit::Years(100),
];

impl Unit {
    fn approx_seconds(self) -> f64 {
        match self {
            Unit::Seconds(s) => s,
            Unit::Months(m) => m as f64 * 30.44 * DAY,
            Unit::Years(y) => y as f64 * 365.25 * DAY,
        }
    }
}

/// Splits local seconds into (days, seconds of the day).
fn split(local: f64) -> (i64, f64) {
    let days = (local / DAY).floor();
    (days as i64, local - days * DAY)
}

/// Formats a time for a tick of the given unit. The flag tells whether the
/// label carries the date (sub-day units) or the year (months).
fn label(local: f64, unit: Unit, locale: &Locale) -> (String, bool) {
    let (days, secs) = split(local);
    let (year, month, day) = civil_from_days(days);
    let secs = secs.round() as u32;
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    match unit {
        Unit::Seconds(u) if u < 60.0 => (locale.time(h, m, Some(s)), false),
        // At midnight, a date says more than "00:00".
        Unit::Seconds(u) if u < DAY && secs != 0 => (locale.time(h, m, None), false),
        Unit::Seconds(_) => (locale.day_month(day, month), true),
        Unit::Months(_) if month == 1 => (locale.month_year(month, year), true),
        Unit::Months(_) => (locale.month(month).to_owned(), false),
        Unit::Years(_) => (year.to_string(), true),
    }
}

/// Formats a time for a tooltip, such as "Oct 25, 2026, 14:30", in English.
pub fn format_datetime(time: f64, utc_offset_minutes: i32) -> String {
    format_datetime_in(time, utc_offset_minutes, &Locale::default())
}

/// Formats a time for a tooltip in the given locale.
pub fn format_datetime_in(time: f64, utc_offset_minutes: i32, locale: &Locale) -> String {
    let local = time + utc_offset_minutes as f64 * 60.0;
    let (days, secs) = split(local);
    let (year, month, day) = civil_from_days(days);
    let secs = secs.round() as u32;
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    let date = locale.date(day, month, year);
    match (h, m, s) {
        (0, 0, 0) => date,
        (_, _, 0) => locale.date_time(&date, &locale.time(h, m, None)),
        _ => locale.date_time(&date, &locale.time(h, m, Some(s))),
    }
}

/// Ticks for the times `start` to `end` (seconds since the epoch), as
/// (time, label). Ticks fall on round local times: whole minutes, midnights,
/// month starts and so on.
pub fn time_ticks(
    start: f64,
    end: f64,
    max_ticks: usize,
    utc_offset_minutes: i32,
) -> Vec<(f64, String)> {
    time_ticks_in(
        start,
        end,
        max_ticks,
        utc_offset_minutes,
        &Locale::default(),
    )
}

/// Ticks for the times `start` to `end`, labeled in the given locale.
pub fn time_ticks_in(
    start: f64,
    end: f64,
    max_ticks: usize,
    utc_offset_minutes: i32,
    locale: &Locale,
) -> Vec<(f64, String)> {
    // Also rejects NaN.
    if end.partial_cmp(&start) != Some(std::cmp::Ordering::Greater) {
        return Vec::new();
    }
    let offset = utc_offset_minutes as f64 * 60.0;
    let span = end - start;
    let intervals = (max_ticks.max(2) - 1) as f64;
    let unit = UNITS
        .iter()
        .copied()
        .find(|u| span / u.approx_seconds() <= intervals)
        .unwrap_or(Unit::Years(100));
    let (local_start, local_end) = (start + offset, end + offset);
    let mut ticks = Vec::new();
    match unit {
        Unit::Seconds(u) => {
            // Weeks start on Monday; 1970-01-01 was a Thursday.
            let shift = if u == 7.0 * DAY { 3.0 * DAY } else { 0.0 };
            let mut t = ((local_start + shift) / u).ceil() * u - shift;
            while t <= local_end + 1e-6 {
                ticks.push((t - offset, label(t, unit, locale)));
                t += u;
            }
        }
        Unit::Months(n) | Unit::Years(n) => {
            let months_per_step = if let Unit::Years(_) = unit { n * 12 } else { n };
            let (year, month, _) = civil_from_days(split(local_start).0);
            // Count months from year 0, and start at the next multiple of the step.
            let mut index = year * 12 + month as i64 - 1;
            let step = months_per_step as i64;
            index = index.div_euclid(step) * step;
            loop {
                let (y, m) = (index.div_euclid(12), index.rem_euclid(12) as u32 + 1);
                let t = days_from_civil(y, m, 1) as f64 * DAY;
                if t > local_end + 1e-6 {
                    break;
                }
                if t >= local_start - 1e-6 {
                    ticks.push((t - offset, label(t, unit, locale)));
                }
                index += step;
            }
        }
    }
    // Zoomed in to less than a day, or a year, no tick may carry the date or
    // the year: the first tick does.
    let context = match unit {
        Unit::Seconds(u) if u < DAY => true,
        Unit::Months(_) => true,
        _ => false,
    } && !ticks.iter().any(|(_, (_, dated))| *dated);
    if context && let Some((t, (label, _))) = ticks.first_mut() {
        let (year, month, day) = civil_from_days(split(*t + offset).0);
        *label = match unit {
            Unit::Months(_) => locale.month_year(month, year),
            _ => format!("{} {label}", locale.day_month(day, month)),
        };
    }
    ticks
        .into_iter()
        .map(|(t, (label, _))| (t, label))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(civil_from_days(20_386), (2025, 10, 25));
        for days in [-800_000, -1, 0, 59, 10_000, 20_386, 2_932_896] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(unix_seconds(2026, 10, 25, 12, 0, 0), 1_792_929_600.0);
    }

    #[test]
    fn hour_ticks_fall_on_round_times() {
        let start = unix_seconds(2026, 10, 25, 9, 17, 0);
        let ticks = time_ticks(start, start + 6.0 * 3600.0, 8, 0);
        let labels: Vec<_> = ticks.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            labels,
            ["Oct 25 10:00", "11:00", "12:00", "13:00", "14:00", "15:00"],
            "without a midnight tick, the first tick has the date"
        );
    }

    #[test]
    fn day_and_month_ticks() {
        let start = unix_seconds(2026, 1, 1, 0, 0, 0);
        let days = time_ticks(start, start + 6.0 * DAY, 8, 0);
        assert_eq!(days[0].1, "Jan 1");
        assert_eq!(days.len(), 7);
        let months = time_ticks(start, unix_seconds(2026, 12, 31, 0, 0, 0), 13, 0);
        let labels: Vec<_> = months.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(labels[..3], ["Jan 2026", "Feb", "Mar"]);
        assert_eq!(months.len(), 12);
        let spring = time_ticks(
            unix_seconds(2026, 3, 1, 0, 0, 0),
            unix_seconds(2026, 6, 1, 0, 0, 0),
            8,
            0,
        );
        let labels: Vec<_> = spring.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            labels,
            ["Mar 2026", "Apr", "May", "Jun"],
            "without January, the first tick has the year"
        );
        let years = time_ticks(start, unix_seconds(2040, 1, 1, 0, 0, 0), 8, 0);
        assert_eq!(years.first().map(|t| t.1.as_str()), Some("2026"));
    }

    #[test]
    fn utc_offset_shifts_local_times() {
        let t = unix_seconds(2026, 10, 25, 12, 0, 0);
        assert_eq!(format_datetime(t, 0), "Oct 25, 2026, 12:00");
        assert_eq!(format_datetime(t, 120), "Oct 25, 2026, 14:00");
        assert_eq!(
            format_datetime(unix_seconds(2026, 10, 25, 0, 0, 0), 0),
            "Oct 25, 2026"
        );
    }

    fn german() -> Locale {
        Locale {
            decimal: ",".into(),
            group: ".".into(),
            months: [
                "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.",
                "Nov.", "Dez.",
            ]
            .map(String::from)
            .to_vec(),
            day_month: "{d}. {MMM}".into(),
            date: "{d}. {MMM} {yyyy}".into(),
            ..Locale::default()
        }
    }

    #[test]
    fn ticks_and_dates_in_another_language() {
        let de = german();
        let start = unix_seconds(2026, 10, 25, 9, 17, 0);
        let hours = time_ticks_in(start, start + 6.0 * 3600.0, 8, 0, &de);
        assert_eq!(hours[0].1, "25. Okt. 10:00");
        let spring = time_ticks_in(
            unix_seconds(2026, 3, 1, 0, 0, 0),
            unix_seconds(2026, 6, 1, 0, 0, 0),
            8,
            0,
            &de,
        );
        let labels: Vec<_> = spring.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(labels, ["März 2026", "Apr.", "Mai", "Juni"]);
        let t = unix_seconds(2026, 10, 25, 14, 30, 0);
        assert_eq!(format_datetime_in(t, 0, &de), "25. Okt. 2026, 14:30");
        let us = Locale {
            hour12: true,
            ..Locale::default()
        };
        assert_eq!(format_datetime_in(t, 0, &us), "Oct 25, 2026, 2:30 PM");
    }
}
