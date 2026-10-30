//! Geometry for bullet charts, after Stephen Few's design: rows of a value
//! bar against a target mark, over shaded ranges such as poor, OK and good,
//! each row on its own value axis or all on one.

use std::fmt::Write;

use crate::layout::{Axis, AxisRequest, Dataset, Kind, Label, Rgba, ValueScale, value_axis};
use crate::ticks::Format;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub value: f64,
    /// Where the target mark goes; NaN for none.
    pub target: f64,
    /// Where each band ends, in any order. A band starts where the one below
    /// it ends, or at the start of the axis.
    pub bands: Vec<f64>,
    /// How the row's axis and value are written.
    pub format: Format,
}

pub struct BulletRequest<'a> {
    pub rows: &'a [Row],
    pub width: f32,
    pub height: f32,
    /// Columns that grow upwards, instead of rows that grow to the right.
    pub vertical: bool,
    /// One axis for all rows, written in `format`, at the bottom (or the
    /// left); otherwise each row has its own.
    pub shared_axis: bool,
    pub format: &'a Format,
    /// Fixed ends of the axes, instead of fitting each row.
    pub fixed: Option<(f64, f64)>,
    pub font_size: f32,
    /// The entry animation, which grows the bars from zero.
    pub progress: f32,
    /// Write each value at the end of its track, or under its column.
    pub show_values: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BulletRow {
    pub index: usize,
    /// The row's part of the plot, and the shaded track in it.
    pub cell: Rect,
    pub track: Rect,
    pub bar: Rect,
    /// The target mark; `None` when the target is missing or off the axis.
    pub target: Option<Rect>,
    /// The band that holds the value, if any.
    pub value_band: Option<usize>,
    /// The value, past the end of the track or under the column.
    pub value_label: Option<Label>,
    /// 1 when the value is past the end of the axis, -1 when it is before
    /// its start, else 0.
    pub overflow: i8,
    /// For a value off the axis, an arrow at the end of the bar, as path
    /// commands.
    pub cue: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BulletBand {
    pub row: usize,
    /// Index of the band in the row's bands.
    pub index: usize,
    pub rect: Rect,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bullet {
    pub rows: Vec<BulletRow>,
    pub bands: Vec<BulletBand>,
    pub ticks: Vec<Label>,
    /// Short marks at the ticks, as path commands.
    pub tick_marks: String,
    /// The size of a row: its height, or a column's width.
    pub step: f32,
}

/// The thickness of a track, in font sizes, at most.
const TRACK: f32 = 1.9;
/// The space between two rows, in font sizes.
const ROW_GAP: f32 = 0.9;
/// The widest a column gets, in font sizes, besides its own tick labels.
const COLUMN: f32 = 7.0;
/// The value bar's share of the track's thickness.
const BAR: f32 = 0.45;

/// The default color of band `index` of `count`: `color` in a translucent
/// step, so the first (often "poor") is the darkest and the last the
/// lightest.
pub fn shade(index: usize, count: usize, [r, g, b, _]: Rgba) -> Rgba {
    let alpha = if count > 1 {
        let t = index.min(count - 1) as f32 / (count - 1) as f32;
        0.32 - 0.21 * t
    } else {
        0.2
    };
    [r, g, b, (alpha * 255.0).round() as u8]
}

/// A target from Slint, where it can't be left out: 0 counts as none
/// unless `has_target` is set.
pub fn target(target: f64, has_target: bool) -> f64 {
    if has_target || target != 0.0 {
        target
    } else {
        f64::NAN
    }
}

/// The color of a row's bar: `below` when it is set (not transparent) and
/// the value is below the target, else the row's `own` color when set, else
/// the chart's.
pub fn bar_color(row: &Row, own: Rgba, chart: Rgba, below: Rgba) -> Rgba {
    if below[3] > 0 && row.value < row.target {
        below
    } else if own[3] > 0 {
        own
    } else {
        chart
    }
}

/// `color` as it looks over `base`, opaque: for legends, whose swatches
/// set their own transparency.
pub fn over([r, g, b, a]: Rgba, [br, bg, bb, _]: Rgba) -> Rgba {
    let a = a as f32 / 255.0;
    let mix = |c: u8, base: u8| (c as f32 * a + base as f32 * (1.0 - a)).round() as u8;
    [mix(r, br), mix(g, bg), mix(b, bb), 255]
}

/// The value minus the target with its sign, such as "+$20" or "-$5", in
/// the row's format; `None` without both.
pub fn delta(row: &Row) -> Option<String> {
    let d = row.value - row.target;
    d.is_finite().then(|| {
        if d > 0.0 {
            format!("+{}", row.format.value(d))
        } else {
            row.format.value(d)
        }
    })
}

/// The value in percent of a positive target, such as 108 for 270 against
/// 250.
pub fn of_target(row: &Row) -> Option<f64> {
    let p = row.value / row.target * 100.0;
    (row.target > 0.0 && p.is_finite()).then_some(p)
}

/// The band ends of a row with their indices, from low to high, without
/// NaN.
fn sorted_bands(bands: &[f64]) -> Vec<(usize, f64)> {
    let mut sorted: Vec<(usize, f64)> = bands
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, to)| !to.is_nan())
        .collect();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1));
    sorted
}

