//! Heatmaps: a matrix of values, and a calendar of daily values like GitHub's
//! contribution graph.

use crate::locale::Locale;
use crate::time::civil_from_days;

/// The smallest and largest finite values, or `None` without any.
pub fn extent(values: impl IntoIterator<Item = f64>) -> Option<(f64, f64)> {
    values
        .into_iter()
        .filter(|v| v.is_finite())
        .fold(None, |acc, v| {
            Some(acc.map_or((v, v), |(lo, hi): (f64, f64)| (lo.min(v), hi.max(v))))
        })
}

/// A day of a calendar heatmap.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Day {
    /// Index of the day's value.
    pub index: usize,
    /// Week (column) and weekday (row, 0 is the first day of the week).
    pub week: usize,
    pub weekday: usize,
    /// 0 for no activity (zero, negative or missing), else 1 to `levels`.
    pub level: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Calendar {
    pub days: Vec<Day>,
    pub weeks: usize,
    /// The week (column) where each month starts, with the month's name.
    pub months: Vec<(usize, String)>,
    /// The values that start levels 2 to `levels`: activity at or above
    /// `thresholds[i]` is level `i + 2`.
    pub thresholds: Vec<f64>,
}

/// The weekday labels of a calendar's rows: Monday, Wednesday and Friday,
/// like GitHub, with the other rows blank.
pub fn weekday_labels(locale: &Locale, week_starts_monday: bool) -> Vec<String> {
    let first = usize::from(!week_starts_monday);
    (0..7)
        .map(|row: usize| match row.checked_sub(first) {
            Some(day @ (0 | 2 | 4)) => locale.weekdays.get(day).cloned().unwrap_or_default(),
            _ => String::new(),
        })
        .collect()
}

/// Lays out one value per day, starting on `start_day` (days since
/// 1970-01-01), in weeks starting on Monday or, like GitHub, on Sunday.
/// Positive values are split into `levels` levels by quantiles, so a few
/// very active days don't wash out the rest.
pub fn calendar(
    start_day: i64,
    values: &[f64],
    levels: usize,
    week_starts_monday: bool,
) -> Calendar {
    calendar_in(
        start_day,
        values,
        levels,
        week_starts_monday,
        &Locale::default(),
    )
}

/// Like [`calendar`], with month names from the given locale.
pub fn calendar_in(
    start_day: i64,
    values: &[f64],
    levels: usize,
    week_starts_monday: bool,
    locale: &Locale,
) -> Calendar {
    let levels = levels.max(1);
    // 1970-01-01 was a Thursday: 4 days after Sunday, 3 after Monday.
    let first_weekday = (start_day + if week_starts_monday { 3 } else { 4 }).rem_euclid(7) as usize;
    let mut active: Vec<f64> = values
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v > 0.0)
        .collect();
    active.sort_by(f64::total_cmp);
    let thresholds: Vec<f64> = (1..levels)
        .map(|i| {
            active
                .get(active.len() * i / levels)
                .copied()
                .unwrap_or(f64::INFINITY)
        })
        .collect();
    let level = |v: f64| {
        if !(v.is_finite() && v > 0.0) {
            0
        } else {
            1 + thresholds.iter().filter(|&&t| v >= t).count()
        }
    };
    let mut months = Vec::new();
    let mut last_month = None;
    let days: Vec<Day> = values
        .iter()
        .enumerate()
        .map(|(index, &v)| {
            let slot = first_weekday + index;
            let (week, weekday) = (slot / 7, slot % 7);
            let (_, month, day) = civil_from_days(start_day + index as i64);
            // A month is labeled at the first week that starts inside it.
            let week_starts_in_month = weekday == 0 && day <= 7;
            if (week_starts_in_month || index == 0) && last_month != Some(month) {
                months.push((week, locale.month(month).to_owned()));
                last_month = Some(month);
            }
            Day {
                index,
                week,
                weekday,
                level: level(v),
            }
        })
        .collect();
    let weeks = days.last().map_or(0, |d| d.week + 1);
    // Drop a label that would collide with the next one (a partial first week).
    if months.len() > 1 && months[1].0 - months[0].0 < 2 {
        months.remove(0);
    }
    Calendar {
        days,
        weeks,
        months,
        thresholds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::days_from_civil;

    #[test]
    fn extent_skips_missing_values() {
        assert_eq!(extent([3.0, f64::NAN, -1.0, 8.0]), Some((-1.0, 8.0)));
        assert_eq!(extent([f64::NAN]), None);
    }

    #[test]
    fn calendar_places_days_in_weeks() {
        // 2026-10-25 is a Sunday.
        let start = days_from_civil(2026, 10, 25);
        let values = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let sunday = calendar(start, &values, 4, false);
        assert_eq!((sunday.days[0].week, sunday.days[0].weekday), (0, 0));
        assert_eq!((sunday.days[7].week, sunday.days[7].weekday), (1, 0));
        let monday = calendar(start, &values, 4, true);
        assert_eq!(
            (monday.days[0].week, monday.days[0].weekday),
            (0, 6),
            "Sunday ends a Monday week"
        );
        assert_eq!(monday.days[1].week, 1);
        assert_eq!(sunday.days[0].level, 0, "no activity");
        assert_eq!(sunday.days[1].level, 1);
        assert_eq!(sunday.days[8].level, 4);
    }

    #[test]
    fn a_year_has_month_labels() {
        let start = days_from_civil(2025, 10, 26);
        let cal = calendar(start, &vec![1.0; 365], 4, false);
        assert_eq!(cal.weeks, 53);
        let names: Vec<_> = cal.months.iter().map(|(_, m)| m.as_str()).collect();
        assert!(names.len() >= 12 && names.contains(&"Jan"), "{names:?}");
        assert!(cal.months.windows(2).all(|w| w[1].0 > w[0].0));
    }

    #[test]
    fn weekday_rows_follow_the_first_day() {
        let en = Locale::default();
        assert_eq!(
            weekday_labels(&en, true),
            ["Mon", "", "Wed", "", "Fri", "", ""]
        );
        assert_eq!(
            weekday_labels(&en, false),
            ["", "Mon", "", "Wed", "", "Fri", ""]
        );
        let fr = Locale {
            weekdays: ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."]
                .map(String::from)
                .to_vec(),
            months: [
                "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
                "nov.", "déc.",
            ]
            .map(String::from)
            .to_vec(),
            ..Locale::default()
        };
        assert_eq!(weekday_labels(&fr, true)[2], "mer.");
        let cal = calendar_in(days_from_civil(2026, 1, 1), &[1.0; 40], 4, true, &fr);
        assert_eq!(cal.months[0].1, "janv.");
    }
}
