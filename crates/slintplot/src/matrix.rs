//! Geometry for matrix charts: rows by columns of circles whose area is
//! proportional to the value, like a bubble grid.

use std::fmt::Write;

use crate::layout::{PALETTE, Rgba};
use crate::ticks::Format;

/// How a matrix chart colors its circles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    /// One color for all.
    #[default]
    Single,
    /// A palette color per row.
    Rows,
    /// A palette color per column.
    Columns,
    /// One color for positive values and another for negative ones.
    Sign,
    /// A color scale over a second measure, or over the values.
    Scale,
}

pub struct MatrixRequest<'a> {
    /// Row by row: `values[row * columns + column]`.
    pub values: &'a [f64],
    /// The measure the `Scale` mode colors by; empty to color by `values`.
    pub color_values: &'a [f64],
    pub rows: usize,
    pub columns: usize,
    pub width: f32,
    pub height: f32,
    pub mode: ColorMode,
    pub color: Rgba,
    /// Negative values in the `Sign` mode, and the low end of a diverging
    /// `Scale`.
    pub negative_color: Rgba,
    pub palette: &'a [Rgba],
    /// The color a `Scale` fades towards for small values.
    pub base: Rgba,
    /// The value of a full-size circle, instead of the largest one.
    pub max_value: Option<f64>,
    /// The entry animation, which grows the circles from nothing.
    pub progress: f32,
    pub font_size: f32,
    pub show_values: bool,
    /// A short dash in each cell without a value, so that it differs from
    /// a cell of 0.
    pub show_empty: bool,
    /// A column of row totals and a row of column totals.
    pub show_totals: bool,
    /// The largest circle of the size legend.
    pub legend_radius: f32,
    pub format: &'a Format,
    pub color_format: &'a Format,
}

/// A circle, which may stand for a block of cells when they are too small
/// to draw one by one: then `row` and `column` are the block's first cell.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dot {
    pub row: usize,
    pub column: usize,
    /// The center and radius, in pixels; the radius grows with `progress`.
    pub x: f32,
    pub y: f32,
    pub r: f32,
    /// The radius once grown.
    pub size: f32,
    pub fill: Rgba,
    pub stroke: Rgba,
    pub value: f64,
    /// The value written in or under the circle, with `show_values`.
    pub label: String,
    /// Whether a label inside the circle should be light, on a dark fill.
    pub light_label: bool,
}

/// A circle of the size legend.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reference {
    pub value: f64,
    pub label: String,
    pub r: f32,
    pub fill: Rgba,
    pub stroke: Rgba,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Matrix {
    pub dots: Vec<Dot>,
    /// The size of a cell; the cells fill `grid_width` by `grid_height`,
    /// and the totals, if any, are right of and below them.
    pub cell_width: f32,
    pub cell_height: f32,
    pub grid_width: f32,
    pub grid_height: f32,
    /// Cells merged into each circle, across and down: 1 unless the cells
    /// are too small to draw one by one.
    pub block_columns: usize,
    pub block_rows: usize,
    /// Lines between the cells, as path commands.
    pub separators: String,
    /// The dashes in cells without a value, with `show_empty`, as path
    /// commands.
    pub empty: String,
    /// The totals, when shown and there is room: one per row and column,
    /// and all together.
    pub row_totals: Vec<String>,
    pub column_totals: Vec<String>,
    pub grand_total: String,
    pub total_width: f32,
    pub total_height: f32,
    /// The size legend, smallest first, and a negative circle when there
    /// are negative values.
    pub sizes: Vec<Reference>,
    pub negative: Option<Reference>,
    /// The `Scale` mode's range, with the colors of its ends and middle.
    pub scale_min: f64,
    pub scale_max: f64,
    pub scale_colors: [Rgba; 3],
    pub scale_min_label: String,
    pub scale_max_label: String,
}

/// The smallest cell, in pixels, drawn as a circle of its own; smaller
/// cells are merged into blocks.
const MIN_CELL: f32 = 6.0;
/// The opacity of a circle's fill; the outline is solid.
const FILL_ALPHA: f32 = 0.85;
/// The weakest color of a scale, mixed into its base color.
const MIN_STRENGTH: f32 = 0.4;
/// The fill of hollow circles, for negative values.
const HOLLOW_ALPHA: f32 = 0.12;

