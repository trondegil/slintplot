//! SVG path command generation for Slint's `Path { commands: ... }`.

use std::fmt::Write;

pub type Point = (f32, f32);

/// No limit for curve control points.
pub const UNBOUNDED: (f32, f32) = (f32::NEG_INFINITY, f32::INFINITY);

/// Builds the commands for a polyline through `points`.
///
/// `tension`: `0` draws straight segments, around `0.4` gives
/// smooth Bézier curves, up to `1`. Curve control points are clamped to `y_bounds`, so a
/// curve between points inside the plot stays inside it.
pub fn line(points: &[Point], tension: f32, y_bounds: (f32, f32)) -> String {
    let points = &clip(points, y_bounds);
    let mut out = String::new();
    let Some(&(x0, y0)) = points.first() else {
        return out;
    };
    let _ = write!(out, "M {x0:.2} {y0:.2}");
    append_segments(&mut out, points, tension, y_bounds);
    out
}

/// Builds the commands for the area between the line through `points` and the
/// horizontal `baseline`.
pub fn area(points: &[Point], tension: f32, baseline: f32, y_bounds: (f32, f32)) -> String {
    let points = &clip(points, y_bounds);
    let mut out = String::new();
    let (Some(&(x0, y0)), Some(&(last_x, _))) = (points.first(), points.last()) else {
        return out;
    };
    let _ = write!(out, "M {x0:.2} {baseline:.2} L {x0:.2} {y0:.2}");
    append_segments(&mut out, points, tension, y_bounds);
    let _ = write!(out, " L {last_x:.2} {baseline:.2} Z");
    out
}

/// Builds the commands for the area between two lines through matching x
/// positions, such as a stacked dataset and the one below it.
pub fn band(top: &[Point], bottom: &[Point], tension: f32, y_bounds: (f32, f32)) -> String {
    let mut out = line(top, tension, y_bounds);
    if out.is_empty() {
        return out;
    }
    let reversed: Vec<Point> = clip(bottom, y_bounds).into_iter().rev().collect();
    if let Some(&(x, y)) = reversed.first() {
        let _ = write!(out, " L {x:.2} {y:.2}");
        append_segments(&mut out, &reversed, tension, y_bounds);
    }
    out.push_str(" Z");
    out
}

/// Turns a polyline into steps: from each point, vertical to the next
/// point's value, then horizontal to it.
pub fn steps(points: &[Point]) -> Vec<Point> {
    let mut out = Vec::with_capacity(points.len() * 2);
    for (i, &(x, y)) in points.iter().enumerate() {
        if i > 0 {
            out.push((points[i - 1].0, y));
        }
        out.push((x, y));
    }
    out
}

/// Pulls points more than a plot's height above or below `y_bounds` in to
/// that distance, adding a point where the line crosses it: the visible part
/// stays the same, and the renderer, whose work grows with a line's length,
/// never gets one millions of pixels long.
fn clip(points: &[Point], (lo, hi): (f32, f32)) -> Vec<Point> {
    let margin = (hi - lo).abs().max(1.0);
    let (top, bottom) = (lo.min(hi) - margin, lo.max(hi) + margin);
    if points.iter().all(|&(_, y)| y >= top && y <= bottom) {
        return points.to_vec();
    }
    let mut out = Vec::with_capacity(points.len() + 4);
    for (i, &(x, y)) in points.iter().enumerate() {
        if i > 0 {
            let (px, py) = points[i - 1];
            // Where the segment crosses the edges, in the order it does.
            let mut crossings: Vec<f32> = [top, bottom]
                .into_iter()
                .filter(|&edge| (py - edge) * (y - edge) < 0.0)
                .map(|edge| (edge - py) / (y - py))
                .collect();
            crossings.sort_by(f32::total_cmp);
            for t in crossings {
                out.push((px + (x - px) * t, (py + (y - py) * t).clamp(top, bottom)));
            }
        }
        out.push((x, y.clamp(top, bottom)));
    }
    out
}

fn append_segments(out: &mut String, points: &[Point], tension: f32, (lo, hi): (f32, f32)) {
    // NaN draws straight lines; past 1 the curves overshoot, and a huge
    // tension puts the control points at infinity.
    let tension = if tension.is_nan() {
        0.0
    } else {
        tension.min(1.0)
    };
    if tension <= 0.0 || points.len() < 3 {
        for &(x, y) in &points[1..] {
            let _ = write!(out, " L {x:.2} {y:.2}");
        }
        return;
    }
    let controls = control_points(points, tension);
    for i in 1..points.len() {
        let (c1x, c1y) = controls[i - 1].1;
        let (c2x, c2y) = controls[i].0;
        let (c1y, c2y) = (c1y.clamp(lo, hi), c2y.clamp(lo, hi));
        let (x, y) = points[i];
        let _ = write!(out, " C {c1x:.2} {c1y:.2} {c2x:.2} {c2y:.2} {x:.2} {y:.2}");
    }
}

