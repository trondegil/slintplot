//! Geometry for bullet charts, after Stephen Few's design: rows of a thin
//! value bar against a target mark, over shaded ranges such as poor, OK and
//! good, each row on its own value axis or all on one.

use std::fmt::Write;

use crate::layout::{Axis, AxisRequest, Dataset, Kind, Rgba, ValueScale, value_axis};
use crate::radar::Label;
use crate::ticks::Format;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub value: f64,
    /// Where the target mark goes; NaN for none.
    pub target: f64,
    /// Where each band ends. A band starts where the previous one ends, or
    /// at the start of the axis.
    pub bands: Vec<f64>,
    /// How the row's own axis is written.
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

/// The default color of band `index` of `count`: `color`, more transparent
/// for each band, so the first (often "poor") is the darkest.
pub fn shade(index: usize, count: usize, [r, g, b, _]: Rgba) -> Rgba {
    let t = if count > 1 {
        index.min(count - 1) as f32 / (count - 1) as f32
    } else {
        0.0
    };
    [r, g, b, ((0.34 - 0.22 * t) * 255.0).round() as u8]
}

/// The axis of a row, or of all rows: from zero, or the smallest value
/// below it, to the largest value, target or band end.
fn axis(rows: &[&Row], req: &BulletRequest, length: f32, format: &Format) -> Axis {
    let values: Vec<f64> = rows
        .iter()
        .flat_map(|r| {
            [r.value, r.target]
                .into_iter()
                .chain(r.bands.iter().copied())
        })
        .collect();
    let spacing = req.font_size * if req.vertical { 3.0 } else { 8.0 };
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
            .filter(|&(lo, hi)| lo.is_finite() && hi.is_finite() && hi > lo),
        logarithmic: false,
        format,
        percent: false,
        grace: 0.0,
        side: None,
        align_ticks: 0,
        ordered: false,
    })
}

