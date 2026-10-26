//! Geometry for waterfall and dumbbell charts.

use std::fmt::Write;

use crate::layout::ValueScale;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Step {
    /// The change, or, for a total, ignored.
    pub value: f64,
    /// A total: a bar from zero to the running total.
    pub total: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Increase,
    Decrease,
    Total,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaterfallBar {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub change: Change,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Waterfall {
    pub bars: Vec<WaterfallBar>,
    /// Lines from each bar's end to the next bar, as path commands.
    pub connectors: String,
    /// Width of a category.
    pub step: f32,
}

/// The running total after each step. Missing values change nothing.
pub fn running(steps: &[Step]) -> Vec<f64> {
    let mut sum = 0.0;
    steps
        .iter()
        .map(|s| {
            if !s.total && s.value.is_finite() {
                sum += s.value;
            }
            sum
        })
        .collect()
}

/// Lays out the bars on a value axis from `y_min` to `y_max`. `progress` is
/// the entry animation, which grows the bars from where they start.
pub fn layout(
    steps: &[Step],
    y_min: f64,
    y_max: f64,
    width: f32,
    height: f32,
    progress: f32,
) -> Waterfall {
    if steps.is_empty() || width <= 0.0 {
        return Waterfall::default();
    }
    let scale = ValueScale {
        min: y_min,
        max: y_max,
        logarithmic: false,
    };
    let totals = running(steps);
    let step = width / steps.len() as f32;
    let bar_width = step * 0.72;
    let progress = progress.clamp(0.0, 1.0) as f64;
    let mut bars = Vec::with_capacity(steps.len());
    let mut connectors = String::new();
    let mut previous = 0.0;
    for (index, s) in steps.iter().enumerate() {
        let (from, to, change) = if s.total {
            (0.0, totals[index], Change::Total)
        } else {
            let change = if totals[index] < previous {
                Change::Decrease
            } else {
                Change::Increase
            };
            (previous, totals[index], change)
        };
        let to = from + (to - from) * progress;
        let (a, b) = (scale.pos(from, height), scale.pos(to, height));
        let x = index as f32 * step + (step - bar_width) / 2.0;
        bars.push(WaterfallBar {
            index,
            x,
            y: a.min(b),
            width: bar_width,
            height: (a - b).abs(),
            change,
        });
        if index + 1 < steps.len() && progress >= 1.0 {
            let y = scale.pos(totals[index], height).round() + 0.5;
            let _ = write!(
                connectors,
                "M {:.2} {y:.2} L {:.2} {y:.2} ",
                x + bar_width,
                x + step
            );
        }
        previous = totals[index];
    }
    Waterfall {
        bars,
        connectors,
        step,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DumbbellRow {
    pub index: usize,
    /// The middle of the row, and the ends of the line through its points.
    pub y: f32,
    pub from: f32,
    pub to: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DumbbellDot {
    pub dataset: usize,
    pub index: usize,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dumbbell {
    pub rows: Vec<DumbbellRow>,
    pub dots: Vec<DumbbellDot>,
    /// Height of a category.
    pub step: f32,
}

/// Lays out `count` categories as rows, with a dot for each visible
/// dataset's value and a line from the smallest to the largest. `progress`
/// moves the dots out from the middle of each row.
pub fn dumbbell(
    datasets: &[(bool, Vec<f64>)],
    count: usize,
    scale: ValueScale,
    width: f32,
    height: f32,
    progress: f32,
) -> Dumbbell {
    if count == 0 {
        return Dumbbell::default();
    }
    let step = height / count as f32;
    let progress = progress.clamp(0.0, 1.0);
    let mut out = Dumbbell {
        step,
        ..Default::default()
    };
    for index in 0..count {
        let y = (index as f32 + 0.5) * step;
        let xs: Vec<(usize, f32)> = datasets
            .iter()
            .enumerate()
            .filter(|(_, (hidden, _))| !hidden)
            .filter_map(|(d, (_, values))| {
                let v = values.get(index).copied().filter(|v| v.is_finite())?;
                // Only positive values have a place on a logarithmic axis.
                if scale.logarithmic && v <= 0.0 {
                    return None;
                }
                Some((d, scale.fraction(v) as f32 * width))
            })
            .collect();
        if xs.is_empty() {
            continue;
        }
        let lo = xs.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let hi = xs.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
        let middle = (lo + hi) / 2.0;
        let at = |x: f32| middle + (x - middle) * progress;
        out.rows.push(DumbbellRow {
            index,
            y,
            from: at(lo),
            to: at(hi),
        });
        out.dots.extend(xs.iter().map(|&(dataset, x)| DumbbellDot {
            dataset,
            index,
            x: at(x),
            y,
        }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps() -> Vec<Step> {
        let step = |value, total| Step { value, total };
        vec![
            step(100.0, false),
            step(-30.0, false),
            step(f64::NAN, false),
            step(0.0, true),
            step(50.0, false),
        ]
    }

    #[test]
    fn running_totals() {
        assert_eq!(running(&steps()), [100.0, 70.0, 70.0, 70.0, 120.0]);
    }

    #[test]
    fn bars_float_from_the_previous_total() {
        let fall = layout(&steps(), 0.0, 200.0, 500.0, 200.0, 1.0);
        assert_eq!(fall.step, 100.0);
        let b = &fall.bars[1];
        assert_eq!((b.y, b.height, b.change), (100.0, 30.0, Change::Decrease));
        assert_eq!(b.x, 114.0);
        let total = &fall.bars[3];
        assert_eq!(
            (total.y, total.height, total.change),
            (130.0, 70.0, Change::Total)
        );
        assert_eq!(fall.bars[4].change, Change::Increase);
        assert!(
            fall.connectors
                .starts_with("M 86.00 100.50 L 114.00 100.50")
        );
    }

    #[test]
    fn dumbbell_rows_span_their_dots() {
        let scale = ValueScale {
            min: 0.0,
            max: 100.0,
            logarithmic: false,
        };
        let data = vec![
            (false, vec![10.0, 50.0]),
            (true, vec![0.0, 0.0]),
            (false, vec![60.0, f64::NAN]),
            (false, vec![30.0]),
        ];
        let bell = dumbbell(&data, 3, scale, 200.0, 90.0, 1.0);
        assert_eq!(bell.rows.len(), 2, "the third category has no values");
        let row = &bell.rows[0];
        assert!(row.y == 15.0 && row.from == 20.0 && (row.to - 120.0).abs() < 1e-4);
        assert_eq!(bell.rows[1].from, bell.rows[1].to, "one value: no line");
        let datasets: Vec<_> = bell.dots.iter().map(|d| d.dataset).collect();
        assert_eq!(datasets, [0, 2, 3, 0], "hidden datasets have no dots");
    }
}