/// For every point, returns its (previous, next) Bézier control points using the
/// distance-weighted spline: control points along the line through the
/// neighbors, scaled by the distance to each.
fn control_points(points: &[Point], tension: f32) -> Vec<(Point, Point)> {
    let n = points.len();
    (0..n)
        .map(|i| {
            let cur = points[i];
            let prev = points[i.saturating_sub(1)];
            let next = points[(i + 1).min(n - 1)];
            let d01 = dist(prev, cur);
            let d12 = dist(cur, next);
            let total = d01 + d12;
            if total == 0.0 {
                return (cur, cur);
            }
            let fa = tension * d01 / total;
            let fb = tension * d12 / total;
            let (dx, dy) = (next.0 - prev.0, next.1 - prev.1);
            (
                (cur.0 - fa * dx, cur.1 - fa * dy),
                (cur.0 + fb * dx, cur.1 + fb * dy),
            )
        })
        .collect()
}

fn dist(a: Point, b: Point) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_line() {
        assert_eq!(
            line(&[(0.0, 0.0), (10.0, 5.0)], 0.0, UNBOUNDED),
            "M 0.00 0.00 L 10.00 5.00"
        );
        assert_eq!(line(&[], 0.4, UNBOUNDED), "");
    }

    #[test]
    fn far_points_are_pulled_in_where_the_line_crosses() {
        // A plot 100 high: points are kept within 100 more either side.
        assert_eq!(
            line(
                &[(0.0, 50.0), (10.0, 1050.0), (20.0, 50.0)],
                0.0,
                (0.0, 100.0)
            ),
            "M 0.00 50.00 L 1.50 200.00 L 10.00 200.00 L 18.50 200.00 L 20.00 50.00"
        );
        // Straight through, top to bottom.
        assert_eq!(
            line(&[(0.0, -1e9), (10.0, 1e9)], 0.0, (0.0, 100.0)),
            "M 0.00 -100.00 L 5.00 -100.00 L 5.00 200.00 L 10.00 200.00"
        );
    }

    #[test]
    fn curve_passes_through_points() {
        let cmds = line(&[(0.0, 10.0), (5.0, 0.0), (10.0, 10.0)], 0.4, UNBOUNDED);
        assert!(cmds.starts_with("M 0.00 10.00 C"));
        assert!(cmds.contains(" 5.00 0.00 C"));
        assert!(cmds.ends_with(" 10.00 10.00"));
    }

    #[test]
    fn band_goes_back_along_the_bottom() {
        let cmds = band(
            &[(0.0, 1.0), (4.0, 2.0)],
            &[(0.0, 5.0), (4.0, 6.0)],
            0.0,
            UNBOUNDED,
        );
        assert_eq!(cmds, "M 0.00 1.00 L 4.00 2.00 L 4.00 6.00 L 0.00 5.00 Z");
    }

    #[test]
    fn steps_go_vertical_then_horizontal() {
        assert_eq!(
            steps(&[(0.0, 5.0), (10.0, 2.0), (20.0, 8.0)]),
            vec![
                (0.0, 5.0),
                (0.0, 2.0),
                (10.0, 2.0),
                (10.0, 8.0),
                (20.0, 8.0)
            ]
        );
    }

    #[test]
    fn control_points_stay_inside_the_bounds() {
        // A peak at the top edge: unclamped, the curve would bulge above 0.
        let points = [(0.0, 50.0), (10.0, 0.0), (20.0, 0.0), (30.0, 50.0)];
        let cmds = line(&points, 0.4, (0.0, 100.0));
        let ys: Vec<f32> = cmds
            .split(|c: char| c.is_ascii_alphabetic())
            .flat_map(|part| {
                part.split_whitespace()
                    .skip(1)
                    .step_by(2)
                    .map(|v| v.parse::<f32>().unwrap())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(ys.iter().all(|&y| y >= 0.0), "{cmds}");
    }

    #[test]
    fn closed_area() {
        let cmds = area(&[(0.0, 2.0), (4.0, 1.0)], 0.0, 8.0, UNBOUNDED);
        assert_eq!(cmds, "M 0.00 8.00 L 0.00 2.00 L 4.00 1.00 L 4.00 8.00 Z");
    }
}
