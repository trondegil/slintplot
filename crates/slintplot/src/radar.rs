//! Geometry for radar charts: one spoke per category, values along the spokes.
//!
//! Angles start at 12 o'clock and grow clockwise.

use std::f32::consts::TAU;
use std::fmt::Write;

use crate::layout::{Dataset, Labels, Point, Series, resolve_color};
use crate::path;
use crate::ticks::{NiceScale, format_tick};

/// A text label positioned in pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Label {
    pub x: f32,
    pub y: f32,
    pub text: String,
    /// Horizontal anchor: 0 when `x` is the left edge, 0.5 for the center,
    /// 1 for the right edge.
    pub align: f32,
    /// Vertical anchor: 0 when `y` is the top edge, 1 for the bottom edge.
    pub valign: f32,
}

pub struct RadarRequest<'a> {
    pub datasets: &'a [Dataset],
    pub labels: &'a dyn Labels,
    pub width: f32,
    pub height: f32,
    pub begin_at_zero: bool,
    /// Entry animation progress: the polygons grow from the center.
    pub progress: f32,
    pub font_size: f32,
    /// Colors for datasets without their own.
    pub palette: &'a [crate::layout::Rgba],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RadarLayout {
    /// One entry per dataset; `line` is the closed polygon.
    pub series: Vec<Series>,
    pub visible: Vec<usize>,
    /// Grid polygons and spokes, as path commands.
    pub grid: String,
    /// Value labels along the vertical spoke.
    pub ticks: Vec<Label>,
    /// Category labels around the chart.
    pub point_labels: Vec<Label>,
    pub center: (f32, f32),
    pub radius: f32,
    pub count: usize,
}

fn spoke(center: (f32, f32), r: f32, index: usize, count: usize) -> (f32, f32) {
    let angle = index as f32 * TAU / count as f32;
    (center.0 + r * angle.sin(), center.1 - r * angle.cos())
}

