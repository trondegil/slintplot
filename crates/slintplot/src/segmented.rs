//! Geometry for segmented ring charts: concentric rings cut into short
//! segments, each segment a unit of the total. The items fill the segments
//! in turn from the start angle, sweeping round across all the rings, or
//! ring by ring from the outside; the segments past the last item are left
//! empty, as a faint track.

use std::f32::consts::TAU;

use crate::pie::{point, slice_path};

pub struct SegmentedRequest<'a> {
    /// The values, NaN where missing.
    pub values: &'a [f64],
    /// Items left out, such as those hidden from the legend.
    pub hidden: &'a [bool],
    pub width: f32,
    pub height: f32,
    /// Where the segments start, in degrees clockwise from 12 o'clock, and
    /// which way they go.
    pub start_angle: f32,
    pub clockwise: bool,
    /// Fill each ring before the next, from the outside, rather than sweep
    /// round across all of them.
    pub by_ring: bool,
    /// The number of rings, and of segments in the outer ring; inner rings
    /// have fewer, in proportion to their size.
    pub rings: i32,
    pub segments: i32,
    /// The value of all segments together; 0 or less for the total of the
    /// values, so that they fill every segment.
    pub max: f64,
    /// The empty middle, as a share of the radius; the gap between rings, as
    /// a share of their width; and the gap between segments, as a share of
    /// the angle each takes.
    pub cutout: f32,
    pub ring_gap: f32,
    pub segment_gap: f32,
    /// Leave room for a dot at the start, outside the outer ring.
    pub marker: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ring {
    pub outer: f32,
    pub inner: f32,
    /// Its number of segments, and the place of the first in the fill
    /// order.
    pub count: usize,
    pub first: usize,
}

/// The segments an item fills, `from` up to `to`, in the fill order.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Span {
    pub index: usize,
    pub from: usize,
    pub to: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Segmented {
    pub center: (f32, f32),
    /// The outside of the outer ring, and the inside of the inner one.
    pub radius: f32,
    pub inner_radius: f32,
    /// In radians clockwise from 12 o'clock.
    pub start: f32,
    /// 1 clockwise, -1 counterclockwise.
    pub direction: f32,
    /// The gap between segments, as a share of the angle each takes.
    pub segment_gap: f32,
    pub by_ring: bool,
    /// The rings, outermost first.
    pub rings: Vec<Ring>,
    /// The visible items, in fill order.
    pub spans: Vec<Span>,
    /// The number of segments, and the value of each.
    pub capacity: usize,
    pub unit: f64,
    /// The total of the visible values.
    pub total: f64,
    /// The dot at the start: its center and diameter (0 for none).
    pub marker: (f32, f32, f32),
}

fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() { v } else { fallback }
}