/// The axis of a row, or of all rows: from zero, or the smallest value
/// below it, to the largest value, target or band end. It ends with the
/// last band when that reaches past the values and targets, else at a tick.
fn axis(rows: &[&Row], req: &BulletRequest, length: f32, format: &Format) -> Axis {
    let values: Vec<f64> = rows
        .iter()
        .flat_map(|r| {
            [r.value, r.target]
                .into_iter()
                .chain(r.bands.iter().copied())
        })
        .collect();
    let finite = |v: &f64| v.is_finite();
    let marks = rows
        .iter()
        .flat_map(|r| [r.value, r.target])
        .filter(finite)
        .fold(f64::NEG_INFINITY, f64::max);
    let last_band = rows
        .iter()
        .flat_map(|r| r.bands.iter().copied())
        .filter(finite)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = values.iter().copied().filter(finite).fold(0.0, f64::min);
    let to_band = (last_band >= marks && last_band > low && (last_band - low).is_finite())
        .then_some((low, last_band));
    let spacing = req.font_size
        * match (req.vertical, req.shared_axis) {
            (true, _) => 3.5,
            (false, true) => 8.0,
            // Each row is labelled: a few ticks are enough.
            (false, false) => 11.0,
        };
    let spacing = if req.vertical || req.shared_axis {
        spacing
    } else {
        spacing.max(length / 4.5)
    };
    value_axis(&AxisRequest {
        datasets: &[Dataset {
            values: values.into(),
            ..Default::default()
        }],
        kind: Kind::Line,
        stacked: false,
        label_count: 0,
        length,
        begin_at_zero: true,
        tick_spacing: spacing,
        view: (0.0, 1.0),
        fixed: req
            .fixed
            .filter(|&(lo, hi)| lo.is_finite() && hi.is_finite() && hi > lo)
            .or(to_band),
        logarithmic: false,
        format,
        percent: false,
        grace: 0.0,
        side: None,
        align_ticks: 0,
        ordered: false,
    })
}

/// A rough width of a label, for the room beside the axis.
fn text_width(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * 0.6
}

/// The value label of a row, or `None` for a value that isn't a number.
fn value_text(row: &Row) -> Option<String> {
    row.value.is_finite().then(|| row.format.value(row.value))
}

/// The height a horizontal chart's rows need at most, each, for a font
/// size: the track, the row's tick labels, and the gap to the next row.
pub fn row_height(font_size: f32, shared_axis: bool) -> f32 {
    let ticks = if shared_axis {
        0.0
    } else {
        font_size * 1.4 + 6.0
    };
    font_size * (TRACK + ROW_GAP) + ticks
}

