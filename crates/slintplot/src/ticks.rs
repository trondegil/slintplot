//! "Nice number" axis scales: bounds and steps of 1, 2 or 5 times a power of ten.

use crate::locale::{Locale, tick_decimals, trim_decimals};

/// A linear scale whose bounds and step are rounded to human-friendly numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NiceScale {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl NiceScale {
    /// Builds a scale covering `[min, max]` with at most roughly `max_ticks` ticks.
    pub fn new(min: f64, max: f64, max_ticks: usize) -> Self {
        let (mut min, mut max) = if min <= max { (min, max) } else { (max, min) };
        if !min.is_finite() || !max.is_finite() {
            return Self {
                min: 0.0,
                max: 1.0,
                step: 0.2,
            };
        }
        if min == max {
            // Widen a flat range by one unit each way.
            min -= 1.0;
            max += 1.0;
        }
        // The smallest nice step that keeps within `max_ticks`.
        let max_spaces = (max_ticks.max(2) - 1) as f64;
        let mut step = nice_num((max - min) / max_spaces);
        let spaces = (max / step).ceil() - (min / step).floor();
        if spaces > max_spaces {
            step = nice_num(spaces * step / max_spaces);
        }
        Self {
            min: clean((min / step).floor() * step),
            max: clean((max / step).ceil() * step),
            step,
        }
    }

    /// The tick values from `min` to `max`, inclusive.
    pub fn ticks(&self) -> Vec<f64> {
        let count = ((self.max - self.min) / self.step).round() as usize;
        (0..=count)
            .map(|i| clean(self.min + i as f64 * self.step))
            .collect()
    }
}

impl NiceScale {
    /// A scale with exactly `count` ticks that covers `[min, max]`, so that
    /// its ticks line up with those of another axis with `count` ticks. Steps
    /// are 1, 1.5, 2, 2.5, 3, 4, 5, 6 or 8 times a power of ten: more than a
    /// free axis uses, so the data fills more of the fixed number of ticks.
    pub fn aligned(min: f64, max: f64, count: usize) -> Self {
        let (min, max) = if max > min {
            (min, max)
        } else {
            (min - 1.0, max + 1.0)
        };
        let intervals = count.max(2) as f64 - 1.0;
        let raw = (max - min) / intervals;
        let mut magnitude = 10f64.powf(raw.log10().floor());
        loop {
            for factor in [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0] {
                let step = clean(factor * magnitude);
                if step < raw * (1.0 - 1e-9) {
                    continue;
                }
                let lo = clean((min / step + 1e-9).floor() * step);
                let hi = clean(lo + step * intervals);
                if hi >= max - step * 1e-9 {
                    return NiceScale {
                        min: lo,
                        max: hi,
                        step,
                    };
                }
            }
            magnitude *= 10.0;
        }
    }
}

/// Rounds `x` to 1, 2, 5 or 10 times a power of ten.
fn nice_num(x: f64) -> f64 {
    let exp = x.log10().floor();
    let nice = match x / 10f64.powf(exp) {
        f if f < 1.5 => 1.0,
        f if f < 3.0 => 2.0,
        f if f < 7.0 => 5.0,
        _ => 10.0,
    };
    nice * 10f64.powf(exp)
}

/// Removes floating point noise such as `0.30000000000000004`.
pub(crate) fn clean(v: f64) -> f64 {
    let rounded = (v * 1e9).round() / 1e9;
    if rounded == 0.0 { 0.0 } else { rounded }
}

/// Formats a tick value with just enough decimals for the given step, in
/// English. [`Locale::tick`] formats for other languages.
pub fn format_tick(value: f64, step: f64) -> String {
    format_number(value, tick_decimals(step))
}

/// Formats a value with thousands separators and at most two decimals, trimming
/// trailing zeros, in English. [`Locale::value`] formats for other languages.
pub fn format_value(value: f64) -> String {
    trim_decimals(format_number(value, 2), ".")
}

/// How values are written on axes, in tooltips and in data labels: a prefix
/// and suffix (such as a currency or unit), and optionally a fixed number of
/// decimals, in the separators of a language.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Format {
    pub prefix: String,
    pub suffix: String,
    /// `None` chooses the decimals automatically.
    pub decimals: Option<usize>,
    pub locale: Locale,
}

impl Format {
    /// A tick label, with just enough decimals for `step` unless fixed.
    pub fn tick(&self, value: f64, step: f64) -> String {
        let text = match self.decimals {
            Some(d) => self.locale.number(value, d),
            None => self.locale.tick(value, step),
        };
        self.wrap(&text)
    }