pub fn layout(req: &SegmentedRequest) -> Segmented {
    let size = |v: f32| finite_or(v, 0.0).max(0.0);
    let (width, height) = (size(req.width), size(req.height));
    let start = finite_or(req.start_angle, 0.0)
        .rem_euclid(360.0)
        .to_radians();
    let direction = if req.clockwise { 1.0 } else { -1.0 };
    let cutout = finite_or(req.cutout, 0.45).clamp(0.0, 0.9);
    let ring_gap = finite_or(req.ring_gap, 0.5).clamp(0.0, 3.0);
    let segment_gap = finite_or(req.segment_gap, 0.3).clamp(0.0, 0.9);
    let n = req.rings.clamp(1, 20) as usize;
    let outer_count = req.segments.clamp(3, 500) as usize;

    // The ring width as a share of the radius; the marker takes a little
    // more than a ring outside it.
    let share = (1.0 - cutout) / (n as f32 + (n as f32 - 1.0) * ring_gap);
    let margin = if req.marker { 1.2 } else { 0.0 };
    let half = width.min(height) / 2.0;
    let radius = half / (1.0 + margin * share);
    let w = radius * share;
    let center = (width / 2.0, height / 2.0);

    // Inner rings keep the segments' length: fewer of them, by radius.
    let mid0 = radius - w / 2.0;
    let mut rings = Vec::with_capacity(n);
    let mut first = 0;
    for k in 0..n {
        let outer = radius - k as f32 * w * (1.0 + ring_gap);
        let mid = outer - w / 2.0;
        let count = if mid0 > 0.0 {
            ((outer_count as f32 * mid / mid0).round() as usize).max(3)
        } else {
            outer_count
        };
        rings.push(Ring {
            outer,
            inner: (outer - w).max(0.0),
            count,
            first,
        });
        first += count;
    }
    let capacity = first;

    let shown: Vec<usize> = (0..req.values.len())
        .filter(|&i| !req.hidden.get(i).copied().unwrap_or(false))
        .collect();
    let value = |i: usize| {
        let v = req.values[i];
        if v.is_finite() && v > 0.0 { v } else { 0.0 }
    };
    let total: f64 = shown.iter().map(|&i| value(i)).sum();
    let total = if total.is_finite() { total } else { f64::MAX };
    let max = if req.max > 0.0 && req.max.is_finite() {
        req.max
    } else {
        total
    };
    let unit = if max > 0.0 {
        max / capacity as f64
    } else {
        1.0
    };
    let unit = if unit > 0.0 && unit.is_finite() {
        unit
    } else {
        1.0
    };

    // Rounding the running total keeps the segments of all the items
    // together as close to the total as the units allow.
    let mut sum = 0.0;
    let mut from = 0;
    let spans = shown
        .iter()
        .map(|&index| {
            sum += value(index);
            let to = ((sum / unit).round().min(capacity as f64) as usize).max(from);
            let span = Span { index, from, to };
            from = to;
            span
        })
        .collect();

    let marker = if req.marker && radius > 0.0 {
        let d = w * 0.7;
        let (x, y) = point(center, radius + w * 0.45 + d / 2.0, start);
        (x, y, d)
    } else {
        (center.0, center.1, 0.0)
    };

    Segmented {
        center,
        radius,
        inner_radius: rings.last().map_or(0.0, |r| r.inner),
        start,
        direction,
        segment_gap,
        by_ring: req.by_ring,
        rings,
        spans,
        capacity,
        unit,
        total,
        marker,
    }
}

/// The angles of a ring's segment `slot`, the smaller first.
fn segment_angles(start: f32, direction: f32, count: usize, slot: usize, gap: f32) -> (f32, f32) {
    let pitch = TAU / count as f32;
    let a = start + direction * slot as f32 * pitch;
    let b = start + direction * (slot + 1) as f32 * pitch;
    let (lo, hi) = (a.min(b), a.max(b));
    let trim = pitch * gap / 2.0;
    (lo + trim, hi - trim)
}

/// The segments in the fill order, as (ring, slot): ring by ring, or by
/// the angle of their middles, the outer ring first at the same angle.
pub fn order(rings: &[Ring], by_ring: bool) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = rings
        .iter()
        .enumerate()
        .flat_map(|(k, r)| (0..r.count).map(move |slot| (k, slot)))
        .collect();
    if !by_ring {
        // Compare (slot + 0.5) / count exactly, in integers.
        out.sort_by(|&(ka, sa), &(kb, sb)| {
            let (ca, cb) = (rings[ka].count as u64, rings[kb].count as u64);
            ((2 * sa as u64 + 1) * cb)
                .cmp(&((2 * sb as u64 + 1) * ca))
                .then(ka.cmp(&kb))
        });
    }
    out
}

/// The segments from `from` up to `to` in the fill order, as one path.
/// Fractions are rounded, so that animated ends step from segment to
/// segment.
pub fn path(chart: &Segmented, from: f32, to: f32) -> String {
    let mut out = String::new();
    let Segmented {
        center,
        start,
        direction,
        segment_gap: gap,
        ref rings,
        by_ring,
        ..
    } = *chart;
    let all = [center.0, center.1, start, direction, gap, from, to];
    if all.iter().any(|v| !v.is_finite()) {
        return out;
    }
    let order = order(rings, by_ring);
    let from = (from.round().max(0.0) as usize).min(order.len());
    let to = (to.round().max(0.0) as usize).min(order.len());
    for &(k, slot) in order.get(from..to).unwrap_or_default() {
        let r = &rings[k];
        if r.outer <= r.inner {
            continue;
        }
        let (a, b) = segment_angles(start, direction, r.count, slot, gap);
        if b > a {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&slice_path(center, r.outer, r.inner, a, b));
        }
    }
    out
}

