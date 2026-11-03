//! Geometry for segmented ring charts: a heatmap bent into concentric
//! rings. Each row of the matrix is a ring, the first outermost, and each
//! column a segment of every ring, going round from the start angle. The
//! matrix sets the size: as many rings as rows, and as many segments as the
//! longest row has values. A cell is colored on a scale by its value, as in
//! a heatmap, or by the class its value names; missing cells are left as a
//! faint track.

use std::f32::consts::TAU;

use crate::layout::Rgba;
use crate::pie::{point, slice_path};

/// The most rings and segments per ring, so that a huge matrix can't make
/// the chart unusably slow.
pub const MAX_ROWS: usize = 50;
pub const MAX_COLUMNS: usize = 720;

pub struct SegmentedRequest<'a> {
    /// The matrix, one row per ring; rows may differ in length, and NaN is
    /// a missing value.
    pub values: &'a [Vec<f64>],
    pub width: f32,
    pub height: f32,
    /// Where the first column starts, in degrees clockwise from 12 o'clock,
    /// and which way the columns go.
    pub start_angle: f32,
    pub clockwise: bool,
    /// The empty middle, as a share of the radius; the gap between rings, as
    /// a share of their width; and the gap between segments, as a share of
    /// the angle each takes.
    pub cutout: f32,
    pub ring_gap: f32,
    pub segment_gap: f32,
    /// The size of the widest column label, written round the outside; 0
    /// for none.
    pub label_width: f32,
    pub label_height: f32,
    /// Leave room for a dot at the start, outside the outer ring.
    pub marker: bool,
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
    pub columns: usize,
    /// One (outer, inner) per row, outermost first.
    pub rings: Vec<(f32, f32)>,
    /// Where the middles of the column labels go, and how many columns
    /// from one label to the next, so that they don't overlap.
    pub label_radius: f32,
    pub label_skip: usize,
    /// The dot at the start: its center and diameter (0 for none).
    pub marker: (f32, f32, f32),
}

fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() { v } else { fallback }
}

/// The size of a matrix as (rows, columns): every row, and the longest,
/// within the limits; at least one of each.
pub fn dimensions(values: &[Vec<f64>]) -> (usize, usize) {
    let rows = values.len().clamp(1, MAX_ROWS);
    let columns = values
        .iter()
        .take(MAX_ROWS)
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .clamp(1, MAX_COLUMNS);
    (rows, columns)
}

pub fn layout(req: &SegmentedRequest) -> Segmented {
    let size = |v: f32| finite_or(v, 0.0).max(0.0);
    let (width, height) = (size(req.width), size(req.height));
    let (label_w, label_h) = (size(req.label_width), size(req.label_height));
    let labels = label_w > 0.0 && label_h > 0.0;
    let start = finite_or(req.start_angle, 0.0)
        .rem_euclid(360.0)
        .to_radians();
    let direction = if req.clockwise { 1.0 } else { -1.0 };
    let cutout = finite_or(req.cutout, 0.45).clamp(0.0, 0.9);
    let ring_gap = finite_or(req.ring_gap, 0.5).clamp(0.0, 3.0);
    let segment_gap = finite_or(req.segment_gap, 0.3).clamp(0.0, 0.9);
    let (rows, columns) = dimensions(req.values);

    // The ring width as a share of the radius: the rings share what the
    // cutout leaves. The marker takes a little more than a ring outside
    // them, and the labels their size.
    let share = (1.0 - cutout) / (rows as f32 + (rows as f32 - 1.0) * ring_gap);
    // Few rings stay narrow, at the outside.
    let share = share.min(0.2);
    let marker_room = if req.marker { 1.2 } else { 0.3 };
    let pad = label_h * 0.4;
    let label_room = if labels {
        pad + label_w.max(label_h)
    } else {
        0.0
    };
    let half = width.min(height) / 2.0;
    let radius = ((half - label_room) / (1.0 + marker_room * share)).max(0.0);
    let w = radius * share;
    let center = (width / 2.0, height / 2.0);

    let rings: Vec<(f32, f32)> = (0..rows)
        .map(|k| {
            let outer = radius - k as f32 * w * (1.0 + ring_gap);
            (outer.max(0.0), (outer - w).max(0.0))
        })
        .collect();

    let marker = if req.marker && radius > 0.0 {
        let d = w * 0.7;
        let (x, y) = point(center, radius + w * 0.45 + d / 2.0, start);
        (x, y, d)
    } else {
        (center.0, center.1, 0.0)
    };
    let label_radius = radius + marker_room * w + pad + label_w.max(label_h) / 2.0;
    let arc = TAU * label_radius / columns as f32;
    let label_skip = if labels && arc > 0.0 {
        ((label_w + label_h * 0.5) / arc).ceil().max(1.0) as usize
    } else {
        1
    }
    .min(columns);

    Segmented {
        center,
        radius,
        inner_radius: rings.last().map_or(0.0, |r| r.1),
        start,
        direction,
        segment_gap,
        columns,
        rings,
        label_radius,
        label_skip,
        marker,
    }
}

