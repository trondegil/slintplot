//! How numbers and dates are written in a language: separators, month and
//! weekday names, and date patterns. The default is English.

/// Number and date formatting for a language. Empty fields, and name lists of
/// the wrong length, fall back to English (see [`Locale::or_english`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Locale {
    /// Before the decimals, as in "1.5" or "1,5".
    pub decimal: String,
    /// Between groups of thousands, as in "1,000" or "1 000".
    pub group: String,
    /// After a percentage, as in "45%" or "45 %".
    pub percent: String,
    /// The 12 short month names, January first.
    pub months: Vec<String>,
    /// The 7 short weekday names, Monday first.
    pub weekdays: Vec<String>,
    /// A day in a month, such as "{MMM} {d}" or "{d}. {MMM}".
    pub day_month: String,
    /// A month in a year, such as "{MMM} {yyyy}".
    pub month_year: String,
    /// A full date, such as "{MMM} {d}, {yyyy}".
    pub date: String,
    /// A date with a time, such as "{date}, {time}".
    pub date_time: String,
    /// A 12-hour clock with `am` and `pm`, instead of 24 hours.
    pub hour12: bool,
    pub am: String,
    pub pm: String,
}

impl Default for Locale {
    fn default() -> Self {
        let names = |names: &[&str]| names.iter().map(|n| n.to_string()).collect();
        Locale {
            decimal: ".".into(),
            group: ",".into(),
            percent: "%".into(),
            months: names(&[
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ]),
            weekdays: names(&["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]),
            day_month: "{MMM} {d}".into(),
            month_year: "{MMM} {yyyy}".into(),
            date: "{MMM} {d}, {yyyy}".into(),
            date_time: "{date}, {time}".into(),
            hour12: false,
            am: "AM".into(),
            pm: "PM".into(),
        }
    }
}

impl Locale {
    /// Replaces empty fields, and name lists of the wrong length, with English.
    /// The separators may be empty only together: "1000" with no grouping
    /// needs a decimal separator still.
    pub fn or_english(mut self) -> Self {
        let en = Locale::default();
        let fill = |s: &mut String, en: String| {
            if s.is_empty() {
                *s = en;
            }
        };
        if self.decimal.is_empty() {
            self.decimal = en.decimal;
            fill(&mut self.group, en.group);
        }
        fill(&mut self.percent, en.percent);
        if self.months.len() != 12 {
            self.months = en.months;
        }
        if self.weekdays.len() != 7 {
            self.weekdays = en.weekdays;
        }
        fill(&mut self.day_month, en.day_month);
        fill(&mut self.month_year, en.month_year);
        fill(&mut self.date, en.date);
        fill(&mut self.date_time, en.date_time);
        fill(&mut self.am, en.am);
        fill(&mut self.pm, en.pm);
        self
    }

    /// A number with `decimals` decimals and grouped thousands.
    pub fn number(&self, value: f64, decimals: usize) -> String {
        number(value, decimals, &self.decimal, &self.group)
    }

    /// A tick value, with just enough decimals for the given step.
    pub fn tick(&self, value: f64, step: f64) -> String {
        self.number(value, tick_decimals(step))
    }

    /// A data value with at most two decimals, without trailing zeros.
    pub fn value(&self, value: f64) -> String {
        trim_decimals(self.number(value, 2), &self.decimal)
    }

    /// The short name of a month, 1 to 12.
    pub fn month(&self, month: u32) -> &str {
        self.months
            .get(month.wrapping_sub(1) as usize)
            .map_or("", String::as_str)
    }

    /// A day in a month, such as "Oct 25".
    pub fn day_month(&self, day: u32, month: u32) -> String {
        self.fill(&self.day_month, day, month, 0)
    }

    /// A month in a year, such as "Oct 2026".
    pub fn month_year(&self, month: u32, year: i64) -> String {
        self.fill(&self.month_year, 0, month, year)
    }

    /// A full date, such as "Oct 25, 2026".
    pub fn date(&self, day: u32, month: u32, year: i64) -> String {
        self.fill(&self.date, day, month, year)
    }

    /// A date and a time, such as "Oct 25, 2026, 14:30".
    pub fn date_time(&self, date: &str, time: &str) -> String {
        self.date_time
            .replace("{date}", date)
            .replace("{time}", time)
    }

    /// A time of day, with seconds if given: "14:30", "2:30 PM", "14:30:05".
    pub fn time(&self, hour: u32, minute: u32, second: Option<u32>) -> String {
        let seconds = second.map_or(String::new(), |s| format!(":{s:02}"));
        if self.hour12 {
            let h = if hour.is_multiple_of(12) {
                12
            } else {
                hour % 12
            };
            let suffix = if hour < 12 { &self.am } else { &self.pm };
            format!("{h}:{minute:02}{seconds} {suffix}")
        } else {
            format!("{hour:02}:{minute:02}{seconds}")
        }
    }

    fn fill(&self, pattern: &str, day: u32, month: u32, year: i64) -> String {
        pattern
            .replace("{d}", &day.to_string())
            .replace("{MMM}", self.month(month))
            .replace("{yyyy}", &year.to_string())
    }
}

/// Just enough decimals to write multiples of `step` exactly: 1 for 0.5 and
/// for 2.5.
pub(crate) fn tick_decimals(step: f64) -> usize {
    if step.is_nan() || step <= 0.0 {
        return 0;
    }
    let mut decimals = (-step.log10().floor()).max(0.0) as usize;
    while decimals < 10 {
        let scaled = step * 10f64.powi(decimals as i32);
        if (scaled - scaled.round()).abs() <= scaled * 1e-9 {
            break;
        }
        decimals += 1;
    }
    decimals
}

/// Removes trailing zeros after the decimal separator, and the separator if
/// nothing is left after it.
pub(crate) fn trim_decimals(s: String, decimal: &str) -> String {
    if decimal.is_empty() || !s.contains(decimal) {
        return s;
    }
    let trimmed = s.trim_end_matches('0');
    trimmed.strip_suffix(decimal).unwrap_or(trimmed).to_owned()
}

/// A number with `decimals` decimals, `decimal` before them and `group`
/// between thousands.
pub(crate) fn number(value: f64, decimals: usize, decimal: &str, group: &str) -> String {
    let raw = format!("{:.*}", decimals, crate::ticks::clean(value).abs());
    let (int, frac) = raw
        .split_once('.')
        .map_or((raw.as_str(), None), |(i, f)| (i, Some(f)));
    let mut out = String::with_capacity(raw.len() + int.len() / 3 * group.len() + 1);
    if value < 0.0 && raw.chars().any(|c| c.is_ascii_digit() && c != '0') {
        out.push('-');
    }
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push_str(group);
        }
        out.push(c);
    }
    if let Some(frac) = frac {
        out.push_str(decimal);
        out.push_str(frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norwegian() -> Locale {
        Locale {
            decimal: ",".into(),
            group: "\u{a0}".into(),
            percent: "\u{a0}%".into(),
            months: [
                "jan.", "feb.", "mar.", "apr.", "mai", "jun.", "jul.", "aug.", "sep.", "okt.",
                "nov.", "des.",
            ]
            .map(String::from)
            .to_vec(),
            weekdays: Vec::new(),
            day_month: "{d}. {MMM}".into(),
            month_year: String::new(),
            date: "{d}. {MMM} {yyyy}".into(),
            date_time: String::new(),
            hour12: false,
            am: String::new(),
            pm: String::new(),
        }
        .or_english()
    }

    #[test]
    fn numbers_use_the_separators() {
        let nb = norwegian();
        assert_eq!(nb.number(1234567.891, 2), "1\u{a0}234\u{a0}567,89");
        assert_eq!(nb.value(-2.5), "-2,5");
        assert_eq!(nb.value(3.0), "3");
        assert_eq!(nb.tick(0.25, 0.05), "0,25");
        let en = Locale::default();
        assert_eq!(en.number(-1234.5, 1), "-1,234.5");
        assert_eq!(en.value(1000.10), "1,000.1");
    }

    #[test]
    fn dates_follow_the_patterns() {
        let nb = norwegian();
        assert_eq!(nb.day_month(25, 10), "25. okt.");
        assert_eq!(nb.date(1, 5, 2026), "1. mai 2026");
        // Left empty: English.
        assert_eq!(nb.month_year(10, 2026), "okt. 2026");
        assert_eq!(nb.weekdays[0], "Mon");
        let en = Locale::default();
        assert_eq!(en.date(25, 10, 2026), "Oct 25, 2026");
        assert_eq!(
            en.date_time("Oct 25, 2026", &en.time(14, 30, None)),
            "Oct 25, 2026, 14:30"
        );
    }

    #[test]
    fn twelve_hour_clock() {
        let us = Locale {
            hour12: true,
            ..Locale::default()
        };
        assert_eq!(us.time(0, 5, None), "12:05 AM");
        assert_eq!(us.time(12, 0, None), "12:00 PM");
        assert_eq!(us.time(23, 59, Some(7)), "11:59:07 PM");
        assert_eq!(Locale::default().time(9, 5, Some(0)), "09:05:00");
    }

    #[test]
    fn separators_fall_back_together() {
        // No grouping, with a decimal comma.
        let plain = Locale {
            decimal: ",".into(),
            group: String::new(),
            ..Locale::default()
        }
        .or_english();
        assert_eq!(plain.number(12345.5, 1), "12345,5");
        let empty = Locale {
            decimal: String::new(),
            group: String::new(),
            ..Locale::default()
        }
        .or_english();
        assert_eq!(empty.number(12345.5, 1), "12,345.5");
    }
}
