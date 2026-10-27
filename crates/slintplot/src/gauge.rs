//! Geometry for gauges: an arc from `min` to `max`, symmetric around 12
//! o'clock, in one of two styles, after common gauge libraries:
//!
//! - Fill (ApexCharts, Highcharts solid gauges): a thick track, filled up to
//!   the value, with rounded ends. Bands are a thin ring outside the track,
//!   like Grafana's threshold markers, and the fill takes the color of the
//!   band the value is in.
//! - Needle (ECharts, dashboards): the bands as a thinner arc, with major and
//!   minor ticks and their labels inside it, and a needle pointing at the
//!   value.

use std::f32::consts::PI;
use std::fmt::Write;

use crate::pie::{point, slice_path};
use crate::ticks::{Format, NiceScale};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Band {
    /// The value where the band ends; it starts where the previous one ends.
    pub to: f64,
}

pub struct GaugeRequest<'a> {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub bands: &'a [Band],
    pub width: f32,
    pub height: f32,
    /// The size of the inside of the arc, as a share of the radius.
    pub cutout: f32,
    /// The angle the arc covers, in degrees.
    pub sweep: f32,
    /// Space needed below the end points (for the min and max labels), and
    /// below the center (for a needle's hub and the value).
    pub below_ends: f32,
    pub below_center: f32,
    pub needle: bool,
    /// Fill style: round the ends of the track and the fill.
    pub rounded: bool,
    /// Needle style: tick marks and labels inside the arc.
    pub ticks: bool,
    pub font_size: f32,
    pub format: &'a Format,
}