    /// A data value, with at most two decimals unless fixed.
    pub fn value(&self, value: f64) -> String {
        let text = match self.decimals {
            Some(d) => self.locale.number(value, d),
            None => self.locale.value(value),
        };
        self.wrap(&text)
    }

    /// Adds the prefix and suffix; a minus sign goes before the prefix, as
    /// in "-$5".
    fn wrap(&self, text: &str) -> String {
        match text.strip_prefix('-') {
            Some(abs) if !self.prefix.is_empty() => format!("-{}{abs}{}", self.prefix, self.suffix),
            _ => format!("{}{text}{}", self.prefix, self.suffix),
        }
    }
}

/// The powers of ten just below `min` and just above `max`, for a
/// logarithmic axis. Both must be positive.
pub fn log_bounds(min: f64, max: f64) -> (f64, f64) {
    let lo = 10f64.powf(min.log10().floor());
    let hi = 10f64.powf(max.log10().ceil());
    if hi > lo {
        (clean(lo), clean(hi))
    } else {
        (clean(lo), clean(lo * 10.0))
    }
}

/// Ticks of a logarithmic axis from `min` to `max`: the powers of ten, plus 2
/// and 5 times them when there is room for at most `max_ticks`.
pub fn log_ticks(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    if !(min > 0.0 && max > min) {
        return Vec::new();
    }
    let first = min.log10().floor() as i32;
    let last = max.log10().ceil() as i32;
    let inside = |v: &f64| *v >= min * (1.0 - 1e-9) && *v <= max * (1.0 + 1e-9);
    for factors in [&[1.0, 2.0, 5.0][..], &[1.0][..]] {
        let ticks: Vec<f64> = (first..=last)
            .flat_map(|e| factors.iter().map(move |f| clean(f * 10f64.powi(e))))
            .filter(inside)
            .collect();
        if ticks.len() <= max_ticks.max(2) {
            return ticks;
        }
    }
    // Many decades: every n-th power of ten.
    let every = ((last - first) as usize)
        .div_ceil(max_ticks.max(2) - 1)
        .max(1);
    (first..=last)
        .step_by(every)
        .map(|e| clean(10f64.powi(e)))
        .filter(inside)
        .collect()
}

fn format_number(value: f64, decimals: usize) -> String {
    crate::locale::number(value, decimals, ".", ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minus_goes_before_the_prefix() {
        let money = Format {
            prefix: "$".into(),
            suffix: "k".into(),
            ..Default::default()
        };
        assert_eq!(money.value(-185.0), "-$185k");
        assert_eq!(money.tick(-50.0, 50.0), "-$50k");
        assert_eq!(money.value(20.0), "$20k");
    }

    #[test]
    fn nice_bounds() {
        let s = NiceScale::new(3.0, 97.0, 6);
        assert_eq!((s.min, s.max, s.step), (0.0, 100.0, 20.0));
        assert_eq!(s.ticks(), vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
    }

    #[test]
    fn negative_and_fractional() {
        let s = NiceScale::new(-0.42, 0.87, 6);
        assert_eq!(s.step, 0.5);
        assert_eq!(s.ticks(), vec![-0.5, 0.0, 0.5, 1.0]);
    }

    #[test]
    fn flat_range() {
        let s = NiceScale::new(5.0, 5.0, 5);
        assert!(s.min < 5.0 && s.max > 5.0);
    }

    #[test]
    fn custom_formats() {
        let money = Format {
            prefix: "$".into(),
            decimals: Some(2),
            ..Default::default()
        };
        assert_eq!(money.value(1234.5), "$1,234.50");
        assert_eq!(money.tick(20.0, 20.0), "$20.00");
        let percent = Format {
            suffix: " %".into(),
            ..Default::default()
        };
        assert_eq!(percent.value(12.345), "12.35 %");
        assert_eq!(percent.tick(0.5, 0.5), "0.5 %");
    }

    #[test]
    fn logarithmic_ticks() {
        assert_eq!(log_bounds(3.0, 4500.0), (1.0, 10000.0));
        assert_eq!(
            log_ticks(1.0, 1000.0, 11),
            vec![1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0]
        );
        assert_eq!(
            log_ticks(1.0, 1e6, 8),
            vec![1.0, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0]
        );
        assert_eq!(log_ticks(1.0, 1e12, 5), vec![1.0, 1e3, 1e6, 1e9, 1e12]);
    }

    #[test]
    fn formatting() {
        assert_eq!(format_tick(1500.0, 500.0), "1,500");
        assert_eq!(format_tick(0.4, 0.2), "0.4");
        assert_eq!(format_tick(-1234567.0, 1000.0), "-1,234,567");
        assert_eq!(format_value(12.5), "12.5");
        assert_eq!(format_value(3.0), "3");
        assert_eq!(format_value(-0.001), "0");
    }
}