/// The angles of a column's segments, the smaller first.
pub fn column_angles(chart: &Segmented, column: usize) -> (f32, f32) {
    let pitch = TAU / chart.columns.max(1) as f32;
    let a = chart.start + chart.direction * column as f32 * pitch;
    let b = chart.start + chart.direction * (column + 1) as f32 * pitch;
    let trim = pitch * chart.segment_gap / 2.0;
    (a.min(b) + trim, a.max(b) - trim)
}

/// The angle of the middle of a column.
pub fn column_middle(chart: &Segmented, column: usize) -> f32 {
    let (a, b) = column_angles(chart, column);
    (a + b) / 2.0
}

fn push_cell(out: &mut String, chart: &Segmented, row: usize, column: usize) {
    let Some(&(outer, inner)) = chart.rings.get(row) else {
        return;
    };
    let (a, b) = column_angles(chart, column);
    if outer > inner && b > a && column < chart.columns {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&slice_path(chart.center, outer, inner, a, b));
    }
}

/// The segment of one cell, as a path.
pub fn cell_path(chart: &Segmented, row: usize, column: usize) -> String {
    let mut out = String::new();
    if chart.center.0.is_finite() && chart.center.1.is_finite() {
        push_cell(&mut out, chart, row, column);
    }
    out
}

/// The cell at `(x, y)`, as (row, column), gaps included.
pub fn hit(chart: &Segmented, x: f32, y: f32) -> Option<(usize, usize)> {
    let (dx, dy) = (x - chart.center.0, y - chart.center.1);
    let distance = (dx * dx + dy * dy).sqrt();
    // Part of the gap round each ring belongs to it.
    let row = chart.rings.iter().position(|&(outer, inner)| {
        let pad = (outer - inner) * 0.25;
        outer > inner && distance >= inner - pad && distance <= outer + pad
    })?;
    let along = ((dx.atan2(-dy) - chart.start) * chart.direction).rem_euclid(TAU);
    let columns = chart.columns.max(1);
    let column = ((along / TAU * columns as f32) as usize).min(columns - 1);
    Some((row, column))
}

/// How cells are colored.
pub enum Coloring<'a> {
    /// On a scale from `low` to `high`, as in a heatmap: from a faint
    /// `color` to the full one, or with `diverging`, from `negative` through
    /// `empty` at `center` to `color`.
    Scale {
        low: f64,
        high: f64,
        diverging: bool,
        center: f64,
        color: Rgba,
        negative: Rgba,
        empty: Rgba,
    },
    /// By category: a value is the index of its category, and the cell takes its
    /// color. Hidden classes, and values that name none, are left empty.
    Classes {
        colors: &'a [Rgba],
        hidden: &'a [bool],
    },
}

fn mix(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    [
        m(from[0], to[0]),
        m(from[1], to[1]),
        m(from[2], to[2]),
        m(from[3], to[3]),
    ]
}

