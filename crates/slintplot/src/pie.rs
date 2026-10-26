//! Geometry for pie, doughnut and polar area charts.
//!
//! Angles are in radians, starting at 12 o'clock and growing clockwise.

use std::f32::consts::{PI, TAU};
use std::fmt::Write;

use crate::layout::{Rgba, Tick, resolve_color};
use crate::ticks::{NiceScale, format_tick};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Slice {
    pub value: f64,
    pub color: Rgba,
    pub hidden: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SliceGeometry {
    /// Index of the slice in the input.
    pub index: usize,
    pub path: String,
    pub color: Rgba,
    pub start: f32,
    pub end: f32,
    /// Share of the visible total, from 0 to 1.
    pub fraction: f32,
    /// Outer radius of this slice; differs per slice in polar area charts.
    pub outer: f32,
}

impl SliceGeometry {
    /// Unit vector pointing from the center through the middle of the slice.
    pub fn direction(&self) -> (f32, f32) {
        let mid = (self.start + self.end) / 2.0;
        (mid.sin(), -mid.cos())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PieLayout {
    pub slices: Vec<SliceGeometry>,
    /// Resolved colors of all input slices, hidden ones included (for the legend).
    pub colors: Vec<Rgba>,
    /// Sum of the visible values.
    pub total: f64,
    pub center: (f32, f32),
    pub radius: f32,
    pub inner_radius: f32,
    /// Polar area charts: the value scale, as rings with their radius in `pos`.
    pub rings: Vec<Tick>,
    /// Polar area charts: the ring outlines, as path commands.
    pub grid: String,
}

pub struct PieRequest<'a> {
    pub slices: &'a [Slice],
    pub width: f32,
    pub height: f32,
    /// Size of the hole, as a fraction of the radius: 0 for a pie, 0.5 for a
    /// doughnut.
    pub cutout: f32,
    /// Space kept free around the pie so hovered slices can move outwards.
    pub hover_offset: f32,
    /// Entry animation progress: the slices sweep in from 12 o'clock, or
    /// grow from the center in polar area charts.
    pub progress: f32,
    /// Polar area chart: equal angles, and the radius shows the value.
    pub polar: bool,
    pub font_size: f32,
    /// Colors for slices without their own.
    pub palette: &'a [Rgba],
}

pub fn layout(req: &PieRequest) -> PieLayout {
    let colors: Vec<Rgba> = req
        .slices
        .iter()
        .enumerate()
        .map(|(i, s)| resolve_color(s.color, i, req.palette))
        .collect();
    let center = (req.width / 2.0, req.height / 2.0);
    let radius = (req.width.min(req.height) / 2.0 - req.hover_offset).max(0.0);
    let inner_radius = radius * req.cutout.clamp(0.0, 0.95);
    let shown = |s: &Slice| !s.hidden && s.value.is_finite() && s.value > 0.0;
    let total: f64 = req
        .slices
        .iter()
        .filter(|s| shown(s))
        .map(|s| s.value)
        .sum();

    let mut slices = Vec::new();
    let (mut rings, mut grid) = (Vec::new(), String::new());
    let mut angle = 0.0;
    if req.polar {
        let shown: Vec<usize> = (0..req.slices.len())
            .filter(|&i| shown(&req.slices[i]))
            .collect();
        let max = shown
            .iter()
            .map(|&i| req.slices[i].value)
            .fold(0.0, f64::max);
        let max_ticks = (radius / (req.font_size.max(1.0) * 2.5))
            .floor()
            .clamp(2.0, 8.0) as usize;
        let scale = NiceScale::new(0.0, if max > 0.0 { max } else { 1.0 }, max_ticks);
        let r_of = |v: f64| (v / scale.max) as f32 * radius;
        for v in scale.ticks().into_iter().filter(|&v| v > 0.0) {
            let r = r_of(v);
            let (top, bottom) = ((center.0, center.1 - r), (center.0, center.1 + r));
            let _ = write!(
                grid,
                "M {:.2} {:.2} A {r:.2} {r:.2} 0 0 1 {:.2} {:.2} A {r:.2} {r:.2} 0 0 1 {:.2} {:.2} ",
                top.0, top.1, bottom.0, bottom.1, top.0, top.1
            );
            rings.push(Tick {
                pos: r,
                label: format_tick(v, scale.step),
            });
        }
        let sweep = TAU / shown.len().max(1) as f32;
        let progress = req.progress.clamp(0.0, 1.0);
        for &index in &shown {
            let outer = r_of(req.slices[index].value) * progress;
            let end = angle + sweep;
            slices.push(SliceGeometry {
                index,
                path: slice_path(center, outer, 0.0, angle, end),
                color: colors[index],
                start: angle,
                end,
                fraction: (req.slices[index].value / total) as f32,
                outer,
            });
            angle = end;
        }
    } else if total > 0.0 {
        let sweep_total = TAU * req.progress.clamp(0.0, 1.0);
        for (index, slice) in req.slices.iter().enumerate() {
            if !shown(slice) {
                continue;
            }
            let fraction = (slice.value / total) as f32;
            let end = angle + fraction * sweep_total;
            slices.push(SliceGeometry {
                index,
                path: slice_path(center, radius, inner_radius, angle, end),
                color: colors[index],
                start: angle,
                end,
                fraction,
                outer: radius,
            });
            angle = end;
        }
    }
    PieLayout {
        slices,
        colors,
        total,
        center,
        radius,
        inner_radius,
        rings,
        grid,
    }
}

/// Returns the input index of the slice at `(x, y)`, if any.
pub fn hit(layout: &PieLayout, x: f32, y: f32) -> Option<usize> {
    let (dx, dy) = (x - layout.center.0, y - layout.center.1);
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < layout.inner_radius {
        return None;
    }
    let angle = dx.atan2(-dy).rem_euclid(TAU);
    layout
        .slices
        .iter()
        .find(|s| angle >= s.start && angle < s.end && distance <= s.outer)
        .map(|s| s.index)
}

pub(crate) fn point(center: (f32, f32), r: f32, angle: f32) -> (f32, f32) {
    (center.0 + r * angle.sin(), center.1 - r * angle.cos())
}

/// The bounding box `(x, y, width, height)` of a ring sector.
pub(crate) fn sector_bounds(
    center: (f32, f32),
    inner: f32,
    outer: f32,
    start: f32,
    end: f32,
) -> (f32, f32, f32, f32) {
    let mut points = vec![
        point(center, inner, start),
        point(center, inner, end),
        point(center, outer, start),
        point(center, outer, end),
    ];
    // Where the outer arc reaches furthest up, right, down or left.
    let mut a = (start / (PI / 2.0)).ceil() * (PI / 2.0);
    while a < end {
        points.push(point(center, outer, a));
        a += PI / 2.0;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for (x, y) in points {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    (
        x0.floor(),
        y0.floor(),
        (x1.ceil() - x0.floor()).max(0.0),
        (y1.ceil() - y0.floor()).max(0.0),
    )
}

pub(crate) fn slice_path(center: (f32, f32), r: f32, inner: f32, start: f32, end: f32) -> String {
    let mut out = String::new();
    if end - start <= 0.0 {
        return out;
    }
    // An SVG arc can't draw a full circle, so long sweeps use two arcs.
    let arcs: &[(f32, f32)] = if end - start > PI {
        let mid = (start + end) / 2.0;
        &[(start, mid), (mid, end)]
    } else {
        &[(start, end)]
    };
    let full = end - start >= TAU - 1e-4;
    let (x, y) = point(center, r, start);
    if inner > 0.0 {
        let _ = write!(out, "M {x:.2} {y:.2}");
    } else {
        let _ = write!(out, "M {:.2} {:.2} L {x:.2} {y:.2}", center.0, center.1);
    }
    for &(_, to) in arcs {
        let (x, y) = point(center, r, to);
        let _ = write!(out, " A {r:.2} {r:.2} 0 0 1 {x:.2} {y:.2}");
    }
    if inner > 0.0 {
        let (x, y) = point(center, inner, end);
        // A full ring is two separate circles; otherwise connect the arcs.
        let _ = write!(out, "{} {x:.2} {y:.2}", if full { " Z M" } else { " L" });
        for &(from, _) in arcs.iter().rev() {
            let (x, y) = point(center, inner, from);
            let _ = write!(out, " A {inner:.2} {inner:.2} 0 0 0 {x:.2} {y:.2}");
        }
    }
    out.push_str(" Z");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slices(values: &[f64]) -> Vec<Slice> {
        values
            .iter()
            .map(|&value| Slice {
                value,
                ..Default::default()
            })
            .collect()
    }

    fn request(slices: &[Slice], cutout: f32) -> PieRequest<'_> {
        PieRequest {
            polar: false,
            font_size: 12.0,
            palette: &[],
            slices,
            width: 200.0,
            height: 100.0,
            cutout,
            hover_offset: 0.0,
            progress: 1.0,
        }
    }

    #[test]
    fn slices_split_the_circle_by_value() {
        let input = slices(&[1.0, 3.0]);
        let pie = layout(&request(&input, 0.0));
        assert_eq!(pie.radius, 50.0);
        assert_eq!(pie.center, (100.0, 50.0));
        assert_eq!(pie.slices[0].fraction, 0.25);
        assert!((pie.slices[0].end - TAU / 4.0).abs() < 1e-5);
        assert!((pie.slices[1].end - TAU).abs() < 1e-5);
        assert!(
            pie.slices[0]
                .path
                .starts_with("M 100.00 50.00 L 100.00 0.00 A")
        );
    }

    #[test]
    fn hidden_and_non_positive_slices_are_skipped() {
        let mut input = slices(&[1.0, 5.0, 0.0, -2.0, 1.0]);
        input[1].hidden = true;
        let pie = layout(&request(&input, 0.0));
        let indices: Vec<_> = pie.slices.iter().map(|s| s.index).collect();
        assert_eq!(indices, vec![0, 4]);
        assert_eq!(pie.colors.len(), 5);
    }

    #[test]
    fn hit_testing() {
        let input = slices(&[1.0, 1.0]);
        let pie = layout(&request(&input, 0.5));
        assert_eq!(
            hit(&pie, 140.0, 50.0),
            Some(0),
            "right of the center: first slice"
        );
        assert_eq!(
            hit(&pie, 60.0, 50.0),
            Some(1),
            "left of the center: second slice"
        );
        assert_eq!(hit(&pie, 100.0, 50.0), None, "inside the hole");
        assert_eq!(hit(&pie, 5.0, 5.0), None, "outside");
    }

    #[test]
    fn polar_slices_have_equal_angles_and_scaled_radii() {
        let input = slices(&[10.0, 5.0, 2.0, 8.0]);
        let mut req = request(&input, 0.0);
        req.polar = true;
        let pie = layout(&req);
        for s in &pie.slices {
            assert!((s.end - s.start - TAU / 4.0).abs() < 1e-5);
        }
        assert_eq!(
            pie.slices[0].outer, 50.0,
            "the largest value reaches the scale maximum"
        );
        assert_eq!(pie.slices[1].outer, 25.0);
        assert_eq!(
            pie.rings.last().map(|t| (t.pos, t.label.as_str())),
            Some((50.0, "10"))
        );
        assert_eq!(hit(&pie, 110.0, 30.0), Some(0), "inside the first slice");
        assert_eq!(
            hit(&pie, 120.0, 70.0),
            None,
            "beyond the 25px of the second slice"
        );
    }

    #[test]
    fn full_ring_is_two_circles() {
        let input = slices(&[1.0]);
        let path = &layout(&request(&input, 0.5)).slices[0].path;
        assert_eq!(path.matches(" A ").count(), 4);
        assert!(path.contains(" Z M "));
    }
}
