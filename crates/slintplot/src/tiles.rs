//! Geometry for tile grids: a matrix drawn as rows of rounded squares, one
//! per value. The matrix sets the size: as many rows as it has, and as many
//! columns as its longest row. The tiles stay square, as large as fit, and
//! the grid is centered. Cells are colored as in a segmented ring chart, on
//! a scale or by category (see [`crate::segmented::Coloring`]); missing cells
//! are left as a faint track.

use std::fmt::Write as _;

use crate::layout::Rgba;
use crate::segmented::{Coloring, paint_cells};

/// The most rows and columns drawn, so that a huge matrix can't make the
/// chart unusably slow.
pub const MAX_ROWS: usize = 200;
pub const MAX_COLUMNS: usize = 1000;

pub struct TileRequest<'a> {
    /// The matrix, one row per row of tiles from the top; rows may differ in
    /// length, and NaN is a missing value.
    pub values: &'a [Vec<f64>],
    pub width: f32,
    pub height: f32,
    /// The gap between tiles and the corner radius, as shares of a tile's
    /// pitch and side.
    pub gap: f32,
    pub radius: f32,
    /// The width of the widest row label, at the left, and the size of the
    /// widest column label, below; 0 for none.
    pub row_label_width: f32,
    pub column_label_width: f32,
    pub label_height: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tiles {
    pub rows: usize,
    pub columns: usize,
    /// The top left of the first tile's cell, and the distance from one cell
    /// to the next; a tile is `size` square, with `radius` corners, centered
    /// in its cell.
    pub x: f32,
    pub y: f32,
    pub pitch: f32,
    pub size: f32,
    pub radius: f32,
    /// Every how many columns there is a column label, and every how many
    /// rows a row label, so that they don't overlap.
    pub label_skip: usize,
    pub row_label_skip: usize,
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

pub fn layout(req: &TileRequest) -> Tiles {
    let size = |v: f32| finite_or(v, 0.0).max(0.0);
    let (width, height) = (size(req.width), size(req.height));
    let gap = finite_or(req.gap, 0.18).clamp(0.0, 0.9);
    let radius = finite_or(req.radius, 0.18).clamp(0.0, 0.5);
    let (rows, columns) = dimensions(req.values);
    let label_h = size(req.label_height);
    let row_labels = size(req.row_label_width);
    let column_labels = size(req.column_label_width);
    // Room for the labels, and a little space between them and the tiles.
    let left = if row_labels > 0.0 {
        row_labels + label_h * 0.6
    } else {
        0.0
    };
    let bottom = if column_labels > 0.0 {
        label_h * 1.4
    } else {
        0.0
    };
    let (avail_w, avail_h) = ((width - left).max(0.0), (height - bottom).max(0.0));
    let pitch = (avail_w / columns as f32).min(avail_h / rows as f32);
    let (grid_w, grid_h) = (pitch * columns as f32, pitch * rows as f32);
    // Centered, the labels included.
    let x = left + (avail_w - grid_w) / 2.0;
    let y = (avail_h - grid_h) / 2.0;
    let tile = pitch * (1.0 - gap);
    let label_skip = if column_labels > 0.0 && pitch > 0.0 {
        ((column_labels + label_h * 0.5) / pitch).ceil().max(1.0) as usize
    } else {
        1
    }
    .min(columns);
    let row_label_skip = if row_labels > 0.0 && pitch > 0.0 {
        ((label_h * 1.1) / pitch).ceil().max(1.0) as usize
    } else {
        1
    }
    .min(rows);
    Tiles {
        rows,
        columns,
        x,
        y,
        pitch,
        size: tile,
        radius: tile * radius,
        label_skip,
        row_label_skip,
    }
}

/// The top left of a tile.
pub fn tile_origin(tiles: &Tiles, row: usize, column: usize) -> (f32, f32) {
    let inset = (tiles.pitch - tiles.size) / 2.0;
    (
        tiles.x + column as f32 * tiles.pitch + inset,
        tiles.y + row as f32 * tiles.pitch + inset,
    )
}

fn push_tile(out: &mut String, tiles: &Tiles, row: usize, column: usize) {
    if row >= tiles.rows || column >= tiles.columns || tiles.size <= 0.0 {
        return;
    }
    let (x, y) = tile_origin(tiles, row, column);
    let (s, r) = (tiles.size, tiles.radius.clamp(0.0, tiles.size / 2.0));
    if !out.is_empty() {
        out.push(' ');
    }
    // Counterclockwise on screen and back to the start before closing, so
    // that Slint's femtovg renderer, which works out from the points whether
    // a shape is solid or a hole, takes every tile as solid; as a hole, its
    // smoothed edge would fall outside it and narrow the gaps.
    // Corners under a pixel can't be seen, and their curves are most of the
    // drawing time of grids of tiny tiles: draw those as squares.
    if r < 1.0 {
        let _ = write!(
            out,
            "M {x:.2} {y:.2} v {s:.2} h {s:.2} v {:.2} h {:.2} Z",
            -s, -s
        );
        return;
    }
    let side = s - 2.0 * r;
    let _ = write!(
        out,
        "M {:.2} {y:.2} a {r:.2} {r:.2} 0 0 0 {:.2} {r:.2} v {side:.2} \
         a {r:.2} {r:.2} 0 0 0 {r:.2} {r:.2} h {side:.2} a {r:.2} {r:.2} 0 0 0 {r:.2} {:.2} \
         v {:.2} a {r:.2} {r:.2} 0 0 0 {:.2} {:.2} h {:.2} Z",
        x + r,
        -r,
        -r,
        -side,
        -r,
        -r,
        -side,
    );
}

/// One tile, as a path.
pub fn tile_path(tiles: &Tiles, row: usize, column: usize) -> String {
    let mut out = String::new();
    if tiles.x.is_finite() && tiles.y.is_finite() {
        push_tile(&mut out, tiles, row, column);
    }
    out
}

/// The cell at `(x, y)`, as (row, column), gaps included.
pub fn hit(tiles: &Tiles, x: f32, y: f32) -> Option<(usize, usize)> {
    if tiles.pitch <= 0.0 || !x.is_finite() || !y.is_finite() {
        return None;
    }
    let (cx, cy) = ((x - tiles.x) / tiles.pitch, (y - tiles.y) / tiles.pitch);
    if cx < 0.0 || cy < 0.0 {
        return None;
    }
    let (column, row) = (cx as usize, cy as usize);
    (row < tiles.rows && column < tiles.columns).then_some((row, column))
}

/// The tiles as one path per color, and the empty ones as the track; the
/// columns light up from the left with `progress`.
pub fn paint(
    tiles: &Tiles,
    values: &[Vec<f64>],
    coloring: &Coloring,
    progress: f32,
) -> (String, Vec<(Rgba, String)>) {
    if !tiles.x.is_finite() || !tiles.y.is_finite() {
        return (String::new(), Vec::new());
    }
    paint_cells(
        tiles.rows,
        tiles.columns,
        values,
        coloring,
        progress,
        |out, row, column| push_tile(out, tiles, row, column),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(values: &[Vec<f64>]) -> TileRequest<'_> {
        TileRequest {
            values,
            width: 400.0,
            height: 200.0,
            gap: 0.2,
            radius: 0.2,
            row_label_width: 0.0,
            column_label_width: 0.0,
            label_height: 0.0,
        }
    }

    fn matrix(rows: usize, columns: usize) -> Vec<Vec<f64>> {
        vec![vec![0.0; columns]; rows]
    }

    const CLASSES: [Rgba; 2] = [[255, 0, 0, 255], [0, 0, 255, 255]];

    #[test]
    fn every_tile_is_solid_for_slint() {
        // See pie::slint_solid: tiles Slint takes as holes are drawn a
        // pixel larger all round, narrowing the gaps.
        for (rows, columns, width) in [(7, 26, 600.0), (200, 1000, 400.0), (1, 1, 50.0)] {
            let values = vec![vec![1.0; columns]; rows];
            let tiles = layout(&TileRequest {
                values: &values,
                width,
                height: width,
                gap: 0.18,
                radius: 0.18,
                row_label_width: 0.0,
                column_label_width: 0.0,
                label_height: 0.0,
            });
            for (row, column) in [(0, 0), (rows - 1, columns - 1), (rows / 2, columns / 2)] {
                let path = tile_path(&tiles, row, column);
                assert!(!path.is_empty());
                assert!(
                    crate::pie::slint_solid(&path),
                    "tile {row}, {column} is a hole: {path}"
                );
            }
        }
    }

    #[test]
    fn square_tiles_fill_the_space_centered() {
        let m = matrix(7, 13);
        let tiles = layout(&request(&m));
        assert_eq!((tiles.rows, tiles.columns), (7, 13));
        // 13 columns in 400 px and 7 rows in 200 px: the height decides.
        assert!((tiles.pitch - 200.0 / 7.0).abs() < 1e-3);
        assert!((tiles.size - tiles.pitch * 0.8).abs() < 1e-3);
        assert!((tiles.radius - tiles.size * 0.2).abs() < 1e-3);
        let grid_w = tiles.pitch * 13.0;
        assert!((tiles.x - (400.0 - grid_w) / 2.0).abs() < 1e-3);
        assert!(tiles.y.abs() < 1e-3);
        // The matrix sets the size: ragged rows take the longest.
        assert_eq!(dimensions(&[vec![1.0; 3], vec![1.0; 30]]), (2, 30));
        assert_eq!(dimensions(&[]), (1, 1));
        assert_eq!(dimensions(&matrix(300, 2000)), (MAX_ROWS, MAX_COLUMNS));
    }

    #[test]
    fn labels_get_room_and_are_thinned() {
        let m = matrix(7, 52);
        let mut req = request(&m);
        req.width = 600.0;
        req.row_label_width = 30.0;
        req.column_label_width = 28.0;
        req.label_height = 14.0;
        let tiles = layout(&req);
        assert!(tiles.x >= 30.0);
        assert!(tiles.y + tiles.pitch * 7.0 <= 200.0 - 14.0);
        assert!(tiles.label_skip > 1, "{}", tiles.label_skip);
        // Rows of 10.8 px for labels of 14: every second one.
        assert_eq!(tiles.row_label_skip, 2, "rows of {} px", tiles.pitch);
        let few = matrix(3, 4);
        req.values = &few;
        assert_eq!(layout(&req).label_skip, 1);
        // Rows shorter than the labels: every second or third one.
        let flat = matrix(8, 48);
        req.values = &flat;
        req.height = 90.0;
        let tiles = layout(&req);
        assert!(tiles.pitch < 14.0 && tiles.row_label_skip > 1, "{tiles:?}");
    }

    #[test]
    fn tiles_are_hit_and_drawn() {
        let m = matrix(2, 3);
        let tiles = layout(&request(&m));
        let (x, y) = tile_origin(&tiles, 1, 2);
        assert_eq!(hit(&tiles, x + 1.0, y + 1.0), Some((1, 2)));
        assert_eq!(hit(&tiles, tiles.x - 1.0, y), None);
        assert_eq!(hit(&tiles, 399.9, 199.9), None, "right of the grid");
        let p = tile_path(&tiles, 0, 0);
        assert_eq!(p.matches(" a ").count(), 4, "{p}");
        assert!(tile_path(&tiles, 5, 0).is_empty());
        let mut square = tiles.clone();
        square.radius = 0.0;
        assert!(!tile_path(&square, 0, 0).contains(" a "));
        // Tiny tiles are squares: their corners can't be seen.
        square.radius = 0.6;
        assert!(!tile_path(&square, 0, 0).contains(" a "));
    }

    #[test]
    fn cells_are_grouped_by_color() {
        let nan = f64::NAN;
        let values = vec![vec![0.0, 1.0, nan], vec![1.0]];
        let tiles = layout(&request(&values));
        let coloring = Coloring::Classes {
            colors: &CLASSES,
            hidden: &[],
        };
        let (track, groups) = paint(&tiles, &values, &coloring, 1.0);
        // The missing cell and the short row's two.
        assert_eq!(track.matches('M').count(), 3);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[1].1.matches('M').count(), 2);
        // Sweeping in from the left.
        let (track, _) = paint(&tiles, &values, &coloring, 0.3);
        assert_eq!(track.matches('M').count(), 4);
    }

    #[test]
    fn bad_input_gives_finite_shapes() {
        let row = vec![f64::NAN, f64::INFINITY, -5.0, 1e300, 0.0];
        for (size, gap, rows) in [
            (300.0, 0.2, 5),
            (f32::NAN, f32::NAN, 0),
            (0.0, 5.0, 1),
            (-10.0, -1.0, 400),
            (1e7, 0.9, 1),
        ] {
            let values = vec![row.clone(); rows];
            let mut req = request(&values);
            (req.width, req.height, req.gap, req.radius) = (size, size, gap, gap);
            (
                req.row_label_width,
                req.column_label_width,
                req.label_height,
            ) = (size, size, 14.0);
            let tiles = layout(&req);
            for v in [tiles.x, tiles.y, tiles.pitch, tiles.size, tiles.radius] {
                assert!(v.is_finite(), "{tiles:?}");
            }
            assert!(tiles.label_skip >= 1 && tiles.row_label_skip >= 1);
            let coloring = Coloring::Classes {
                colors: &CLASSES,
                hidden: &[],
            };
            let (track, groups) = paint(&tiles, &values, &coloring, f32::NAN);
            for p in groups.iter().map(|g| &g.1).chain([&track]) {
                assert!(!p.contains("NaN") && !p.contains("inf"));
            }
            let _ = hit(&tiles, 1.0, 1.0);
        }
    }
}