pub fn layout(req: &BulletRequest) -> Bullet {
    let n = req.rows.len();
    let font = if req.font_size.is_finite() {
        req.font_size.clamp(1.0, 100.0)
    } else {
        12.0
    };
    if n == 0 || !(req.width > 0.0 && req.height > 0.0) {
        return Bullet::default();
    }
    let req = &BulletRequest {
        font_size: font,
        ..*req
    };
    let (width, height) = (req.width.min(1e6), req.height.min(1e6));
    let progress = if req.progress.is_nan() {
        1.0
    } else {
        req.progress.clamp(0.0, 1.0)
    };
    let all: Vec<&Row> = req.rows.iter().collect();
    let axis_of = |index: usize, length: f32| {
        if req.shared_axis {
            axis(&all, req, length, req.format)
        } else {
            let row = &req.rows[index];
            axis(&[row], req, length, &row.format)
        }
    };
    let tick_gap = 6.0;
    let mut out = Bullet::default();
    let widest = |a: &Axis| text_width(&a.widest_label, font);
    // The room for the value labels: past the end of the tracks, or under
    // the columns.
    let widest_value = req
        .rows
        .iter()
        .filter_map(value_text)
        .map(|t| text_width(&t, font))
        .fold(0.0, f32::max);
    let label_room = font * 1.4 + tick_gap;
    let value_room = match (req.show_values, req.vertical) {
        (false, _) => 0.0,
        (true, false) => (widest_value + 8.0).min(width / 3.0),
        (true, true) => (font * 1.4 + 4.0).min(height / 3.0),
    };
    let shared_width = if req.vertical && req.shared_axis {
        widest(&axis_of(0, height)) + tick_gap + 2.0
    } else {
        0.0
    };
    // Along the axis: from 0 to `length`, leaving half a label at the far
    // end, or room for the values.
    let pad = font * 0.7;
    let length_for = |a: &Axis| {
        if req.vertical {
            (height - pad - value_room.max(pad)).max(0.0)
        } else {
            (width - value_room.max((widest(a) / 2.0).min(width / 4.0))).max(0.0)
        }
    };
    let across = if req.vertical {
        (width - shared_width).max(0.0)
    } else {
        height
    };
    let below = if req.shared_axis && !req.vertical {
        label_room.min(across / 2.0)
    } else {
        0.0
    };
    // Rows as tall as they need and no taller, from the top; columns at
    // most so wide, centered.
    let own_room = |a: &Axis| {
        if req.shared_axis {
            0.0
        } else if req.vertical {
            widest(a) + tick_gap + 2.0
        } else {
            label_room
        }
    };
    let longest = if req.vertical && !req.shared_axis {
        (0..n)
            .map(|i| own_room(&axis_of(i, height)))
            .fold(0.0, f32::max)
    } else {
        0.0
    };
    let most = if req.vertical {
        font * COLUMN + longest
    } else {
        row_height(font, req.shared_axis)
    };
    let step = ((across - below).max(0.0) / n as f32).min(most);
    out.step = step;
    let first = if req.vertical {
        ((width - shared_width - n as f32 * step) / 2.0).max(0.0) + shared_width
    } else {
        0.0
    };
    // A shared axis is the same for every row: computed once.
    let shared = req.shared_axis.then(|| {
        let first = axis_of(0, if req.vertical { height } else { width });
        let length = length_for(&first);
        (axis_of(0, length), length)
    });

    for (index, row) in req.rows.iter().enumerate() {
        let (axis, length) = match &shared {
            Some(s) => s.clone(),
            None => {
                let first = axis_of(index, if req.vertical { height } else { width });
                let length = length_for(&first);
                (axis_of(index, length), length)
            }
        };
        // The room the row's own tick labels need beside the track.
        let own = own_room(&axis);
        let room = step - own;
        let with_ticks = !req.shared_axis && room >= 8.0;
        let (own, room) = if with_ticks || req.shared_axis {
            (own, room.max(0.0))
        } else {
            (0.0, step)
        };
        let thickness = if req.vertical {
            (room * 0.6).min(font * TRACK * 1.3)
        } else {
            (room * 0.8).min(font * TRACK)
        };
        // The track and its tick labels, centered in the row.
        let start = index as f32 * step + (step - thickness - own) / 2.0;
        let (cell, track) = if req.vertical {
            (
                Rect {
                    x: first + index as f32 * step,
                    y: 0.0,
                    width: step,
                    height,
                },
                Rect {
                    x: first + start + own,
                    y: pad,
                    width: thickness,
                    height: length,
                },
            )
        } else {
            (
                Rect {
                    x: 0.0,
                    y: index as f32 * step,
                    width,
                    height: step,
                },
                Rect {
                    x: 0.0,
                    y: start,
                    width: length,
                    height: thickness,
                },
            )
        };
        let scale = ValueScale {
            min: axis.min,
            max: axis.max,
            logarithmic: false,
        };
        let f = |v: f64| scale.fraction(v).clamp(0.0, 1.0) as f32;
        // The part of the track from fraction `a` to `b`, `share` of its
        // thickness wide.
        let span = |a: f32, b: f32, share: f32| {
            let (lo, hi) = (a.min(b), a.max(b));
            let inset = thickness * (1.0 - share) / 2.0;
            if req.vertical {
                Rect {
                    x: track.x + inset,
                    y: track.y + (1.0 - hi) * length,
                    width: thickness - 2.0 * inset,
                    height: (hi - lo) * length,
                }
            } else {
                Rect {
                    x: track.x + lo * length,
                    y: track.y + inset,
                    width: (hi - lo) * length,
                    height: thickness - 2.0 * inset,
                }
            }
        };

        let mut from = axis.min;
        let mut value_band = None;
        for (band_index, to) in sorted_bands(&row.bands) {
            let to = to.min(axis.max);
            if to > from {
                out.bands.push(BulletBand {
                    row: index,
                    index: band_index,
                    rect: span(f(from), f(to), 1.0),
                });
                if row.value >= from && (row.value < to || (to >= axis.max && row.value <= to)) {
                    value_band = Some(band_index);
                }
                from = to;
            }
        }

        let base = f(0.0);
        let end = if row.value.is_finite() {
            base + (f(row.value) - base) * progress
        } else {
            base
        };
        let bar = span(base, end, BAR);
        let overflow = if !row.value.is_finite() {
            0
        } else if row.value > axis.max {
            1
        } else if row.value < axis.min {
            -1
        } else {
            0
        };
        // An arrow past the end of the bar, pointing off the axis.
        let size = if req.vertical { bar.width } else { bar.height };
        let reach = (size * 0.7).min(pad);
        let mut cue = String::new();
        if overflow != 0 && size > 0.0 && progress >= 1.0 {
            let sign = overflow as f32;
            if req.vertical {
                let (cx, y) = (
                    bar.x + bar.width / 2.0,
                    if overflow > 0 {
                        bar.y
                    } else {
                        bar.y + bar.height
                    },
                );
                let _ = write!(
                    cue,
                    "M {:.2} {y:.2} L {:.2} {y:.2} L {cx:.2} {:.2} Z",
                    bar.x,
                    bar.x + bar.width,
                    y - sign * reach
                );
            } else {
                let (cy, x) = (
                    bar.y + bar.height / 2.0,
                    if overflow > 0 {
                        bar.x + bar.width
                    } else {
                        bar.x
                    },
                );
                let _ = write!(
                    cue,
                    "M {x:.2} {:.2} L {x:.2} {:.2} L {:.2} {cy:.2} Z",
                    bar.y,
                    bar.y + bar.height,
                    x + sign * reach
                );
            }
        }
        let value_label = value_text(row).filter(|_| req.show_values).map(|text| {
            if req.vertical {
                let below = if overflow < 0 { reach } else { 0.0 };
                Label {
                    x: track.x + thickness / 2.0,
                    y: track.y + length + 4.0 + below,
                    text,
                    align: 0.5,
                    valign: 0.0,
                }
            } else {
                let past = if overflow > 0 { reach } else { 0.0 };
                Label {
                    x: track.x + length + 6.0 + past,
                    y: track.y + thickness / 2.0,
                    text,
                    align: 0.0,
                    valign: 0.5,
                }
            }
        });
        let target = (row.target.is_finite() && row.target >= axis.min && row.target <= axis.max)
            .then(|| {
                let line = if thickness < 12.0 { 2.0 } else { 3.0 };
                let at = f(row.target);
                let mut r = span(at, at, 0.75);
                if req.vertical {
                    r.y -= line / 2.0;
                    r.height = line;
                } else {
                    r.x -= line / 2.0;
                    r.width = line;
                }
                r
            });

        // Each row's own ticks, or the shared ones once, under the last row
        // or left of the first column.
        if with_ticks || (req.shared_axis && index == 0) {
            for tick in &axis.ticks {
                let at = if req.vertical {
                    track.y + tick.pos
                } else {
                    track.x + length - tick.pos
                };
                if req.vertical {
                    let x = if req.shared_axis {
                        first - 2.0
                    } else {
                        track.x - 2.0
                    };
                    let _ = write!(
                        out.tick_marks,
                        "M {:.2} {at:.2} L {:.2} {at:.2} ",
                        x - 4.0,
                        x
                    );
                    out.ticks.push(Label {
                        x: x - tick_gap,
                        y: at,
                        text: tick.label.clone(),
                        align: 1.0,
                        valign: 0.5,
                    });
                } else {
                    let y = if req.shared_axis {
                        n as f32 * step + 2.0
                    } else {
                        track.y + thickness + 2.0
                    };
                    let _ = write!(out.tick_marks, "M {at:.2} {y:.2} L {at:.2} {:.2} ", y + 4.0);
                    out.ticks.push(Label {
                        x: at,
                        y: y + tick_gap,
                        text: tick.label.clone(),
                        align: 0.5,
                        valign: 0.0,
                    });
                }
            }
        }

        out.rows.push(BulletRow {
            index,
            cell,
            track,
            bar,
            target,
            value_band,
            value_label,
            overflow,
            cue,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    static FORMAT: std::sync::LazyLock<Format> = std::sync::LazyLock::new(Format::default);

    fn row(value: f64, target: f64, bands: &[f64]) -> Row {
        Row {
            value,
            target,
            bands: bands.to_vec(),
            format: Format::default(),
        }
    }

    fn request(rows: &[Row]) -> BulletRequest<'_> {
        BulletRequest {
            rows,
            width: 400.0,
            height: 200.0,
            vertical: false,
            shared_axis: false,
            format: &FORMAT,
            fixed: None,
            font_size: 10.0,
            progress: 1.0,
            show_values: false,
        }
    }

    #[test]
    fn rows_have_a_bar_a_target_and_bands() {
        let rows = [row(60.0, 80.0, &[50.0, 75.0, 100.0]), row(30.0, 20.0, &[])];
        let bullet = layout(&request(&rows));
        // As tall as a row needs, not half the height: 19px of track, 9px
        // of gap and 20px of tick labels.
        assert_eq!(bullet.step, 48.0);
        assert_eq!(bullet.step, row_height(10.0, false));
        let first = &bullet.rows[0];
        // 0 to 100, where the last band ends, leaving half of "100" at the
        // right: 391px long.
        assert_eq!(first.track.width, 391.0);
        assert_eq!(first.track.height, 19.0);
        assert_eq!(first.track.y, 4.5);
        assert_eq!(first.bar.x, 0.0);
        assert!((first.bar.width - 0.6 * 391.0).abs() < 1e-3);
        assert!((first.bar.height - 19.0 * BAR).abs() < 1e-3);
        assert!((first.bar.y + first.bar.height / 2.0 - 14.0).abs() < 1e-3);
        let target = first.target.unwrap();
        assert!((target.x + target.width / 2.0 - 0.8 * 391.0).abs() < 1e-3);
        assert_eq!(first.value_band, Some(1));
        let bands: Vec<_> = bullet.bands.iter().map(|b| (b.row, b.index)).collect();
        assert_eq!(bands, [(0, 0), (0, 1), (0, 2)]);
        assert!((bullet.bands[1].rect.x - 0.5 * 391.0).abs() < 1e-3);
        // Each row has its own axis, labelled under the track, with a few
        // ticks.
        assert!(bullet.ticks.iter().any(|t| t.text == "100"));
        assert!(bullet.ticks.iter().any(|t| t.y > 48.0), "the second row's");
        assert!(bullet.ticks.iter().filter(|t| t.y < 48.0).count() <= 5);
        assert_eq!(bullet.ticks[0].y, 4.5 + 19.0 + 2.0 + 6.0);
        assert!(first.value_label.is_none());
        assert_eq!(first.overflow, 0);
    }

    #[test]
    fn rows_without_bands_end_at_a_tick() {
        let rows = [row(30.0, 20.0, &[]), row(30.0, 20.0, &[25.0])];
        let bullet = layout(&request(&rows));
        let ends: Vec<_> = bullet
            .ticks
            .iter()
            .filter(|t| (t.x - bullet.rows[0].track.width).abs() < 1e-3)
            .map(|t| t.text.as_str())
            .collect();
        assert!(!ends.is_empty(), "the track ends at a tick");
        // A band that ends below the value doesn't end the axis.
        assert!(bullet.rows[1].bar.width < bullet.rows[1].track.width);
    }

    #[test]
    fn values_are_written_past_the_tracks() {
        let rows = [row(270.0, 250.0, &[300.0]), row(4.5, 4.2, &[5.0])];
        let mut req = request(&rows);
        req.show_values = true;
        let bullet = layout(&req);
        let (a, b) = (&bullet.rows[0], &bullet.rows[1]);
        // Room for "270", the widest value, and a gap: the tracks line up.
        assert_eq!(a.track.width, 400.0 - 18.0 - 8.0);
        assert_eq!(a.track.width, b.track.width);
        let label = a.value_label.as_ref().unwrap();
        assert_eq!(label.text, "270");
        assert_eq!(label.x, a.track.width + 6.0);
        assert_eq!(label.y, a.track.y + a.track.height / 2.0);
        assert_eq!((label.align, label.valign), (0.0, 0.5));
        assert_eq!(b.value_label.as_ref().unwrap().text, "4.5");
        // Not for a value that isn't a number.
        let rows = [row(f64::NAN, 1.0, &[])];
        let mut req = request(&rows);
        req.show_values = true;
        assert!(layout(&req).rows[0].value_label.is_none());
    }

    #[test]
    fn values_are_written_under_columns() {
        let rows = [row(50.0, 100.0, &[100.0])];
        let mut req = request(&rows);
        (req.vertical, req.show_values) = (true, true);
        let r = &layout(&req).rows[0];
        let label = r.value_label.as_ref().unwrap();
        assert_eq!(label.y, r.track.y + r.track.height + 4.0);
        assert_eq!(label.x, r.track.x + r.track.width / 2.0);
        assert_eq!(
            r.track.y + r.track.height,
            200.0 - 18.0,
            "room for the label"
        );
    }

    #[test]
    fn a_shared_axis_is_labelled_once() {
        let rows = [row(60.0, 80.0, &[]), row(150.0, 120.0, &[])];
        let mut req = request(&rows);
        req.shared_axis = true;
        let bullet = layout(&req);
        assert_eq!(bullet.step, 28.0, "no tick labels in the rows");
        assert!(bullet.ticks.iter().all(|t| t.y == 56.0 + 2.0 + 6.0));
        let (a, b) = (&bullet.rows[0], &bullet.rows[1]);
        assert_eq!(a.track.width, b.track.width);
        assert!((b.bar.width / a.bar.width - 2.5).abs() < 1e-3);
    }

    #[test]
    fn few_rows_share_out_the_height_at_most() {
        let rows = vec![row(1.0, 2.0, &[3.0]); 20];
        let bullet = layout(&request(&rows));
        assert_eq!(bullet.step, 10.0);
        assert!(bullet.rows.iter().all(|r| r.track.height <= 8.0));
    }

    #[test]
    fn vertical_bars_grow_up() {
        let rows = [row(50.0, 100.0, &[100.0])];
        let mut req = request(&rows);
        req.vertical = true;
        let bullet = layout(&req);
        let r = &bullet.rows[0];
        assert_eq!(r.track.y, 7.0);
        assert_eq!(r.track.height, 186.0);
        assert!((r.bar.y - (7.0 + 93.0)).abs() < 1e-3);
        assert!((r.bar.height - 93.0).abs() < 1e-3);
        assert!(
            r.track.x - r.cell.x > 20.0,
            "the tick labels are on the left"
        );
        assert_eq!(bullet.ticks[0].align, 1.0);
        let target = r.target.unwrap();
        assert!((target.y + target.height / 2.0 - 7.0).abs() < 1e-3);
    }

    #[test]
    fn columns_on_a_shared_axis_are_centered() {
        let rows = [row(50.0, 100.0, &[100.0]), row(80.0, 100.0, &[100.0])];
        let mut req = request(&rows);
        (req.vertical, req.shared_axis) = (true, true);
        let bullet = layout(&req);
        let (a, b) = (&bullet.rows[0], &bullet.rows[1]);
        // Columns at most 7 font sizes wide, beside the axis labels, with as
        // much room left and right of them all.
        assert_eq!(bullet.step, 70.0);
        let widest = text_width("100", 10.0) + 6.0 + 2.0;
        assert!((a.cell.x - widest - (400.0 - b.cell.x - b.cell.width)).abs() < 1e-3);
        assert_eq!(b.cell.x - a.cell.x, 70.0);
        assert!((b.bar.height / a.bar.height - 1.6).abs() < 1e-3);
        // The ticks once, right-aligned left of the first column.
        assert!(bullet.ticks.iter().all(|t| t.x == a.cell.x - 2.0 - 6.0));
    }

    #[test]
    fn negative_values_grow_left_from_zero() {
        let rows = [row(-50.0, f64::NAN, &[])];
        let bullet = layout(&request(&rows));
        let r = &bullet.rows[0];
        assert!(r.target.is_none(), "no target");
        assert!(r.bar.width > 0.0 && (r.bar.x + r.bar.width - r.track.width).abs() < 1e-3);
    }

    #[test]
    fn a_zero_target_counts_as_none() {
        assert!(target(0.0, false).is_nan());
        assert_eq!(target(0.0, true), 0.0);
        assert_eq!(target(-3.0, false), -3.0);
        let rows = [row(5.0, target(0.0, false), &[10.0])];
        assert!(layout(&request(&rows)).rows[0].target.is_none());
        let rows = [row(5.0, target(0.0, true), &[10.0])];
        assert!(layout(&request(&rows)).rows[0].target.is_some());
    }

    #[test]
    fn bars_below_their_target_can_change_color() {
        let (own, chart, below) = ([1, 1, 1, 255], [2, 2, 2, 255], [3, 3, 3, 255]);
        let none = [0, 0, 0, 0];
        let low = row(5.0, 10.0, &[]);
        let high = row(15.0, 10.0, &[]);
        let untargeted = row(5.0, f64::NAN, &[]);
        assert_eq!(bar_color(&low, none, chart, none), chart);
        assert_eq!(bar_color(&low, own, chart, none), own);
        assert_eq!(bar_color(&low, own, chart, below), below);
        assert_eq!(bar_color(&high, own, chart, below), own);
        assert_eq!(bar_color(&high, none, chart, below), chart);
        assert_eq!(bar_color(&untargeted, none, chart, below), chart);
    }

    #[test]
    fn bands_in_any_order() {
        let rows = [row(60.0, 80.0, &[100.0, 50.0, f64::NAN, 75.0])];
        let bullet = layout(&request(&rows));
        let order: Vec<_> = bullet.bands.iter().map(|b| b.index).collect();
        assert_eq!(order, [1, 3, 0], "low to high, keeping their indices");
        let rects: Vec<_> = bullet.bands.iter().map(|b| b.rect).collect();
        assert_eq!(rects[0].x, 0.0);
        for pair in rects.windows(2) {
            assert!((pair[0].x + pair[0].width - pair[1].x).abs() < 1e-3);
        }
        assert_eq!(bullet.rows[0].value_band, Some(3));
    }

    #[test]
    fn values_off_a_fixed_axis_get_an_arrow() {
        let rows = [
            row(150.0, 80.0, &[100.0]),
            row(-20.0, 80.0, &[100.0]),
            row(50.0, 80.0, &[100.0]),
        ];
        let mut req = request(&rows);
        req.fixed = Some((0.0, 100.0));
        req.show_values = true;
        let bullet = layout(&req);
        let (over, under, inside) = (&bullet.rows[0], &bullet.rows[1], &bullet.rows[2]);
        assert_eq!((over.overflow, under.overflow, inside.overflow), (1, -1, 0));
        assert!((over.bar.width - over.track.width).abs() < 1e-3, "clamped");
        assert!(over.cue.starts_with('M') && under.cue.starts_with('M'));
        assert!(inside.cue.is_empty());
        // The label moves past the arrow.
        let past = over.value_label.as_ref().unwrap().x;
        assert!(past > inside.value_label.as_ref().unwrap().x);
        assert_eq!(over.value_label.as_ref().unwrap().text, "150");
        // Not while the bars grow in.
        req.progress = 0.5;
        assert!(layout(&req).rows[0].cue.is_empty());
    }

    #[test]
    fn the_entry_animation_grows_the_bars() {
        let rows = [row(100.0, 50.0, &[])];
        let mut req = request(&rows);
        req.progress = 0.5;
        let r = &layout(&req).rows[0];
        assert!((r.bar.width - r.track.width / 2.0).abs() < 1e-3);
    }

    #[test]
    fn compared_to_the_target() {
        let mut r = row(270.0, 250.0, &[]);
        r.format.prefix = "$".into();
        assert_eq!(delta(&r).as_deref(), Some("+$20"));
        assert_eq!(of_target(&r), Some(108.0));
        r.value = 200.0;
        assert_eq!(delta(&r).as_deref(), Some("-$50"));
        r.target = f64::NAN;
        assert_eq!((delta(&r), of_target(&r)), (None, None));
        r.target = 0.0;
        assert_eq!(of_target(&r), None, "no percent of nothing");
        r.target = -10.0;
        assert_eq!(of_target(&r), None);
    }

    #[test]
    fn odd_data_gives_finite_shapes() {
        // Values come from Slint as f32.
        let big = f32::MAX as f64;
        let odd = [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            big,
            -big,
            1e-30,
            0.0,
        ];
        let rows: Vec<Row> = odd
            .iter()
            .flat_map(|&v| odd.iter().map(move |&t| row(v, t, &[t, v, big, -big])))
            .collect();
        for vertical in [false, true] {
            for shared_axis in [false, true] {
                for show_values in [false, true] {
                    for (width, height) in [(400.0, 300.0), (1.0, 1.0), (f32::MAX, f32::NAN)] {
                        for fixed in [None, Some((f64::NAN, f64::INFINITY)), Some((0.0, 1.0))] {
                            let mut req = request(&rows);
                            (req.vertical, req.shared_axis) = (vertical, shared_axis);
                            (req.width, req.height) = (width, height);
                            (req.fixed, req.show_values) = (fixed, show_values);
                            let bullet = layout(&req);
                            let rects = bullet
                                .rows
                                .iter()
                                .flat_map(|r| [r.cell, r.track, r.bar].into_iter().chain(r.target));
                            for r in rects.chain(bullet.bands.iter().map(|b| b.rect)) {
                                assert!(
                                    [r.x, r.y, r.width, r.height].iter().all(|v| v.is_finite()),
                                    "{r:?}"
                                );
                                assert!(r.width >= 0.0 && r.height >= 0.0, "{r:?}");
                            }
                            for l in bullet.rows.iter().filter_map(|r| r.value_label.as_ref()) {
                                assert!(l.x.is_finite() && l.y.is_finite(), "{l:?}");
                            }
                            for r in &bullet.rows {
                                assert!(!r.cue.contains("NaN") && !r.cue.contains("inf"));
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(layout(&request(&[])), Bullet::default());
    }

    #[test]
    fn shades_step_from_dark_to_light() {
        assert_eq!(shade(0, 3, [0, 0, 0, 255])[3], 82);
        assert_eq!(shade(1, 3, [0, 0, 0, 255])[3], 55);
        assert_eq!(shade(2, 3, [0, 0, 0, 255])[3], 28);
        assert_eq!(shade(0, 1, [0, 0, 0, 255])[3], 51);
        assert_eq!(
            over([0, 0, 0, 51], [255, 255, 255, 255]),
            [204, 204, 204, 255]
        );
        assert_eq!(over([10, 20, 30, 255], [255, 0, 0, 255]), [10, 20, 30, 255]);
    }
}
