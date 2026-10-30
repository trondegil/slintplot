//! Geometry for radial bar charts: one ring per item, the first outermost,
//! each filled along its arc in proportion to its value, like activity
//! rings. Each ring's label goes before its start, in the opening that a
//! sweep of less than a full turn leaves.

use std::f32::consts::{PI, TAU};

use crate::gauge::rounded_arc;
use crate::layout::Label;
use crate::pie::{point, slice_path};

pub struct RadialBarRequest<'a> {
    /// The values, NaN where missing.
    pub values: &'a [f64],
    /// Items left out, such as those hidden from the legend.
    pub hidden: &'a [bool],
    pub width: f32,
    pub height: f32,
    /// Where the rings start, in degrees clockwise from 12 o'clock, and the
    /// angle they cover.
    pub start_angle: f32,
    pub sweep: f32,
    /// The value of a full ring; 0 or less for the largest value.
    pub max: f64,
    /// The empty middle, as a share of the radius, and the gap between the
    /// rings, as a share of their width.
    pub cutout: f32,
    pub gap: f32,
    /// The size of the widest label; 0 for none.
    pub label_width: f32,
    pub label_height: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ring {
    /// The item's index in the input.
    pub index: usize,
    pub outer: f32,
    pub inner: f32,
    /// The angle the value reaches; the start for none.
    pub end: f32,
    /// Where the label goes, before the start of the ring (without text).
    pub label: Label,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RadialBars {
    pub center: (f32, f32),
    pub radius: f32,
    /// The angles of the rings' ends, in radians clockwise from 12 o'clock.
    pub start: f32,
    pub end: f32,
    /// The value of a full ring.
    pub max: f64,
    /// The rings of the visible items, outermost first.
    pub rings: Vec<Ring>,
}

/// The width of each of `count` rings in a circle of `radius`. Few rings
/// stay narrow, at the outside.
fn ring_width(radius: f32, count: usize, cutout: f32, gap: f32) -> f32 {
    let n = count.max(1) as f32;
    (radius * (1.0 - cutout) / (n + (n - 1.0) * gap)).min(radius * 0.2)
}

/// The anchor and alignment of a label before the start of a ring: back
/// along the ring's direction, so that a ring starting at 12 o'clock has its
/// label to the left, right-aligned.
fn label_at(center: (f32, f32), mid: f32, start: f32, pad: f32) -> Label {
    let (tx, ty) = (start.cos(), start.sin());
    let (x, y) = point(center, mid, start);
    Label {
        x: x - tx * pad,
        y: y - ty * pad,
        text: String::new(),
        align: (1.0 + tx) / 2.0,
        valign: (1.0 + ty) / 2.0,
    }
}

pub fn layout(req: &RadialBarRequest) -> RadialBars {
    // Unusable settings fall back to the defaults, so that no angle or
    // radius is NaN.
    let sweep = if req.sweep.is_finite() {
        req.sweep.clamp(10.0, 360.0)
    } else {
        270.0
    }
    .to_radians();
    let start = if req.start_angle.is_finite() {
        req.start_angle.rem_euclid(360.0)
    } else {
        0.0
    }
    .to_radians();
    let cutout = if req.cutout.is_finite() {
        req.cutout.clamp(0.0, 0.9)
    } else {
        0.25
    };
    let gap = if req.gap.is_finite() {
        req.gap.clamp(0.0, 2.0)
    } else {
        0.25
    };
    let size = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
    let (width, height) = (size(req.width), size(req.height));
    let (label_w, label_h) = (size(req.label_width), size(req.label_height));
    let labels = label_w > 0.0 && label_h > 0.0;
    let pad = label_h * 0.5;

    let shown: Vec<usize> = (0..req.values.len())
        .filter(|&i| !req.hidden.get(i).copied().unwrap_or(false))
        .collect();
    let count = shown.len();

    // The box around the arc, its center and the labels, for a radius.
    let bounds = |r: f32| {
        let mut points = vec![(0.0, 0.0), point((0.0, 0.0), r, start)];
        points.push(point((0.0, 0.0), r, start + sweep));
        let mut a = (start / (PI / 2.0)).ceil() * (PI / 2.0);
        while a < start + sweep {
            points.push(point((0.0, 0.0), r, a));
            a += PI / 2.0;
        }
        if labels && count > 0 {
            let w = ring_width(r, count, cutout, gap);
            for k in 0..count {
                let mid = r - k as f32 * w * (1.0 + gap) - w / 2.0;
                let l = label_at((0.0, 0.0), mid, start, pad);
                let x = l.x - l.align * label_w;
                let y = l.y - l.valign * label_h;
                points.push((x, y));
                points.push((x + label_w, y + label_h));
            }
        }
        points.iter().fold(
            (
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ),
            |(x0, y0, x1, y1), &(x, y)| (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
        )
    };
    let fits = |r: f32| {
        let (x0, y0, x1, y1) = bounds(r);
        x1 - x0 <= width && y1 - y0 <= height
    };
    // The box grows with the radius: bisect for the largest that fits.
    let (mut lo, mut hi) = (0.0f32, width.max(height));
    if fits(hi) {
        lo = hi;
    } else {
        for _ in 0..40 {
            let mid = (lo + hi) / 2.0;
            if fits(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
    }
    let radius = lo;
    let (x0, y0, x1, y1) = bounds(radius);
    let center = (
        width / 2.0 - (x0 + x1) / 2.0,
        height / 2.0 - (y0 + y1) / 2.0,
    );

    // The value of a full ring: the fixed one, or the largest value.
    let max = if req.max > 0.0 && req.max.is_finite() {
        req.max
    } else {
        shown
            .iter()
            .map(|&i| req.values[i])
            .filter(|v| v.is_finite() && *v > 0.0)
            .fold(0.0, f64::max)
    };
    let max = if max > 0.0 { max } else { 1.0 };

    let w = ring_width(radius, count, cutout, gap);
    let rings = shown
        .iter()
        .enumerate()
        .map(|(k, &index)| {
            let outer = radius - k as f32 * w * (1.0 + gap);
            let f = (req.values[index] / max).clamp(0.0, 1.0);
            let f = if f.is_nan() { 0.0 } else { f as f32 };
            Ring {
                index,
                outer,
                inner: outer - w,
                end: start + f * sweep,
                label: label_at(center, outer - w / 2.0, start, pad),
            }
        })
        .collect();
    RadialBars {
        center,
        radius,
        start,
        end: start + sweep,
        max,
        rings,
    }
}

/// A ring from `start` to `end`, with round ends if `rounded`; a full turn
/// is a closed ring.
pub fn bar_path(
    center: (f32, f32),
    outer: f32,
    inner: f32,
    start: f32,
    end: f32,
    rounded: bool,
) -> String {
    let all = [center.0, center.1, outer, inner, start, end];
    if all.iter().any(|v| !v.is_finite()) || end <= start || outer <= inner {
        return String::new();
    }
    let inner = inner.max(0.0);
    if end - start >= TAU - 1e-4 {
        slice_path(center, outer, inner, start, start + TAU)
    } else if rounded {
        rounded_arc(center, outer, inner, start, end)
    } else {
        slice_path(center, outer, inner, start, end)
    }
}

/// Where a value label of `size` (width, height) goes on a ring from
/// `inner` to `outer` whose bar ends at `end`: its center, and whether it is inside the bar. It goes just
/// past the end, on the track up to `limit`, or else inside the end of the
/// bar if it fits across it, or else past the end of the track, in the
/// opening. `None` when the ring is too thin for it, or full without an
/// opening.
pub fn value_spot(
    center: (f32, f32),
    (outer, inner): (f32, f32),
    start: f32,
    end: f32,
    limit: f32,
    (width, height): (f32, f32),
) -> Option<(f32, f32, bool)> {
    let all = [
        center.0, center.1, outer, inner, start, end, limit, width, height,
    ];
    if all.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mid = (outer + inner) / 2.0;
    let thickness = outer - inner;
    if mid <= 0.0 || thickness < height * 0.7 {
        return None;
    }
    // Half the label's extent along the ring at angle `a`, as an angle.
    let half = |a: f32| (a.cos().abs() * width + a.sin().abs() * height) / 2.0 / mid;
    let pad = height * 0.4 / mid;
    let after = end + pad + half(end);
    let after = end + pad + half(after);
    if after + half(after) <= limit {
        let (x, y) = point(center, mid, after);
        return Some((x, y, false));
    }
    let pad = thickness * 0.45 / mid;
    let before = end - pad - half(end);
    let before = end - pad - half(before);
    // The label's extent across the ring.
    let across = before.sin().abs() * width + before.cos().abs() * height;
    if before - half(before) >= start && across <= thickness {
        let (x, y) = point(center, mid, before);
        return Some((x, y, true));
    }
    (limit - start < TAU - 0.1).then(|| {
        let (x, y) = point(center, mid, after);
        (x, y, false)
    })
}

/// The item at `(x, y)`: on its ring, anywhere along the sweep, or on its
/// label of at most `label_width` × `label_height`.
pub fn hit(
    bars: &RadialBars,
    x: f32,
    y: f32,
    label_width: f32,
    label_height: f32,
) -> Option<usize> {
    let (dx, dy) = (x - bars.center.0, y - bars.center.1);
    let distance = (dx * dx + dy * dy).sqrt();
    let along = (dx.atan2(-dy) - bars.start).rem_euclid(TAU);
    let on_sweep = along <= bars.end - bars.start + 1e-4;
    bars.rings
        .iter()
        .find(|r| {
            let on_ring = on_sweep && distance >= r.inner && distance <= r.outer;
            let l = &r.label;
            let (lx, ly) = (l.x - l.align * label_width, l.y - l.valign * label_height);
            let on_label = x >= lx && x <= lx + label_width && y >= ly && y <= ly + label_height;
            on_ring || on_label
        })
        .map(|r| r.index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>(values: &'a [f64], hidden: &'a [bool]) -> RadialBarRequest<'a> {
        RadialBarRequest {
            values,
            hidden,
            width: 400.0,
            height: 300.0,
            start_angle: 0.0,
            sweep: 270.0,
            max: 0.0,
            cutout: 0.25,
            gap: 0.25,
            label_width: 0.0,
            label_height: 0.0,
        }
    }

    #[test]
    fn rings_fill_from_the_outside() {
        let bars = layout(&request(&[100.0, 50.0, 25.0, 0.0], &[]));
        assert!((bars.radius - 150.0).abs() < 1e-3, "fits the height");
        assert!((bars.center.0 - 200.0).abs() < 1e-3 && (bars.center.1 - 150.0).abs() < 1e-3);
        assert_eq!(bars.max, 100.0, "the largest value");
        assert_eq!(bars.rings.len(), 4);
        let r = &bars.rings;
        assert_eq!(r[0].outer, bars.radius);
        // 112.5 px for four rings and three gaps of a quarter ring.
        assert!((r[0].outer - r[0].inner - 112.5 / 4.75).abs() < 1e-3);
        assert!(r[1].outer < r[0].inner && (r[3].inner - 150.0 * 0.25).abs() < 1e-3);
        let full = 1.5 * PI;
        assert!((r[0].end - full).abs() < 1e-5);
        assert!((r[1].end - full / 2.0).abs() < 1e-5);
        assert_eq!(r[3].end, 0.0, "nothing filled for 0");
    }

    #[test]
    fn fixed_maximum_and_hidden_items() {
        let mut req = request(&[30.0, 90.0, 120.0], &[false, true, false]);
        req.max = 60.0;
        let bars = layout(&req);
        assert_eq!(bars.max, 60.0);
        let indices: Vec<_> = bars.rings.iter().map(|r| r.index).collect();
        assert_eq!(indices, [0, 2]);
        assert!((bars.rings[0].end - 0.75 * PI).abs() < 1e-5);
        assert!(
            (bars.rings[1].end - 1.5 * PI).abs() < 1e-5,
            "values past the maximum fill the ring"
        );
        // Two rings stay narrow rather than filling the whole radius.
        let r = &bars.rings[0];
        assert_eq!(r.outer - r.inner, bars.radius * 0.2);
    }

    #[test]
    fn labels_go_left_of_the_start() {
        let mut req = request(&[3.0, 2.0, 1.0], &[]);
        req.label_width = 260.0;
        req.label_height = 14.0;
        let bars = layout(&req);
        let l = &bars.rings[0].label;
        assert_eq!((l.align, l.valign), (1.0, 0.5));
        assert!((l.x - (bars.center.0 - 7.0)).abs() < 1e-3);
        assert!(l.x - 260.0 >= -1e-3, "the labels fit: {}", l.x);
        assert!(bars.radius < 150.0, "the circle made room for them");
        // A ring and its label are both the item.
        let (cx, cy) = bars.center;
        assert_eq!(hit(&bars, cx + bars.radius - 2.0, cy, 260.0, 14.0), Some(0));
        assert_eq!(hit(&bars, l.x - 100.0, l.y, 260.0, 14.0), Some(0));
        assert_eq!(hit(&bars, cx - 5.0, cy + 5.0, 260.0, 14.0), None);
        let inner = &bars.rings[2];
        let mid = (inner.outer + inner.inner) / 2.0;
        assert_eq!(
            hit(&bars, cx - mid, cy, 0.0, 0.0),
            Some(2),
            "at the end of the sweep"
        );
    }

    #[test]
    fn paths_and_value_labels() {
        let c = (100.0, 100.0);
        assert!(bar_path(c, 90.0, 70.0, 0.0, 0.0, true).is_empty());
        assert_eq!(
            bar_path(c, 90.0, 70.0, 0.0, 2.0, true)
                .matches(" A ")
                .count(),
            4
        );
        assert_eq!(
            bar_path(c, 90.0, 70.0, 0.0, 2.0, false)
                .matches(" A ")
                .count(),
            2
        );
        let ring = bar_path(c, 90.0, 70.0, 1.0, 1.0 + TAU, true);
        assert!(
            ring.contains(" Z M"),
            "a full turn is a closed ring: {ring}"
        );
        // Past the end on the track, then inside the bar once it is nearly full.
        let (_, _, inside) = value_spot(c, (90.0, 70.0), 0.0, 1.0, 4.0, (30.0, 14.0)).unwrap();
        assert!(!inside);
        let (x, y, inside) = value_spot(c, (95.0, 65.0), 0.0, 3.9, 4.0, (30.0, 14.0)).unwrap();
        assert!(inside && x.is_finite() && y.is_finite());
        // Too wide to fit across the bar: past the end of the track.
        let (x, y, inside) = value_spot(c, (90.0, 70.0), 0.0, 3.9, 4.0, (30.0, 14.0)).unwrap();
        assert!(!inside && (y - c.1).atan2(x - c.0) + PI / 2.0 > 4.0);
        assert_eq!(
            value_spot(c, (90.0, 70.0), 0.0, TAU, TAU, (30.0, 14.0)),
            None,
            "a full turn has no opening"
        );
        assert_eq!(
            value_spot(c, (90.0, 85.0), 0.0, 1.0, 4.0, (30.0, 14.0)),
            None,
            "too thin"
        );
    }

    #[test]
    fn bad_input_gives_finite_shapes() {
        let values = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -5.0, 1e300, 0.0];
        for (max, sweep, start, cutout, size) in [
            (0.0, 270.0, 0.0, 0.25, 300.0),
            (f64::NAN, f32::NAN, f32::NAN, f32::NAN, f32::NAN),
            (-1.0, f32::INFINITY, f32::NEG_INFINITY, 5.0, 0.0),
            (f64::INFINITY, -90.0, 1e30, -1.0, -10.0),
            (1e-300, 1e9, -720.0, 0.9, 1e7),
        ] {
            let mut req = request(&values, &[]);
            (req.max, req.sweep, req.start_angle, req.cutout) = (max, sweep, start, cutout);
            (req.width, req.height, req.label_width, req.label_height) = (size, size, size, 14.0);
            let bars = layout(&req);
            assert!(
                bars.radius.is_finite() && bars.center.0.is_finite() && bars.center.1.is_finite()
            );
            assert!(bars.max > 0.0 && bars.max.is_finite());
            for r in &bars.rings {
                for v in [r.outer, r.inner, r.end, r.label.x, r.label.y] {
                    assert!(v.is_finite(), "{bars:?}");
                }
                let path = bar_path(bars.center, r.outer, r.inner, bars.start, r.end, true);
                assert!(!path.contains("NaN") && !path.contains("inf"), "{path}");
            }
        }
        assert!(layout(&request(&[], &[])).rings.is_empty());
        let none = layout(&request(&[f64::NAN, -1.0], &[]));
        assert_eq!(none.max, 1.0);
        assert!(none.rings.iter().all(|r| r.end == none.start));
    }
}