pub fn layout(req: &RadarRequest) -> RadarLayout {
    let count = req
        .datasets
        .iter()
        .map(|d| d.values.len())
        .chain(std::iter::once(req.labels.count()))
        .max()
        .unwrap_or(0);
    let visible: Vec<usize> = (0..req.datasets.len())
        .filter(|&i| !req.datasets[i].hidden)
        .collect();
    let center = (req.width / 2.0, req.height / 2.0);
    // Leave room for the category labels around the chart.
    let widest = (0..req.labels.count())
        .map(|i| req.labels.get(i).chars().count())
        .max()
        .unwrap_or(0) as f32;
    let label_gap = req.font_size * 0.6;
    let radius = (req.width / 2.0 - widest * req.font_size * 0.6 - label_gap)
        .min(req.height / 2.0 - req.font_size * 1.5 - label_gap)
        .max(0.0);
    let mut out = RadarLayout {
        visible,
        center,
        radius,
        count,
        ..Default::default()
    };
    if count < 3 {
        // A radar chart needs at least three spokes to enclose an area.
        return out;
    }

    let (mut min, mut max) = out
        .visible
        .iter()
        .flat_map(|&i| req.datasets[i].values.iter())
        .filter(|v| v.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    if min > max {
        (min, max) = (0.0, 1.0);
    }
    if req.begin_at_zero {
        min = min.min(0.0);
    }
    let max_ticks = (radius / (req.font_size.max(1.0) * 2.5))
        .floor()
        .clamp(2.0, 8.0) as usize;
    let scale = NiceScale::new(min, max, max_ticks);
    let r_of = |v: f64| ((v - scale.min) / (scale.max - scale.min)) as f32 * radius;

    for v in scale.ticks() {
        let r = r_of(v);
        if r > 0.0 {
            for i in 0..=count {
                let (x, y) = spoke(center, r, i % count, count);
                let _ = write!(
                    out.grid,
                    "{} {x:.2} {y:.2} ",
                    if i == 0 { "M" } else { "L" }
                );
            }
        }
        out.ticks.push(Label {
            x: center.0,
            y: center.1 - r,
            text: format_tick(v, scale.step),
            align: 0.5,
            valign: 0.5,
        });
    }
    for i in 0..count {
        let (x, y) = spoke(center, radius, i, count);
        let _ = write!(
            out.grid,
            "M {:.2} {:.2} L {x:.2} {y:.2} ",
            center.0, center.1
        );
        let (lx, ly) = spoke(center, radius + label_gap, i, count);
        let angle = i as f32 * TAU / count as f32;
        out.point_labels.push(Label {
            x: lx,
            y: ly,
            text: if i < req.labels.count() {
                req.labels.get(i)
            } else {
                String::new()
            },
            // Anchor each label on the side facing the chart: labels on the
            // right start at the spoke, labels at the top end above it, etc.
            align: 0.5 - 0.5 * angle.sin(),
            valign: 0.5 + 0.5 * angle.cos(),
        });
    }

    let progress = req.progress.clamp(0.0, 1.0);
    out.series = req
        .datasets
        .iter()
        .enumerate()
        .map(|(ds, d)| {
            let color = resolve_color(d.color, ds, req.palette);
            if d.hidden {
                return Series {
                    color,
                    ..Default::default()
                };
            }
            let points: Vec<Point> = d
                .values
                .iter()
                .enumerate()
                .take(count)
                .filter(|(_, v)| v.is_finite())
                .map(|(index, v)| {
                    let (x, y) = spoke(center, r_of(v).max(0.0) * progress, index, count);
                    Point {
                        x,
                        y,
                        index,
                        r: 0.0,
                    }
                })
                .collect();
            let coords: Vec<_> = points.iter().map(|p| (p.x, p.y)).collect();
            let polygon = if coords.len() > 1 {
                path::line(&coords, 0.0, path::UNBOUNDED) + " Z"
            } else {
                String::new()
            };
            Series {
                color,
                area: if d.fill {
                    polygon.clone()
                } else {
                    String::new()
                },
                line: polygon,
                points,
                ..Default::default()
            }
        })
        .collect();
    out
}

/// The category whose spoke is closest to `(x, y)`, if the point is within
/// the chart (plus a small margin).
pub fn hit(layout: &RadarLayout, x: f32, y: f32) -> Option<usize> {
    let (dx, dy) = (x - layout.center.0, y - layout.center.1);
    if layout.count < 3 || (dx * dx + dy * dy).sqrt() > layout.radius + 12.0 {
        return None;
    }
    let step = TAU / layout.count as f32;
    let angle = dx.atan2(-dy).rem_euclid(TAU);
    Some((angle / step).round() as usize % layout.count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(values: &[f64]) -> Dataset {
        Dataset {
            values: values.to_vec().into(),
            ..Default::default()
        }
    }

    #[test]
    fn spokes_and_values() {
        let labels: Vec<String> = ["A", "B", "C", "D"].map(String::from).to_vec();
        let data = [ds(&[10.0, 5.0, 0.0, 10.0])];
        let l = layout(&RadarRequest {
            datasets: &data,
            labels: &labels,
            width: 200.0,
            height: 200.0,
            begin_at_zero: true,
            progress: 1.0,
            font_size: 10.0,
            palette: &[],
        });
        assert_eq!(l.count, 4);
        let r = l.radius;
        let p = &l.series[0].points;
        assert!(
            (p[0].x - 100.0).abs() < 1e-3 && (p[0].y - (100.0 - r)).abs() < 1e-3,
            "A: top, full radius"
        );
        assert!(
            (p[1].x - (100.0 + r / 2.0)).abs() < 1e-3,
            "B: right, half radius"
        );
        assert!(
            (p[2].x - 100.0).abs() < 1e-3 && (p[2].y - 100.0).abs() < 1e-3,
            "C: zero, at the center"
        );
        assert!(l.series[0].line.ends_with(" Z"));
        assert!(
            l.point_labels[1].align.abs() < 1e-6,
            "right side labels start at the spoke"
        );
        assert!(
            (l.point_labels[3].align - 1.0).abs() < 1e-6,
            "left side labels end at the spoke"
        );
        assert!(
            (l.point_labels[0].valign - 1.0).abs() < 1e-6,
            "the top label sits above its spoke"
        );
        assert_eq!(hit(&l, 100.0 + r * 0.7, 100.0), Some(1));
        assert_eq!(hit(&l, 100.0, 100.0 - r * 0.5), Some(0));
        assert_eq!(hit(&l, 0.0, 0.0), None);
    }

    #[test]
    fn needs_three_spokes() {
        let labels: Vec<String> = vec!["A".into(), "B".into()];
        let l = layout(&RadarRequest {
            datasets: &[ds(&[1.0, 2.0])],
            labels: &labels,
            width: 100.0,
            height: 100.0,
            begin_at_zero: true,
            progress: 1.0,
            font_size: 10.0,
            palette: &[],
        });
        assert!(l.series.is_empty() && l.grid.is_empty());
    }
}
