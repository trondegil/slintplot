//! Geometry for funnel charts: the stages as slices of one funnel, each as
//! wide at the top as its value and narrowing to the next stage's width,
//! with curved sides.

use std::fmt::Write;

use crate::layout::Rgba;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stage {
    /// Index of the stage in the input.
    pub index: usize,
    /// The slice's outline, relative to its bounding box `(x, y, width,
    /// height)`.
    pub path: String,
    pub bounds: (f32, f32, f32, f32),
    /// The widths at the top and bottom of the slice, centered on the
    /// funnel's axis.
    pub top: f32,
    pub bottom: f32,
    /// The value as a share of the first visible stage, and of the previous
    /// one (1 for the first).
    pub of_first: f64,
    pub of_previous: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Funnel {
    pub stages: Vec<Stage>,
    /// Height of a row: a slice and the gap below it.
    pub step: f32,
}

pub struct FunnelRequest<'a> {
    /// The values; `None` marks a hidden stage.
    pub values: &'a [Option<f64>],
    pub width: f32,
    pub height: f32,
    /// Space between the slices, in pixels.
    pub gap: f32,
    /// Curve the sides between the stages, instead of straight edges.
    pub curved: bool,
    /// The entry animation: the stages open out one after the other, from
    /// the top.
    pub progress: f32,
}

pub fn layout(req: &FunnelRequest) -> Funnel {
    let shown: Vec<(usize, f64)> = req
        .values
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|v| (i, if v.is_finite() { v.max(0.0) } else { 0.0 })))
        .collect();
    if shown.is_empty() || req.width <= 0.0 || req.height <= 0.0 {
        return Funnel::default();
    }
    let n = shown.len();
    let max = shown.iter().map(|s| s.1).fold(0.0, f64::max);
    let step = req.height / n as f32;
    // At most a fifth of a row, so many stages still read as one funnel.
    let gap = req.gap.clamp(0.0, step * 0.2);
    let center = req.width / 2.0;
    // Each stage opens out a little after the one above it.
    let progress = req.progress.clamp(0.0, 1.0);
    let stagger = 0.6;
    let width_of = |row: usize| {
        let delay = if n > 1 {
            row as f32 / (n - 1) as f32
        } else {
            0.0
        };
        let p = (progress * (1.0 + stagger) - stagger * delay).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - p).powi(3);
        let value = shown[row].1;
        if max > 0.0 && value > 0.0 {
            // Even tiny stages stay visible.
            ((value / max) as f32 * req.width).max(2.0) * eased
        } else {
            0.0
        }
    };
    let share = |a: f64, b: f64| if b > 0.0 { a / b } else { 0.0 };
    let stages = (0..n)
        .map(|row| {
            let (index, value) = shown[row];
            let top = width_of(row);
            // The last stage keeps its width: the funnel's spout.
            let bottom = if row + 1 < n { width_of(row + 1) } else { top };
            let h = step - gap;
            let wide = top.max(bottom);
            let bounds = (center - wide / 2.0, row as f32 * step, wide, h);
            let (x0, x1) = ((wide - top) / 2.0, (wide - bottom) / 2.0);
            let (r0, r1) = (x0 + top, x1 + bottom);
            let mut path = String::new();
            if req.curved {
                // Vertical tangents at both ends: an S-curve on each side.
                let m = h / 2.0;
                let _ = write!(
                    path,
                    "M {x0:.2} 0 L {r0:.2} 0 C {r0:.2} {m:.2} {r1:.2} {m:.2} {r1:.2} {h:.2} \
                     L {x1:.2} {h:.2} C {x1:.2} {m:.2} {x0:.2} {m:.2} {x0:.2} 0 Z"
                );
            } else {
                let _ = write!(
                    path,
                    "M {x0:.2} 0 L {r0:.2} 0 L {r1:.2} {h:.2} L {x1:.2} {h:.2} Z"
                );
            }
            Stage {
                index,
                path,
                bounds,
                top,
                bottom,
                of_first: share(value, shown[0].1),
                of_previous: if row == 0 {
                    1.0
                } else {
                    share(value, shown[row - 1].1)
                },
            }
        })
        .collect();
    Funnel { stages, step }
}

/// The color of each of `count` stages without a color of their own: `base`
/// for the first, fading towards white (or, in dark mode, towards black) for
/// the later ones, so the order reads in the color.
pub fn ramp(base: Rgba, count: usize, dark: bool) -> Vec<Rgba> {
    let (target, reach) = if dark { (24.0, 0.55) } else { (255.0, 0.62) };
    (0..count)
        .map(|i| {
            let t = if count > 1 {
                i as f32 / (count - 1) as f32 * reach
            } else {
                0.0
            };
            let mix = |c: u8| (c as f32 + (target - c as f32) * t).round() as u8;
            [mix(base[0]), mix(base[1]), mix(base[2]), base[3]]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(values: &[Option<f64>]) -> FunnelRequest<'_> {
        FunnelRequest {
            values,
            width: 200.0,
            height: 300.0,
            gap: 4.0,
            curved: false,
            progress: 1.0,
        }
    }

    #[test]
    fn slices_narrow_to_the_next_stage() {
        let values = [Some(100.0), None, Some(50.0), Some(10.0)];
        let funnel = layout(&request(&values));
        let indices: Vec<_> = funnel.stages.iter().map(|s| s.index).collect();
        assert_eq!(indices, [0, 2, 3], "hidden stages are left out");
        assert_eq!(funnel.step, 100.0);
        let first = &funnel.stages[0];
        assert_eq!((first.top, first.bottom), (200.0, 100.0));
        assert_eq!(first.bounds, (0.0, 0.0, 200.0, 96.0));
        assert_eq!(
            first.path,
            "M 0.00 0 L 200.00 0 L 150.00 96.00 L 50.00 96.00 Z"
        );
        let last = &funnel.stages[2];
        assert_eq!((last.top, last.bottom), (20.0, 20.0), "the spout");
        assert_eq!(last.bounds, (90.0, 200.0, 20.0, 96.0));
        let second = &funnel.stages[1];
        assert_eq!((second.of_first, second.of_previous), (0.5, 0.5));
        assert_eq!(last.of_previous, 0.2);
    }

    #[test]
    fn curved_sides_and_the_cascade() {
        let values = [Some(100.0), Some(50.0)];
        let mut req = request(&values);
        req.curved = true;
        assert_eq!(layout(&req).stages[0].path.matches(" C ").count(), 2);
        req.progress = 0.5;
        let half = layout(&req);
        assert!(
            half.stages[0].top > half.stages[1].top * 2.0 + 1.0,
            "the second stage opens after the first"
        );
    }

    #[test]
    fn missing_values_take_no_width() {
        let funnel = layout(&request(&[Some(f64::NAN), Some(-1.0)]));
        assert!(
            funnel
                .stages
                .iter()
                .all(|s| s.top == 0.0 && s.of_first == 0.0)
        );
    }

    #[test]
    fn colors_fade_down_the_funnel() {
        let light = ramp([0, 100, 200, 255], 3, false);
        assert_eq!(light[0], [0, 100, 200, 255]);
        assert_eq!(light[2], [158, 196, 234, 255]);
        let dark = ramp([0, 100, 200, 255], 2, true);
        assert_eq!(dark[1], [13, 58, 103, 255]);
    }
}