fn with_alpha([r, g, b, _]: Rgba, alpha: f32) -> Rgba {
    [r, g, b, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8]
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

/// Relative luminance, from 0 (black) to 1 (white).
fn luminance([r, g, b, _]: Rgba) -> f32 {
    let lin = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// A rough width of a label.
fn text_width(text: &str, font_size: f32) -> f32 {
    text.chars().count() as f32 * font_size * 0.6
}

/// The largest round number at or below `v`: 1, 1.5, 2, 2.5, 3, 4, 5, 6 or 8
/// times a power of ten.
pub fn nice_below(v: f64) -> f64 {
    if !(v.is_finite() && v > 0.0) {
        return 0.0;
    }
    let e = 10f64.powf(v.log10().floor());
    let f = v / e;
    let step = [10.0, 8.0, 6.0, 5.0, 4.0, 3.0, 2.5, 2.0, 1.5, 1.0]
        .into_iter()
        .find(|&s| s <= f * (1.0 + 1e-9))
        .unwrap_or(1.0);
    step * e
}

/// The radius of a circle for `value`: its area is proportional to the
/// value's size, up to `max_r` for `max` and beyond.
pub fn radius(value: f64, max: f64, max_r: f32) -> f32 {
    if !(value.is_finite() && value != 0.0 && max > 0.0 && max.is_finite()) {
        return 0.0;
    }
    let r = max_r * (value.abs() / max).min(1.0).sqrt() as f32;
    // Even the smallest value shows.
    r.max(1.0_f32.min(max_r))
}

/// The value of a cell, or NaN when it has none.
fn value_at(values: &[f64], index: usize) -> f64 {
    values.get(index).copied().unwrap_or(f64::NAN)
}

pub fn layout(req: &MatrixRequest) -> Matrix {
    let rows = req.rows.max(1);
    let columns = req.columns.max(1);
    let finite = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
    let (width, height) = (finite(req.width), finite(req.height));
    let font = if req.font_size.is_finite() {
        req.font_size.clamp(1.0, 200.0)
    } else {
        12.0
    };
    let progress = if req.progress.is_finite() {
        req.progress.clamp(0.0, 1.0)
    } else {
        1.0
    };
    // Only the values that have a cell.
    let count = rows.saturating_mul(columns).min(req.values.len());
    let values = &req.values[..count];
    let largest = values
        .iter()
        .filter(|v| v.is_finite())
        .fold(0.0_f64, |m, v| m.max(v.abs()));
    let max = req
        .max_value
        .map(f64::abs)
        .filter(|m| m.is_finite() && *m > 0.0)
        .unwrap_or(largest);

    // Room for the totals, when the cells are large enough to have them.
    let mut out = Matrix::default();
    let totals = req.show_totals && count > 0;
    let mut row_totals = Vec::new();
    let mut column_totals = Vec::new();
    if totals {
        let guess_w = width / (columns as f32 + 1.0);
        let guess_h = height / (rows as f32 + 1.0);
        if guess_h >= font * 1.2 {
            row_totals = sums(values, rows, columns, true);
        }
        if guess_w >= font * 2.0 {
            column_totals = sums(values, rows, columns, false);
        }
    }
    let total_text = |sum: &f64| req.format.value(*sum);
    if !row_totals.is_empty() {
        let widest = row_totals
            .iter()
            .chain(std::iter::once(&row_totals.iter().sum::<f64>()))
            .map(|s| text_width(&total_text(s), font))
            .fold(0.0, f32::max);
        out.total_width = (widest + font * 1.2).min(width * 0.3);
    }
    if !column_totals.is_empty() {
        out.total_height = (font * 2.0).min(height * 0.25);
    }
    let grid_w = (width - out.total_width).max(0.0);
    let grid_h = (height - out.total_height).max(0.0);
    let cell_w = grid_w / columns as f32;
    let cell_h = grid_h / rows as f32;
    out.cell_width = cell_w;
    out.cell_height = cell_h;
    out.grid_width = grid_w;
    out.grid_height = grid_h;
    if !row_totals.is_empty() {
        out.row_totals = row_totals.iter().map(total_text).collect();
    }
    if !column_totals.is_empty() {
        out.column_totals = column_totals.iter().map(total_text).collect();
    }
    if !row_totals.is_empty() || !column_totals.is_empty() {
        let all: f64 = values.iter().filter(|v| v.is_finite()).sum();
        out.grand_total = req.format.value(all);
    }

    // The circles: one per cell, or per block of cells when they're small.
    let block_columns = if cell_w >= MIN_CELL || cell_w <= 0.0 {
        1
    } else {
        ((MIN_CELL / cell_w).ceil() as usize).max(1)
    };
    let block_rows = if cell_h >= MIN_CELL || cell_h <= 0.0 {
        1
    } else {
        ((MIN_CELL / cell_h).ceil() as usize).max(1)
    };
    out.block_columns = block_columns;
    out.block_rows = block_rows;
    let step_w = cell_w * block_columns as f32;
    let step_h = cell_h * block_rows as f32;
    // A dash in each cell without a value, where cells are drawn one by one.
    if req.show_empty && block_columns == 1 && block_rows == 1 && cell_w > 0.0 && cell_h > 0.0 {
        let half = (cell_w.min(cell_h) * 0.12).clamp(2.0, font * 0.5);
        let mut path = String::new();
        for index in 0..rows * columns {
            if !value_at(values, index).is_finite() {
                let x = (index % columns) as f32 * cell_w + cell_w / 2.0;
                let y = (index / columns) as f32 * cell_h + cell_h / 2.0;
                let _ = write!(path, "M {} {y} L {} {y} ", x - half, x + half);
            }
        }
        out.empty = path;
    }
    // A little room around the largest circle.
    let max_r = (step_w.min(step_h) / 2.0 - (step_w.min(step_h) * 0.06).max(1.0)).max(0.0);

    // The color measure: its range, and how strong each value is.
    let color_values = if req.color_values.is_empty() {
        values
    } else {
        req.color_values
    };
    let (scale_min, scale_max) = crate::heatmap::extent(color_values.iter().take(count).copied())
        .unwrap_or((f64::NAN, f64::NAN));
    let diverging = scale_min < 0.0 && scale_max > 0.0;
    let positive = with_alpha(req.color, 1.0);
    let negative = with_alpha(req.negative_color, 1.0);
    let base = with_alpha(req.base, 1.0);
    let strength = |v: f64| -> f32 {
        if !v.is_finite() {
            return 0.0;
        }
        if diverging {
            let reach = scale_min.abs().max(scale_max.abs());
            (v / reach).clamp(-1.0, 1.0) as f32
        } else if scale_max > scale_min {
            ((v - scale_min) / (scale_max - scale_min)).clamp(0.0, 1.0) as f32
        } else {
            1.0
        }
    };
    // Even the weakest color shows against the background, and the
    // colors grow strong quickly, so the middle values aren't washed out.
    let scale_color = |s: f32| {
        let t = MIN_STRENGTH + (1.0 - MIN_STRENGTH) * s.abs().sqrt();
        mix(base, if s < 0.0 { negative } else { positive }, t)
    };
    let palette = if req.palette.is_empty() {
        &PALETTE[..]
    } else {
        req.palette
    };
    let color_of = |row: usize, column: usize, value: f64, index: usize| -> Rgba {
        match req.mode {
            ColorMode::Single => positive,
            ColorMode::Rows => with_alpha(palette[row % palette.len()], 1.0),
            ColorMode::Columns => with_alpha(palette[column % palette.len()], 1.0),
            ColorMode::Sign => {
                if value < 0.0 {
                    negative
                } else {
                    positive
                }
            }
            ColorMode::Scale => scale_color(strength(value_at(color_values, index))),
        }
    };
    // Negative values are hollow, except where their color tells them apart.
    let paint = |color: Rgba, value: f64| -> (Rgba, Rgba, bool) {
        if value < 0.0 && req.mode != ColorMode::Sign {
            (with_alpha(color, HOLLOW_ALPHA), color, false)
        } else {
            (with_alpha(color, FILL_ALPHA), color, luminance(color) < 0.3)
        }
    };

    if max_r > 0.0 && max > 0.0 {
        let block_count_x = columns.div_ceil(block_columns);
        let block_count_y = rows.div_ceil(block_rows);
        // Bounded by the pixels: blocks are at least `MIN_CELL` wide.
        let cells = block_count_x.saturating_mul(block_count_y);
        let merged = block_columns > 1 || block_rows > 1;
        // The value of each block: the one farthest from zero, so peaks
        // stay visible; with its index.
        let mut blocks: Vec<(f64, usize)> = Vec::new();
        if merged {
            blocks = vec![(f64::NAN, 0); cells];
            for (index, &v) in values.iter().enumerate() {
                if !v.is_finite() {
                    continue;
                }
                let (row, column) = (index / columns, index % columns);
                let slot = (row / block_rows) * block_count_x + column / block_columns;
                if let Some(b) = blocks.get_mut(slot)
                    && (b.0.is_nan() || v.abs() > b.0.abs())
                {
                    *b = (v, index);
                }
            }
        }
        let label_room = !merged && req.show_values && cell_w.min(cell_h) >= font * 1.2;
        let mut push = |row: usize, column: usize, value: f64, index: usize| {
            let size = radius(value, max, max_r);
            let color = color_of(row, column, value, index);
            let (fill, stroke, light) = paint(color, value);
            out.dots.push(Dot {
                row,
                column,
                x: (column as f32 + block_columns as f32 / 2.0) * cell_w,
                y: (row as f32 + block_rows as f32 / 2.0) * cell_h,
                r: size * progress,
                size,
                fill,
                stroke,
                value,
                label: if label_room && size > 0.0 {
                    req.format.value(value)
                } else {
                    String::new()
                },
                light_label: light,
            });
        };
        if merged {
            for (slot, &(value, index)) in blocks.iter().enumerate() {
                let row = slot / block_count_x * block_rows;
                let column = slot % block_count_x * block_columns;
                push(row, column, value, index);
            }
        } else {
            for (index, &value) in values.iter().enumerate() {
                push(index / columns, index % columns, value, index);
            }
        }

        // Lines between the cells, where they are far enough apart.
        if !merged {
            let mut path = String::new();
            for c in 1..columns {
                let x = c as f32 * cell_w;
                let _ = write!(path, "M {x} 0 L {x} {grid_h} ");
            }
            for r in 1..rows {
                let y = r as f32 * cell_h;
                let _ = write!(path, "M 0 {y} L {grid_w} {y} ");
            }
            out.separators = path;
        }

        // The size legend: round values, at the chart's scale, no larger
        // than the legend has room for.
        let cap = if req.legend_radius.is_finite() {
            req.legend_radius.max(0.0)
        } else {
            0.0
        };
        if cap >= 1.0 {
            let reach = if max_r > cap {
                max * ((cap / max_r) as f64).powi(2)
            } else {
                max
            };
            let top = nice_below(reach);
            let mut refs: Vec<f64> = Vec::new();
            for v in [top / 16.0, top / 4.0, top] {
                let v = nice_below(v);
                if v > 0.0 && refs.last().is_none_or(|&last| v > last) {
                    refs.push(v);
                }
            }
            let legend_color = match req.mode {
                ColorMode::Rows | ColorMode::Columns => with_alpha(req.color, 1.0),
                ColorMode::Scale => scale_color(1.0),
                _ => positive,
            };
            out.sizes = refs
                .iter()
                .map(|&v| {
                    let (fill, stroke, _) = paint(legend_color, v);
                    Reference {
                        value: v,
                        label: req.format.value(v),
                        r: radius(v, max, max_r),
                        fill,
                        stroke,
                    }
                })
                .collect();
            if values.iter().any(|&v| v < 0.0)
                && let Some(&v) = refs.last()
            {
                let color = if req.mode == ColorMode::Sign {
                    negative
                } else {
                    legend_color
                };
                let (fill, stroke, _) = paint(color, -v);
                out.negative = Some(Reference {
                    value: -v,
                    label: req.format.value(-v),
                    r: radius(v, max, max_r),
                    fill,
                    stroke,
                });
            }
        }
    }

    out.scale_min = scale_min;
    out.scale_max = scale_max;
    out.scale_colors = if diverging {
        [negative, base, positive]
    } else {
        [scale_color(0.0), scale_color(0.5), scale_color(1.0)]
    };
    if scale_min.is_finite() {
        out.scale_min_label = req.color_format.value(scale_min);
        out.scale_max_label = req.color_format.value(scale_max);
    }
    out
}

/// The sum of each row, or of each column, of the finite values.
fn sums(values: &[f64], rows: usize, columns: usize, by_row: bool) -> Vec<f64> {
    let mut sums = vec![0.0; if by_row { rows } else { columns }];
    for (index, &v) in values.iter().enumerate() {
        if v.is_finite() {
            let slot = if by_row {
                index / columns
            } else {
                index % columns
            };
            if let Some(s) = sums.get_mut(slot) {
                *s += v;
            }
        }
    }
    sums
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLUE: Rgba = [0x3b, 0x82, 0xf6, 255];
    const RED: Rgba = [0xf4, 0x3f, 0x5e, 255];

    fn request<'a>(values: &'a [f64], format: &'a Format) -> MatrixRequest<'a> {
        MatrixRequest {
            values,
            color_values: &[],
            rows: 2,
            columns: 3,
            width: 300.0,
            height: 200.0,
            mode: ColorMode::Single,
            color: BLUE,
            negative_color: RED,
            palette: &[],
            base: [0xee, 0xf0, 0xf3, 255],
            max_value: None,
            progress: 1.0,
            font_size: 12.0,
            show_values: false,
            show_empty: false,
            show_totals: false,
            legend_radius: 14.0,
            format,
            color_format: format,
        }
    }

    #[test]
    fn empty_cells_get_a_dash_but_zero_stays_blank() {
        let format = Format::default();
        // A missing value, a zero, and two cells past the end of `values`.
        let values = [1.0, f64::NAN, 0.0, 2.0];
        let m = layout(&request(&values, &format));
        assert!(m.empty.is_empty(), "off by default");
        let m = layout(&MatrixRequest {
            show_empty: true,
            ..request(&values, &format)
        });
        assert_eq!(m.empty.matches('M').count(), 3, "{}", m.empty);
        let all_missing = layout(&MatrixRequest {
            show_empty: true,
            ..request(&[], &format)
        });
        assert_eq!(all_missing.empty.matches('M').count(), 6);
    }

    #[test]
    fn area_follows_the_value() {
        let format = Format::default();
        let m = layout(&request(
            &[100.0, 25.0, 0.0, f64::NAN, -100.0, 4.0],
            &format,
        ));
        assert_eq!(m.dots.len(), 6);
        assert_eq!((m.cell_width, m.cell_height), (100.0, 100.0));
        let full = m.dots[0].r;
        assert!(full > 40.0 && full < 50.0, "{full}");
        // A quarter of the value, a quarter of the area: half the radius.
        assert!((m.dots[1].r - full / 2.0).abs() < 0.01);
        assert_eq!(m.dots[2].r, 0.0, "zero draws nothing");
        assert_eq!(m.dots[3].r, 0.0, "missing draws nothing");
        assert_eq!(m.dots[4].r, full, "negatives by their size");
        assert_eq!((m.dots[4].row, m.dots[4].column), (1, 1));
        assert_eq!((m.dots[0].x, m.dots[0].y), (50.0, 50.0));
    }

    #[test]
    fn negatives_are_hollow_or_in_their_own_color() {
        let format = Format::default();
        let values = [10.0, -10.0];
        let mut req = request(&values, &format);
        let m = layout(&req);
        assert_eq!(m.dots[1].stroke, BLUE);
        assert!(m.dots[1].fill[3] < 64, "hollow");
        assert!(m.dots[0].fill[3] > 200);
        assert!(m.negative.is_some());
        req.mode = ColorMode::Sign;
        let m = layout(&req);
        assert_eq!(m.dots[1].stroke, RED);
        assert!(m.dots[1].fill[3] > 200);
        assert_eq!(m.negative.unwrap().stroke, RED);
    }

    #[test]
    fn rows_and_columns_take_palette_colors() {
        let format = Format::default();
        let values = [1.0; 6];
        let palette = [BLUE, RED, [0, 200, 0, 255]];
        let mut req = request(&values, &format);
        req.palette = &palette;
        req.mode = ColorMode::Rows;
        let m = layout(&req);
        assert_eq!(m.dots[2].stroke, BLUE);
        assert_eq!(m.dots[3].stroke, RED);
        req.mode = ColorMode::Columns;
        let m = layout(&req);
        assert_eq!(m.dots[2].stroke, [0, 200, 0, 255]);
        assert_eq!(m.dots[3].stroke, BLUE);
    }

    #[test]
    fn a_scale_colors_by_the_second_measure() {
        let format = Format::default();
        let values = [1.0; 6];
        let colors = [-1.0, 0.0, 1.0];
        let mut req = request(&values, &format);
        req.mode = ColorMode::Scale;
        req.color_values = &colors;
        let m = layout(&req);
        assert_eq!(m.dots[0].stroke, RED, "diverging");
        assert_eq!(m.dots[2].stroke, BLUE);
        assert_eq!(m.scale_colors[1], req.base);
        assert_eq!((m.scale_min, m.scale_max), (-1.0, 1.0));
        // A cell without a color value takes the weakest color.
        assert_eq!(m.dots[4].stroke, mix(req.base, BLUE, MIN_STRENGTH));
    }

    #[test]
    fn crowded_cells_merge_into_blocks() {
        let format = Format::default();
        let values: Vec<f64> = (0..500 * 500).map(|i| (i % 7) as f64).collect();
        let mut req = request(&values, &format);
        req.rows = 500;
        req.columns = 500;
        req.width = 1000.0;
        req.height = 600.0;
        req.show_values = true;
        req.show_totals = true;
        let m = layout(&req);
        assert!(m.block_columns > 1 && m.block_rows > 1);
        assert!(m.dots.len() < 20_000, "{}", m.dots.len());
        assert!(m.dots.iter().all(|d| d.label.is_empty()));
        assert!(m.separators.is_empty());
        assert!(m.row_totals.is_empty(), "no room for totals");
        // Each block shows its largest value.
        assert!(m.dots.iter().all(|d| d.value == 6.0));
    }

    #[test]
    fn totals_and_labels_where_there_is_room() {
        let format = Format::default();
        let values = [1.0, 2.0, 3.0, 4.0, f64::NAN, 6.0];
        let mut req = request(&values, &format);
        req.show_totals = true;
        req.show_values = true;
        let m = layout(&req);
        assert_eq!(m.row_totals, ["6", "10"]);
        assert_eq!(m.column_totals, ["5", "2", "9"]);
        assert_eq!(m.grand_total, "16");
        assert!(m.total_width > 0.0 && (m.grid_width + m.total_width - 300.0).abs() < 1e-3);
        assert_eq!(m.dots[5].label, "6");
        assert_eq!(m.dots[4].label, "", "missing");
    }

    #[test]
    fn size_legend_has_round_values_at_scale() {
        let format = Format::default();
        let values = [873.0, 10.0];
        let mut req = request(&values, &format);
        req.rows = 1;
        req.columns = 2;
        req.width = 40.0;
        req.height = 20.0;
        let m = layout(&req);
        let sizes: Vec<f64> = m.sizes.iter().map(|s| s.value).collect();
        assert_eq!(sizes, [50.0, 200.0, 800.0]);
        // Large cells: the legend shows smaller values, at the same scale.
        req.width = 600.0;
        req.height = 300.0;
        let m = layout(&req);
        let top = m.sizes.last().unwrap();
        assert!(top.r <= 14.0 && top.value < 873.0);
        assert!((top.r - radius(top.value, 873.0, m.dots[0].size)).abs() < 1e-3);
    }

    #[test]
    fn nice_numbers() {
        assert_eq!(nice_below(873.0), 800.0);
        assert_eq!(nice_below(1.0), 1.0);
        assert_eq!(nice_below(0.27), 0.25);
        assert_eq!(nice_below(f64::NAN), 0.0);
        assert_eq!(nice_below(-5.0), 0.0);
    }

    #[test]
    fn bad_input_draws_nothing_and_does_not_panic() {
        let format = Format::default();
        for values in [
            &[][..],
            &[f64::NAN, f64::INFINITY, f64::NEG_INFINITY][..],
            &[0.0, 0.0][..],
            &[f64::MAX, -f64::MAX, f64::MIN_POSITIVE][..],
        ] {
            for (w, h) in [
                (0.0, 0.0),
                (-5.0, 10.0),
                (f32::NAN, f32::INFINITY),
                (1.0, 1.0),
            ] {
                let mut req = request(values, &format);
                req.width = w;
                req.height = h;
                req.rows = usize::MAX;
                req.columns = usize::MAX;
                req.show_totals = true;
                req.progress = f32::NAN;
                req.font_size = f32::NAN;
                let m = layout(&req);
                assert!(m.dots.iter().all(|d| d.r.is_finite() && d.x.is_finite()));
            }
        }
        let m = layout(&MatrixRequest {
            max_value: Some(f64::NAN),
            ..request(&[f64::MAX, -f64::MAX], &format)
        });
        assert!(m.dots.iter().all(|d| d.r.is_finite() && d.r > 0.0));
    }
}