/// A tick label: its center, and its text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickLabel {
    pub x: f32,
    pub y: f32,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BandArc {
    pub index: usize,
    pub path: String,
    pub start: f32,
    pub end: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gauge {
    pub center: (f32, f32),
    pub radius: f32,
    pub inner: f32,
    /// The whole arc, the arc up to the value, and the bands.
    pub track: String,
    pub fill: String,
    pub bands: Vec<BandArc>,
    pub needle: String,
    /// Needle style: the tick marks, as one path, and their labels.
    pub ticks: String,
    pub tick_labels: Vec<TickLabel>,
    /// The band that holds the value, if any.
    pub value_band: Option<usize>,
    /// The ends of the arc, in the middle of its width.
    pub start_point: (f32, f32),
    pub end_point: (f32, f32),
    pub start: f32,
    pub end: f32,
}

/// The lowest and highest point of a unit arc from `start` to `end`, and its
/// widest x, with the center included.
fn extent(start: f32, end: f32) -> (f32, f32, f32) {
    let mut angles = vec![start, end];
    for a in [-PI, -PI / 2.0, 0.0, PI / 2.0, PI] {
        if a > start && a < end {
            angles.push(a);
        }
    }
    let (mut top, mut bottom, mut wide) = (0.0f32, 0.0f32, 0.0f32);
    for a in angles {
        top = top.min(-a.cos());
        bottom = bottom.max(-a.cos());
        wide = wide.max(a.sin().abs());
    }
    (-top, bottom, wide)
}

pub fn layout(req: &GaugeRequest) -> Gauge {
    let sweep = req.sweep.clamp(30.0, 360.0).to_radians();
    let (start, end) = (-sweep / 2.0, sweep / 2.0);
    let (top, bottom, wide) = extent(start, end);
    let end_y = -start.cos();
    // Fit the height: the arc above the center, and below it whatever reaches
    // furthest: the arc, the end labels or the content below the center.
    let fit = |r: f32| {
        top * r
            + (bottom * r)
                .max(end_y * r + req.below_ends)
                .max(req.below_center)
    };
    let mut radius = (req.width / 2.0 / wide.max(0.5)).max(0.0);
    if fit(radius) > req.height {
        // `fit` is piecewise linear and increasing: bisect.
        let (mut lo, mut hi) = (0.0f32, radius);
        for _ in 0..30 {
            let mid = (lo + hi) / 2.0;
            if fit(mid) > req.height {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        radius = lo;
    }
    let center = (
        req.width / 2.0,
        (req.height - fit(radius)) / 2.0 + top * radius,
    );
    let inner = radius * req.cutout.clamp(0.0, 0.95);
    let thickness = radius - inner;
    let span = req.max - req.min;
    let angle_of = |v: f64| {
        let f = if span > 0.0 {
            ((v - req.min) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        start + f as f32 * sweep
    };
    let value_angle = if req.value.is_finite() {
        angle_of(req.value)
    } else {
        start
    };
    // Fill style with bands: the bands are a thin ring outside the track,
    // which keeps the rest of the width.
    let ring_bands = !req.needle && !req.bands.is_empty();
    let ring = (thickness * 0.2).max(3.0);
    let (band_inner, track_outer) = if ring_bands {
        (radius - ring, radius - ring - (thickness * 0.08).max(2.0))
    } else {
        (inner, radius)
    };
    let mut bands = Vec::new();
    let mut from = req.min;
    let mut value_band = None;
    for (index, band) in req.bands.iter().enumerate() {
        let to = band.to.min(req.max);
        if to > from {
            let (a, b) = (angle_of(from), angle_of(to));
            bands.push(BandArc {
                index,
                path: slice_path(center, radius, band_inner, a, b),
                start: a,
                end: b,
            });
            if req.value >= from && (req.value < to || (to >= req.max && req.value <= to)) {
                value_band = Some(index);
            }
            from = to;
        }
    }
    let arc = |a: f32, b: f32| {
        if req.rounded && !req.needle {
            rounded_arc(center, track_outer, inner, a, b)
        } else {
            slice_path(center, track_outer, inner, a, b)
        }
    };
    let (mut ticks, mut tick_labels) = (String::new(), Vec::new());
    if req.needle && req.ticks && span > 0.0 && radius > 0.0 {
        // Major ticks about every 40°, with labels, and minor ticks between
        // them where there is room.
        let count = ((req.sweep / 40.0).round() as usize + 1).clamp(3, 9);
        let scale = NiceScale::new(req.min, req.max, count);
        let major = (thickness * 0.8).clamp(5.0, 14.0);
        let outer = inner - 2.0;
        let values: Vec<f64> = scale
            .ticks()
            .into_iter()
            .filter(|v| *v >= req.min - span * 1e-9 && *v <= req.max + span * 1e-9)
            .collect();
        let room = (scale.step / span) as f32 * sweep * outer;
        let minors = if room > 60.0 {
            5
        } else if room > 24.0 {
            2
        } else {
            1
        };
        let mut tick = |v: f64, length: f32| {
            let a = angle_of(v);
            let (p, q) = (point(center, outer, a), point(center, outer - length, a));
            let _ = write!(ticks, "M {:.2} {:.2} L {:.2} {:.2} ", p.0, p.1, q.0, q.1);
        };
        for (i, &v) in values.iter().enumerate() {
            tick(v, major);
            for m in 1..minors {
                let minor = v + scale.step * m as f64 / minors as f64;
                if i + 1 < values.len() || minor <= req.max {
                    tick(minor, major * 0.5);
                }
            }
        }
        // Minor ticks below the first major one.
        if let Some(&first) = values.first() {
            for m in 1..minors {
                let minor = first - scale.step * m as f64 / minors as f64;
                if minor >= req.min {
                    tick(minor, major * 0.5);
                }
            }
        }
        for v in values {
            let a = angle_of(v);
            let text = req.format.tick(v, scale.step);
            // Far enough in that the text, horizontal, clears the ticks.
            let half = req.font_size * 0.6
                + a.sin().abs() * text.chars().count() as f32 * req.font_size * 0.28;
            let (x, y) = point(center, outer - major - 3.0 - half, a);
            tick_labels.push(TickLabel { x, y, text });
        }
    }
    let mut needle = String::new();
    if req.needle {
        // A tapered needle from the hub to the middle of the arc, and a short
        // tail behind the hub.
        let length = (radius + inner) / 2.0;
        let base = (radius * 0.03).max(2.5);
        let tip = point(center, length, value_angle);
        let tail = point(center, base * 2.5, value_angle + PI);
        let left = point(center, base, value_angle - PI / 2.0);
        let right = point(center, base, value_angle + PI / 2.0);
        let _ = write!(
            needle,
            "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z",
            left.0, left.1, tip.0, tip.1, right.0, right.1, tail.0, tail.1
        );
    }
    let mid = (track_outer + inner) / 2.0;
    Gauge {
        center,
        radius,
        inner,
        track: arc(start, end),
        fill: if value_angle > start {
            arc(start, value_angle)
        } else {
            String::new()
        },
        bands,
        needle,
        ticks,
        tick_labels,
        value_band,
        start_point: point(center, mid, start),
        end_point: point(center, mid, end),
        start,
        end,
    }
}

/// A ring sector from `start` to `end` with round ends, which stay inside
/// the angles. Too short for both ends, it is a dot.
fn rounded_arc(center: (f32, f32), outer: f32, inner: f32, start: f32, end: f32) -> String {
    let mut out = String::new();
    if end <= start || outer <= inner {
        return out;
    }
    let w = (outer - inner) / 2.0;
    let mid = (outer + inner) / 2.0;
    let cap = w / mid;
    if end - start <= 2.0 * cap {
        let (x, y) = point(center, mid, (start + end) / 2.0);
        let _ = write!(
            out,
            "M {:.2} {y:.2} A {w:.2} {w:.2} 0 1 0 {:.2} {y:.2} A {w:.2} {w:.2} 0 1 0 {:.2} {y:.2} Z",
            x - w,
            x + w,
            x - w
        );
        return out;
    }
    let (a, b) = (start + cap, end - cap);
    let large = u8::from(b - a > PI);
    let (o1, o2) = (point(center, outer, a), point(center, outer, b));
    let (i1, i2) = (point(center, inner, a), point(center, inner, b));
    let _ = write!(
        out,
        "M {:.2} {:.2} A {outer:.2} {outer:.2} 0 {large} 1 {:.2} {:.2} A {w:.2} {w:.2} 0 0 1 {:.2} {:.2} \
         A {inner:.2} {inner:.2} 0 {large} 0 {:.2} {:.2} A {w:.2} {w:.2} 0 0 1 {:.2} {:.2} Z",
        o1.0, o1.1, o2.0, o2.1, i2.0, i2.1, i1.0, i1.1, o1.0, o1.1
    );
    out
}

/// Whether `(x, y)` lies on the arc, and the band there, if any.
pub fn hit(gauge: &Gauge, bands: &[BandArc], x: f32, y: f32) -> Option<Option<usize>> {
    let (dx, dy) = (x - gauge.center.0, y - gauge.center.1);
    let distance = (dx * dx + dy * dy).sqrt();
    let angle = dx.atan2(-dy);
    let on_arc = distance >= gauge.inner
        && distance <= gauge.radius
        && angle >= gauge.start
        && angle <= gauge.end;
    on_arc.then(|| {
        bands
            .iter()
            .rev()
            .find(|b| angle >= b.start && angle <= b.end)
            .map(|b| b.index)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    static FORMAT: std::sync::LazyLock<Format> = std::sync::LazyLock::new(Format::default);

    fn request(bands: &[Band]) -> GaugeRequest<'_> {
        GaugeRequest {
            value: 25.0,
            min: 0.0,
            max: 100.0,
            bands,
            width: 200.0,
            height: 200.0,
            cutout: 0.75,
            sweep: 180.0,
            below_ends: 20.0,
            below_center: 0.0,
            needle: true,
            rounded: true,
            ticks: true,
            font_size: 12.0,
            format: &FORMAT,
        }
    }

    #[test]
    fn half_circle_fills_the_width() {
        let gauge = layout(&request(&[]));
        assert_eq!(gauge.radius, 100.0);
        assert_eq!(
            gauge.center,
            (100.0, 140.0),
            "centered with the labels below"
        );
        assert_eq!(gauge.inner, 75.0);
        assert!(gauge.needle.starts_with("M "));
        let (x, y) = gauge.start_point;
        assert!((x - 12.5).abs() < 1e-3 && (y - 140.0).abs() < 1e-3);
    }

    #[test]
    fn short_space_shrinks_the_arc() {
        let mut req = request(&[]);
        req.height = 80.0;
        let gauge = layout(&req);
        assert!((gauge.radius - 60.0).abs() < 1e-3, "{}", gauge.radius);
        let mut req = request(&[]);
        req.sweep = 270.0;
        req.height = 150.0;
        let gauge = layout(&req);
        assert!((gauge.radius - 76.2).abs() < 0.1, "{}", gauge.radius);
    }

    #[test]
    fn needle_ticks_and_labels() {
        let gauge = layout(&request(&[]));
        let labels: Vec<_> = gauge.tick_labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(labels, ["0", "20", "40", "60", "80", "100"]);
        // Every 20; at this size, with one minor tick between: 11 marks.
        assert_eq!(gauge.ticks.matches('M').count(), 11);
        let mut big = request(&[]);
        (big.width, big.height) = (400.0, 400.0);
        assert_eq!(
            layout(&big).ticks.matches('M').count(),
            26,
            "four minor ticks between"
        );
        let first = &gauge.tick_labels[0];
        assert!(
            first.x > 12.5 && first.x < 75.0,
            "inside the arc: {}",
            first.x
        );
    }

    #[test]
    fn fill_style_has_round_ends_and_a_band_ring() {
        let bands = [Band { to: 50.0 }, Band { to: 100.0 }];
        let mut req = request(&bands);
        req.needle = false;
        let gauge = layout(&req);
        assert!(gauge.ticks.is_empty() && gauge.needle.is_empty());
        assert_eq!(
            gauge.track.matches(" A ").count(),
            4,
            "two arcs and two round ends"
        );
        // Bands are a 5px ring at the outside; the track keeps the rest.
        assert!(gauge.bands[0].path.contains("A 95.00 95.00"));
        assert!(gauge.track.contains("A 93.00 93.00"));
        req.value = 0.0;
        assert!(
            layout(&req).fill.is_empty(),
            "nothing to fill at the minimum"
        );
        req.value = 0.5;
        assert!(layout(&req).fill.contains(" 1 0 "), "a dot");
    }

    #[test]
    fn bands_and_hits() {
        let bands = [Band { to: 50.0 }, Band { to: 80.0 }, Band { to: 100.0 }];
        let gauge = layout(&request(&bands));
        assert_eq!(gauge.bands.len(), 3);
        assert_eq!(gauge.value_band, Some(0));
        let (cx, cy) = gauge.center;
        assert_eq!(
            hit(&gauge, &gauge.bands, cx + 87.0, cy - 5.0),
            Some(Some(2))
        );
        assert_eq!(
            hit(&gauge, &gauge.bands, cx, cy - 87.0),
            Some(Some(1)),
            "50 is where the second band starts"
        );
        assert_eq!(
            hit(&gauge, &gauge.bands, cx, cy - 50.0),
            None,
            "inside the arc"
        );
        let mut req = request(&bands);
        req.value = 100.0;
        assert_eq!(
            layout(&req).value_band,
            Some(2),
            "the maximum is in the last band"
        );
    }
}