/// A rough width of a tick label, for the room beside the axis.
fn text_width(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * 0.6
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
    // Room for the tick labels: under each track (or the plot), or to their
    // left when vertical.
    let tick_gap = 6.0;
    let mut out = Bullet::default();
    let widest = |a: &Axis| text_width(&a.widest_label, font);
    let shared_width = if req.vertical {
        if req.shared_axis {
            widest(&axis_of(0, height)) + tick_gap + 2.0
        } else {
            0.0
        }
    } else {
        0.0
    };
    // Along the axis: from 0 to `length`, leaving half a label at the far end.
    let pad = if req.vertical { font * 0.7 } else { 0.0 };
    let length_for = |a: &Axis| {
        if req.vertical {
            (height - 2.0 * pad).max(0.0)
        } else {
            (width - (widest(a) / 2.0).min(width / 4.0)).max(0.0)
        }
    };
    let across = if req.vertical {
        width - shared_width
    } else {
        height
    };
    let label_room = font * 1.4 + tick_gap;
    let below = if req.shared_axis && !req.vertical {
        label_room.min(across / 2.0)
    } else {
        0.0
    };
    let step = (across - below).max(0.0) / n as f32;
    out.step = step;
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
        let own = if req.shared_axis {
            0.0
        } else if req.vertical {
            widest(&axis) + tick_gap + 2.0
        } else {
            label_room
        };
        let room = step - own;
        let with_ticks = !req.shared_axis && room >= 8.0;
        let (own, room) = if with_ticks || req.shared_axis {
            (own, room.max(0.0))
        } else {
            (0.0, step)
        };
        let thickness = (room * 0.75).min(40.0);
        // The track and its tick labels, centered in the row.
        let start = index as f32 * step + (step - thickness - own) / 2.0;
        let (cell, track) = if req.vertical {
            let x = shared_width + start + own;
            (
                Rect {
                    x: shared_width + index as f32 * step,
                    y: 0.0,
                    width: step,
                    height,
                },
                Rect {
                    x,
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
        for (band_index, &to) in row.bands.iter().enumerate() {
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

        let bar = if row.value.is_finite() {
            let base = f(0.0);
            let end = base + (f(row.value) - base) * progress;
            span(base, end, 1.0 / 3.0)
        } else {
            span(0.0, 0.0, 1.0 / 3.0)
        };
        let target = (row.target.is_finite() && row.target >= axis.min && row.target <= axis.max)
            .then(|| {
                let line = if thickness < 12.0 { 2.0 } else { 3.0 };
                let at = f(row.target);
                let mut r = span(at, at, 0.7);
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
                        shared_width - 2.0
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
        }
    }

    #[test]
    fn rows_have_a_bar_a_target_and_bands() {
        let rows = [row(60.0, 80.0, &[50.0, 75.0, 100.0]), row(30.0, 20.0, &[])];
        let bullet = layout(&request(&rows));
        assert_eq!(bullet.step, 100.0);
        let first = &bullet.rows[0];
        // 0 to 100, leaving half of "100" at the right: 391px long.
        assert_eq!(first.track.width, 391.0);
        // At most 40px thick, centered in the row with its 20px of tick
        // labels.
        assert_eq!(first.track.height, 40.0);
        assert_eq!(first.track.y, 20.0);
        assert_eq!(first.bar.x, 0.0);
        assert!((first.bar.width - 0.6 * 391.0).abs() < 1e-3);
        assert!((first.bar.y - 20.0 - 40.0 / 3.0).abs() < 1e-3);
        let target = first.target.unwrap();
        assert!((target.x + target.width / 2.0 - 0.8 * 391.0).abs() < 1e-3);
        assert_eq!(first.value_band, Some(1));
        let bands: Vec<_> = bullet.bands.iter().map(|b| (b.row, b.index)).collect();
        assert_eq!(bands, [(0, 0), (0, 1), (0, 2)]);
        assert!((bullet.bands[1].rect.x - 0.5 * 391.0).abs() < 1e-3);
        // Each row has its own axis, labelled under the track.
        assert!(bullet.ticks.iter().any(|t| t.text == "100"));
        assert!(bullet.ticks.iter().any(|t| t.text == "30"));
        assert_eq!(bullet.ticks[0].y, 20.0 + 40.0 + 2.0 + 6.0);
    }

    #[test]
    fn a_shared_axis_is_labelled_once() {
        let rows = [row(60.0, 80.0, &[]), row(150.0, 120.0, &[])];
        let mut req = request(&rows);
        req.shared_axis = true;
        let bullet = layout(&req);
        assert_eq!(bullet.step, 90.0, "the labels take 20px at the bottom");
        assert!(bullet.ticks.iter().all(|t| t.y == 180.0 + 2.0 + 6.0));
        let (a, b) = (&bullet.rows[0], &bullet.rows[1]);
        assert_eq!(a.track.width, b.track.width);
        assert!((b.bar.width / a.bar.width - 2.5).abs() < 1e-3);
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
        assert!(r.track.x > 20.0, "the tick labels are on the left");
        assert_eq!(bullet.ticks[0].align, 1.0);
        let target = r.target.unwrap();
        assert!((target.y + target.height / 2.0 - 7.0).abs() < 1e-3);
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
    fn the_entry_animation_grows_the_bars() {
        let rows = [row(100.0, 50.0, &[])];
        let mut req = request(&rows);
        req.progress = 0.5;
        let r = &layout(&req).rows[0];
        assert!((r.bar.width - r.track.width / 2.0).abs() < 1e-3);
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
            .flat_map(|&v| odd.iter().map(move |&t| row(v, t, &[t, v, big])))
            .collect();
        for vertical in [false, true] {
            for shared_axis in [false, true] {
                for (width, height) in [(400.0, 300.0), (1.0, 1.0), (f32::MAX, f32::NAN)] {
                    let mut req = request(&rows);
                    (req.vertical, req.shared_axis) = (vertical, shared_axis);
                    (req.width, req.height) = (width, height);
                    req.fixed = Some((f64::NAN, f64::INFINITY));
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
                }
            }
        }
        assert_eq!(layout(&request(&[])), Bullet::default());
    }

    #[test]
    fn shades_fade() {
        assert_eq!(shade(0, 3, [0, 0, 0, 255])[3], 87);
        assert_eq!(shade(2, 3, [0, 0, 0, 255])[3], 31);
        assert_eq!(shade(0, 1, [0, 0, 0, 255])[3], 87);
    }
}