/// The steps of a color scale, so that cells of nearly the same value
/// share a path.
const SCALE_STEPS: f64 = 32.0;

/// A cell's color, or `None` for an empty cell.
pub fn cell_color(value: f64, coloring: &Coloring) -> Option<Rgba> {
    if !value.is_finite() {
        return None;
    }
    match *coloring {
        Coloring::Scale {
            low,
            high,
            diverging,
            center,
            color,
            negative,
            empty,
        } => {
            let s = if diverging {
                let reach = (high - center).abs().max((low - center).abs());
                if reach > 0.0 {
                    ((value - center) / reach).clamp(-1.0, 1.0)
                } else {
                    0.0
                }
            } else if high > low {
                ((value - low) / (high - low)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let s = if s.is_finite() { s } else { 0.0 };
            let s = ((s * SCALE_STEPS).round() / SCALE_STEPS) as f32;
            // Even the smallest value is visibly colored.
            Some(if s < 0.0 {
                mix(empty, negative, 0.12 - s * 0.88)
            } else if diverging {
                mix(empty, color, s)
            } else {
                mix(empty, color, 0.12 + s * 0.88)
            })
        }
        Coloring::Classes { colors, hidden } => {
            let class = value.round();
            if class < 0.0 || class >= colors.len() as f64 {
                return None;
            }
            let class = class as usize;
            (!hidden.get(class).copied().unwrap_or(false)).then(|| colors[class])
        }
    }
}

/// The cells of a `rows` × `columns` matrix as one path per color, and the
/// empty ones, missing cells included, as the track; `push` adds a cell's
/// shape to a path. Only the first `progress` (0 to 1) of the columns are
/// colored, so that a chart can sweep in. Shared by the charts of matrices
/// of cells, such as segmented rings and tile grids.
pub fn paint_cells(
    rows: usize,
    columns: usize,
    values: &[Vec<f64>],
    coloring: &Coloring,
    progress: f32,
    mut push: impl FnMut(&mut String, usize, usize),
) -> (String, Vec<(Rgba, String)>) {
    let mut track = String::new();
    let mut groups: Vec<(Rgba, String)> = Vec::new();
    let lit = (finite_or(progress, 1.0).clamp(0.0, 1.0) * columns as f32).ceil() as usize;
    for row in 0..rows {
        for column in 0..columns {
            let value = values
                .get(row)
                .and_then(|r| r.get(column))
                .copied()
                .unwrap_or(f64::NAN);
            let color = if column < lit {
                cell_color(value, coloring)
            } else {
                None
            };
            let out = match color {
                None => &mut track,
                Some(c) => match groups.iter().position(|g| g.0 == c) {
                    Some(i) => &mut groups[i].1,
                    None => {
                        groups.push((c, String::new()));
                        &mut groups.last_mut().unwrap().1
                    }
                },
            };
            push(out, row, column);
        }
    }
    (track, groups)
}

/// The rings' cells as one path per color, and the empty ones as the
/// track; see [`paint_cells`].
pub fn paint(
    chart: &Segmented,
    values: &[Vec<f64>],
    coloring: &Coloring,
    progress: f32,
) -> (String, Vec<(Rgba, String)>) {
    if !chart.center.0.is_finite() || !chart.center.1.is_finite() {
        return (String::new(), Vec::new());
    }
    paint_cells(
        chart.rings.len(),
        chart.columns,
        values,
        coloring,
        progress,
        |out, row, column| push_cell(out, chart, row, column),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(values: &[Vec<f64>]) -> SegmentedRequest<'_> {
        SegmentedRequest {
            values,
            width: 400.0,
            height: 300.0,
            start_angle: 0.0,
            clockwise: true,
            cutout: 0.45,
            ring_gap: 0.5,
            segment_gap: 0.3,
            label_width: 0.0,
            label_height: 0.0,
            marker: false,
        }
    }

    fn matrix(rows: usize, columns: usize) -> Vec<Vec<f64>> {
        vec![vec![0.0; columns]; rows]
    }

    const CLASSES: [Rgba; 3] = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]];

    #[test]
    fn the_matrix_sets_the_rings_and_segments() {
        let m = matrix(5, 24);
        let chart = layout(&request(&m));
        assert_eq!(chart.center, (200.0, 150.0));
        assert_eq!(chart.rings.len(), 5);
        assert_eq!(chart.columns, 24);
        // A small room is left outside, as when there is a marker.
        assert!(chart.radius < 150.0 && chart.radius > 140.0);
        assert_eq!(chart.rings[0].0, chart.radius);
        for pair in chart.rings.windows(2) {
            assert!(pair[1].0 < pair[0].1);
        }
        assert!((chart.inner_radius - chart.radius * 0.45).abs() < 1e-3);
        // Ragged rows: the longest sets the columns.
        let ragged = vec![vec![1.0; 3], vec![1.0; 30], vec![]];
        assert_eq!(dimensions(&ragged), (3, 30));
        assert_eq!(dimensions(&[]), (1, 1));
        assert_eq!(dimensions(&matrix(80, 1000)), (MAX_ROWS, MAX_COLUMNS));
        // More rings are narrower; a single ring is not the whole radius.
        let many = layout(&request(&matrix(20, 10)));
        let one = layout(&request(&matrix(1, 10)));
        let width = |c: &Segmented| c.rings[0].0 - c.rings[0].1;
        assert!(width(&many) < width(&layout(&request(&m))));
        assert!((width(&one) - one.radius * 0.2).abs() < 1e-3);
    }

    #[test]
    fn columns_go_round_both_ways_and_are_hit() {
        let m = matrix(3, 12);
        let mut req = request(&m);
        let chart = layout(&req);
        let (outer, inner) = chart.rings[1];
        let mid = (outer + inner) / 2.0;
        let (x, y) = point(chart.center, mid, 0.1);
        assert_eq!(hit(&chart, x, y), Some((1, 0)));
        let (x, y) = point(chart.center, mid, -0.1);
        assert_eq!(hit(&chart, x, y), Some((1, 11)));
        let (x, y) = point(chart.center, mid, std::f32::consts::PI + 0.1);
        assert_eq!(hit(&chart, x, y), Some((1, 6)));
        assert_eq!(hit(&chart, chart.center.0, chart.center.1), None);

        req.clockwise = false;
        let chart = layout(&req);
        let (x, y) = point(chart.center, mid, -0.1);
        assert_eq!(hit(&chart, x, y), Some((1, 0)));
        assert!(column_middle(&chart, 0) < 0.0);
        assert_eq!(cell_path(&chart, 0, 0).matches('M').count(), 1);
        assert!(cell_path(&chart, 9, 0).is_empty());
    }

    #[test]
    fn cells_are_grouped_by_color() {
        let nan = f64::NAN;
        let values = vec![vec![0.0, 1.0, 0.0], vec![2.0, nan, 7.0]];
        let chart = layout(&request(&values));
        let coloring = Coloring::Classes {
            colors: &CLASSES,
            hidden: &[false, false, true],
        };
        let (track, groups) = paint(&chart, &values, &coloring, 1.0);
        // Missing, unknown and hidden classes are empty.
        assert_eq!(track.matches('M').count(), 3);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, CLASSES[0]);
        assert_eq!(groups[0].1.matches('M').count(), 2);
        assert_eq!(groups[1].1.matches('M').count(), 1);
        // A short row leaves the rest of its ring empty.
        let short = vec![vec![0.0, 0.0, 0.0], vec![0.0]];
        let chart = layout(&request(&short));
        let (track, groups) = paint(&chart, &short, &coloring, 1.0);
        assert_eq!(track.matches('M').count(), 2);
        assert_eq!(groups[0].1.matches('M').count(), 4);
        // Sweeping in colors only the first columns.
        let full = matrix(2, 3);
        let (track, groups) = paint(&chart, &full, &coloring, 0.3);
        assert_eq!(track.matches('M').count(), 4);
        assert_eq!(groups[0].1.matches('M').count(), 2);
    }

    #[test]
    fn scale_colors_like_a_heatmap() {
        let empty = [238, 240, 243, 255];
        let color = [0, 100, 200, 255];
        let scale = Coloring::Scale {
            low: 0.0,
            high: 10.0,
            diverging: false,
            center: 0.0,
            color,
            negative: [200, 0, 0, 255],
            empty,
        };
        assert_eq!(cell_color(10.0, &scale), Some(color));
        assert_eq!(cell_color(99.0, &scale), Some(color));
        assert_eq!(cell_color(0.0, &scale), Some(mix(empty, color, 0.12)));
        assert_eq!(cell_color(f64::NAN, &scale), None);
        // Nearly equal values share a color.
        assert_eq!(cell_color(5.0, &scale), cell_color(5.001, &scale));
        let diverging = Coloring::Scale {
            low: -5.0,
            high: 10.0,
            diverging: true,
            center: 0.0,
            color,
            negative: [200, 0, 0, 255],
            empty,
        };
        assert_eq!(cell_color(0.0, &diverging), Some(empty));
        assert_eq!(cell_color(-10.0, &diverging), Some([200, 0, 0, 255]));
    }

    #[test]
    fn labels_and_marker_get_room() {
        let m = matrix(4, 24);
        let mut req = request(&m);
        let plain = layout(&req);
        req.marker = true;
        req.start_angle = 90.0;
        let marked = layout(&req);
        let (x, y, d) = marked.marker;
        assert!(d > 0.0 && (y - marked.center.1).abs() < 1e-3);
        assert!(x - d / 2.0 > marked.center.0 + marked.radius);
        assert!(marked.radius < plain.radius);
        req.label_width = 30.0;
        req.label_height = 14.0;
        let labeled = layout(&req);
        assert!(labeled.radius < marked.radius);
        assert!(labeled.label_radius + 15.0 <= 150.0 + 1e-3);
        // 24 labels of 30 px don't fit round a circle of this size.
        assert!(labeled.label_skip > 1, "{}", labeled.label_skip);
        let few = matrix(4, 6);
        req.values = &few;
        assert_eq!(layout(&req).label_skip, 1);
    }

    #[test]
    fn bad_input_gives_finite_shapes() {
        let row = vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -5.0, 1e300, 0.0];
        for (start, cutout, size, rows) in [
            (0.0, 0.45, 300.0, 5),
            (f32::NAN, f32::NAN, f32::NAN, 0),
            (f32::INFINITY, 5.0, 0.0, 1),
            (1e30, -1.0, -10.0, 200),
            (-720.0, 0.9, 1e7, 1),
        ] {
            let values = vec![row.clone(); rows];
            let mut req = request(&values);
            (req.start_angle, req.cutout, req.width, req.height) = (start, cutout, size, size);
            (req.label_width, req.label_height, req.marker) = (size, 14.0, true);
            let chart = layout(&req);
            assert!(chart.radius.is_finite() && chart.label_radius.is_finite());
            assert!(chart.rings.len() <= MAX_ROWS && chart.columns <= MAX_COLUMNS);
            assert!(chart.label_skip >= 1);
            for coloring in [
                Coloring::Scale {
                    low: f64::NAN,
                    high: f64::INFINITY,
                    diverging: true,
                    center: f64::NAN,
                    color: [0; 4],
                    negative: [0; 4],
                    empty: [0; 4],
                },
                Coloring::Classes {
                    colors: &CLASSES,
                    hidden: &[],
                },
            ] {
                let (track, groups) = paint(&chart, &values, &coloring, f32::NAN);
                for p in groups.iter().map(|g| &g.1).chain([&track]) {
                    assert!(!p.contains("NaN") && !p.contains("inf"));
                }
            }
            let _ = hit(&chart, 1.0, 1.0);
        }
    }
}