/// The place in the fill order of the segment at `(x, y)`, gaps included.
pub fn segment_at(chart: &Segmented, x: f32, y: f32) -> Option<usize> {
    let (dx, dy) = (x - chart.center.0, y - chart.center.1);
    let distance = (dx * dx + dy * dy).sqrt();
    // Half a gap around each ring belongs to it.
    let k = chart.rings.iter().position(|r| {
        let pad = (r.outer - r.inner) * 0.25;
        distance >= r.inner - pad && distance <= r.outer + pad
    })?;
    let ring = &chart.rings[k];
    let along = ((dx.atan2(-dy) - chart.start) * chart.direction).rem_euclid(TAU);
    let slot = ((along / TAU * ring.count as f32) as usize).min(ring.count.max(1) - 1);
    if chart.by_ring {
        Some(ring.first + slot)
    } else {
        order(&chart.rings, false)
            .iter()
            .position(|&s| s == (k, slot))
    }
}

/// The item whose segment is at `(x, y)`.
pub fn hit(chart: &Segmented, x: f32, y: f32) -> Option<usize> {
    let k = segment_at(chart, x, y)?;
    chart
        .spans
        .iter()
        .find(|s| k >= s.from && k < s.to)
        .map(|s| s.index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>(values: &'a [f64], hidden: &'a [bool]) -> SegmentedRequest<'a> {
        SegmentedRequest {
            values,
            hidden,
            width: 400.0,
            height: 300.0,
            start_angle: 0.0,
            clockwise: true,
            by_ring: true,
            rings: 5,
            segments: 60,
            max: 0.0,
            cutout: 0.45,
            ring_gap: 0.5,
            segment_gap: 0.3,
            marker: false,
        }
    }

    #[test]
    fn rings_get_fewer_segments_inwards() {
        let chart = layout(&request(&[1.0], &[]));
        assert_eq!(chart.radius, 150.0);
        assert_eq!(chart.center, (200.0, 150.0));
        assert_eq!(chart.rings.len(), 5);
        assert_eq!(chart.rings[0].count, 60);
        for pair in chart.rings.windows(2) {
            assert!(pair[1].count < pair[0].count);
            assert!(pair[1].outer < pair[0].inner);
            assert_eq!(pair[1].first, pair[0].first + pair[0].count);
        }
        assert!((chart.inner_radius - 150.0 * 0.45).abs() < 1e-3);
        let last = chart.rings[4];
        assert_eq!(chart.capacity, last.first + last.count);
    }

    #[test]
    fn items_fill_the_segments_in_turn() {
        let mut req = request(&[300.0, 100.0, 0.0, 50.0], &[false, false, false, true]);
        req.max = 0.0;
        let chart = layout(&req);
        assert_eq!(chart.total, 400.0);
        let c = chart.capacity;
        assert_eq!(chart.spans.len(), 3, "the hidden item is left out");
        assert_eq!(chart.spans[0].from, 0);
        assert_eq!(chart.spans[0].to, (c as f64 * 0.75).round() as usize);
        assert_eq!(chart.spans[1].to, c, "the values fill every segment");
        assert_eq!(chart.spans[2].from, chart.spans[2].to);

        req.max = 800.0;
        let chart = layout(&req);
        assert_eq!(chart.spans[1].to, (c as f64 / 2.0).round() as usize);
        req.max = 100.0;
        let chart = layout(&req);
        assert_eq!(chart.spans[0].to, c, "past the maximum fills every segment");
        assert_eq!(chart.spans[1].from, c);
    }

    #[test]
    fn segments_go_both_ways_and_are_hit() {
        let values = [10.0, 10.0];
        let mut req = request(&values, &[]);
        req.max = 0.0;
        let chart = layout(&req);
        let (cx, cy) = chart.center;
        let mid = (chart.rings[0].outer + chart.rings[0].inner) / 2.0;
        // Just clockwise of 12 o'clock is the first segment.
        let (x, y) = point(chart.center, mid, 0.05);
        assert_eq!(segment_at(&chart, x, y), Some(0));
        assert_eq!(hit(&chart, x, y), Some(0));
        let (x, y) = point(chart.center, mid, -0.05);
        assert_eq!(segment_at(&chart, x, y), Some(59));
        assert_eq!(hit(&chart, cx, cy), None, "the middle");

        req.clockwise = false;
        let chart = layout(&req);
        let (x, y) = point(chart.center, mid, -0.05);
        assert_eq!(segment_at(&chart, x, y), Some(0));
        let p = path(&chart, 0.0, 1.0);
        assert_eq!(p.matches('M').count(), 1);
        assert!(p.starts_with("M "), "{p}");
        // The segment is left of 12 o'clock.
        let x: f32 = p[2..].split(' ').next().unwrap().parse().unwrap();
        assert!(x < cx, "{p}");
    }

    #[test]
    fn paths_round_to_whole_segments() {
        let chart = layout(&request(&[1.0], &[]));
        let p = |from, to| path(&chart, from, to);
        assert!(p(0.0, 0.0).is_empty());
        assert_eq!(p(0.0, 3.4).matches('M').count(), 3);
        assert_eq!(p(58.0, 62.0).matches('M').count(), 4, "across two rings");
        assert_eq!(p(0.0, 1e9).matches('M').count(), chart.capacity);
        assert!(p(f32::NAN, 3.0).is_empty());
    }

    #[test]
    fn sweeping_fill_goes_round_across_the_rings() {
        let mut req = request(&[1.0], &[]);
        req.by_ring = false;
        let chart = layout(&req);
        let order = order(&chart.rings, false);
        assert_eq!(order.len(), chart.capacity);
        // Every ring starts at the start, outermost first.
        let starts: Vec<_> = order[..5].to_vec();
        assert_eq!(starts, [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]);
        // The outer ring's last segment ends nearest the start, so comes last.
        assert_eq!(order.last(), Some(&(0, 59)));
        // The hit test agrees with the order.
        let r = chart.rings[2];
        let mid = (r.outer + r.inner) / 2.0;
        let (x, y) = point(chart.center, mid, 0.01);
        assert_eq!(segment_at(&chart, x, y), Some(2));
        let (x, y) = point(chart.center, mid, std::f32::consts::PI);
        let k = segment_at(&chart, x, y).unwrap();
        assert_eq!(order[k], (2, r.count / 2));
    }

    #[test]
    fn marker_sits_outside_at_the_start() {
        let mut req = request(&[1.0], &[]);
        req.marker = true;
        req.start_angle = 90.0;
        let chart = layout(&req);
        let (x, y, d) = chart.marker;
        assert!(d > 0.0);
        assert!((y - chart.center.1).abs() < 1e-3);
        assert!(x - d / 2.0 > chart.center.0 + chart.radius);
        assert!(x + d / 2.0 <= 400.0 && chart.radius < 150.0);
    }

    #[test]
    fn bad_input_gives_finite_shapes() {
        let values = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -5.0, 1e300, 0.0];
        for (max, start, cutout, size, rings, segments) in [
            (0.0, 0.0, 0.45, 300.0, 5, 60),
            (f64::NAN, f32::NAN, f32::NAN, f32::NAN, 0, 0),
            (-1.0, f32::INFINITY, 5.0, 0.0, -3, -10),
            (f64::INFINITY, 1e30, -1.0, -10.0, 1000, 1_000_000),
            (1e-300, -720.0, 0.9, 1e7, 1, 3),
        ] {
            let mut req = request(&values, &[]);
            (req.max, req.start_angle, req.cutout) = (max, start, cutout);
            (req.width, req.height, req.rings, req.segments) = (size, size, rings, segments);
            req.marker = true;
            let chart = layout(&req);
            assert!(chart.radius.is_finite() && chart.unit > 0.0 && chart.unit.is_finite());
            assert!(chart.capacity <= 20 * 500);
            for s in &chart.spans {
                assert!(s.from <= s.to && s.to <= chart.capacity);
            }
            let mut swept = chart.clone();
            swept.by_ring = false;
            let p = path(&swept, 0.0, chart.capacity as f32);
            assert!(!p.contains("NaN") && !p.contains("inf"));
            let _ = hit(&chart, 1.0, 1.0);
        }
        assert!(layout(&request(&[], &[])).spans.is_empty());
    }
}
