//! Treemaps: rectangles whose areas are proportional to their values, laid
//! out with the squarified algorithm (Bruls, Huizing and van Wijk, 2000), so
//! the rectangles stay close to square and are easy to compare.

/// A rectangle in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    fn inset(self, by: f64) -> Rect {
        Rect {
            x: self.x + by,
            y: self.y + by,
            width: (self.width - 2.0 * by).max(0.0),
            height: (self.height - 2.0 * by).max(0.0),
        }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// An item of a treemap.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    pub value: f64,
    /// Items with the same group are placed together. An empty group puts
    /// the item in a group of its own.
    pub group: String,
}

/// A laid out item.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cell {
    /// Index of the item in the input.
    pub index: usize,
    /// Index of the item's group in [`Treemap::groups`].
    pub group: usize,
    pub rect: Rect,
}

/// A laid out group: its area, and whether it has room for a header.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Group {
    pub label: String,
    pub rect: Rect,
    pub header: bool,
    pub hidden: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Treemap {
    pub cells: Vec<Cell>,
    /// Groups in order of first appearance; empty when no item has a group.
    pub groups: Vec<Group>,
    /// The sum of the visible values.
    pub total: f64,
}

/// The value of an item as drawn: missing, negative and zero values take no
/// space.
fn size(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

/// Lays out `items` in a `width` × `height` area. When items have groups,
/// each group gets an area for its total, with a header of `header` pixels
/// when the area is large enough, and `gap` pixels between groups. Groups
/// whose bit is set in `hidden_groups` take no space.
pub fn layout(
    items: &[Item],
    width: f64,
    height: f64,
    header: f64,
    gap: f64,
    hidden_groups: u32,
) -> Treemap {
    let mut out = Treemap::default();
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width: width.max(0.0),
        height: height.max(0.0),
    };
    let grouped = items.iter().any(|item| !item.group.is_empty());
    if !grouped {
        let sizes: Vec<(usize, f64)> = items
            .iter()
            .map(|item| size(item.value))
            .enumerate()
            .collect();
        out.total = sizes.iter().map(|s| s.1).sum();
        out.cells = squarify(sizes, area)
            .into_iter()
            .map(|(index, rect)| Cell {
                index,
                group: 0,
                rect,
            })
            .collect();
        return out;
    }

    // Groups in order of first appearance, each with its items.
    let mut members: Vec<Vec<(usize, f64)>> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let position = out.groups.iter().position(|g| g.label == item.group);
        let group = position.unwrap_or_else(|| {
            let hidden = out.groups.len() < 32 && hidden_groups >> out.groups.len() & 1 == 1;
            out.groups.push(Group {
                label: item.group.clone(),
                hidden,
                ..Default::default()
            });
            members.push(Vec::new());
            out.groups.len() - 1
        });
        let value = if out.groups[group].hidden {
            0.0
        } else {
            size(item.value)
        };
        members[group].push((index, value));
    }
    let totals: Vec<(usize, f64)> = members
        .iter()
        .map(|m| m.iter().map(|s| s.1).sum())
        .enumerate()
        .collect();
    out.total = totals.iter().map(|t| t.1).sum();
    for (group, rect) in squarify(totals, area) {
        // Half the gap on each side of a group gives `gap` between groups.
        let rect = rect.inset(gap / 2.0);
        let label = &out.groups[group].label;
        // A header only where it leaves most of the area for the items.
        let has_header =
            !label.is_empty() && rect.height >= header * 3.0 && rect.width >= header * 3.0;
        let inner = if has_header {
            Rect {
                y: rect.y + header,
                height: rect.height - header,
                ..rect
            }
        } else {
            rect
        };
        out.groups[group].rect = rect;
        out.groups[group].header = has_header;
        out.cells.extend(
            squarify(members[group].clone(), inner)
                .into_iter()
                .map(|(index, rect)| Cell { index, group, rect }),
        );
    }
    out
}

/// The worst aspect ratio of a row of `areas` laid along a side of `side`.
fn worst(areas: &[f64], side: f64) -> f64 {
    let sum: f64 = areas.iter().sum();
    let (min, max) = areas.iter().fold((f64::INFINITY, 0.0f64), |(lo, hi), &a| {
        (lo.min(a), hi.max(a))
    });
    if sum <= 0.0 || min <= 0.0 {
        return f64::INFINITY;
    }
    let side2 = side * side;
    (side2 * max / (sum * sum)).max(sum * sum / (side2 * min))
}

/// Squarified layout of `(index, value)` pairs in `rect`: rectangles are
/// placed largest first, in rows along the shorter side, and a row grows
/// while that keeps its rectangles closer to square. Items without a
/// positive value are left out.
pub fn squarify(mut items: Vec<(usize, f64)>, rect: Rect) -> Vec<(usize, Rect)> {
    items.retain(|item| item.1 > 0.0);
    items.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let total: f64 = items.iter().map(|item| item.1).sum();
    let mut out = Vec::with_capacity(items.len());
    if total <= 0.0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return out;
    }
    let scale = rect.width * rect.height / total;
    let areas: Vec<f64> = items.iter().map(|item| item.1 * scale).collect();
    let mut free = rect;
    let mut start = 0;
    while start < items.len() {
        let side = free.width.min(free.height);
        let mut end = start + 1;
        while end < items.len()
            && worst(&areas[start..=end], side) <= worst(&areas[start..end], side)
        {
            end += 1;
        }
        // The last row takes all the space that is left, so rounding errors
        // don't leave a sliver.
        let row_area: f64 = if end == items.len() {
            free.width * free.height
        } else {
            areas[start..end].iter().sum()
        };
        let row_sum: f64 = areas[start..end].iter().sum();
        if free.width >= free.height {
            // A column on the left.
            let column = (row_area / free.height).min(free.width);
            let mut y = free.y;
            for i in start..end {
                let height = free.height * areas[i] / row_sum;
                out.push((
                    items[i].0,
                    Rect {
                        x: free.x,
                        y,
                        width: column,
                        height,
                    },
                ));
                y += height;
            }
            free.x += column;
            free.width -= column;
        } else {
            // A row along the top.
            let row = (row_area / free.width).min(free.height);
            let mut x = free.x;
            for i in start..end {
                let width = free.width * areas[i] / row_sum;
                out.push((
                    items[i].0,
                    Rect {
                        x,
                        y: free.y,
                        width,
                        height: row,
                    },
                ));
                x += width;
            }
            free.y += row;
            free.height -= row;
        }
        start = end;
    }
    out
}

/// The cell at (x, y), as a position in `cells`, or `None`.
pub fn hit(cells: &[Rect], x: f64, y: f64) -> Option<usize> {
    cells.iter().position(|r| r.contains(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(values: &[f64]) -> Vec<Item> {
        values
            .iter()
            .map(|&value| Item {
                value,
                group: String::new(),
            })
            .collect()
    }

    fn area(r: &Rect) -> f64 {
        r.width * r.height
    }

    #[test]
    fn areas_are_proportional_and_fill_the_space() {
        let map = layout(
            &items(&[6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0]),
            600.0,
            400.0,
            0.0,
            0.0,
            0,
        );
        assert_eq!(map.cells.len(), 7);
        assert_eq!(map.total, 24.0);
        let covered: f64 = map.cells.iter().map(|c| area(&c.rect)).sum();
        assert!((covered - 240_000.0).abs() < 1e-6);
        for cell in &map.cells {
            let expected = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0][cell.index] / 24.0 * 240_000.0;
            assert!((area(&cell.rect) - expected).abs() < 1e-6, "{cell:?}");
            assert!(cell.rect.x >= 0.0 && cell.rect.x + cell.rect.width <= 600.0 + 1e-9);
            assert!(cell.rect.y >= 0.0 && cell.rect.y + cell.rect.height <= 400.0 + 1e-9);
        }
    }

    #[test]
    fn cells_do_not_overlap_and_stay_square() {
        let map = layout(
            &items(&[50.0, 30.0, 20.0, 10.0, 8.0, 5.0, 3.0, 2.0]),
            500.0,
            300.0,
            0.0,
            0.0,
            0,
        );
        for (i, a) in map.cells.iter().enumerate() {
            for b in &map.cells[i + 1..] {
                let overlap_x =
                    (a.rect.x + a.rect.width).min(b.rect.x + b.rect.width) - a.rect.x.max(b.rect.x);
                let overlap_y = (a.rect.y + a.rect.height).min(b.rect.y + b.rect.height)
                    - a.rect.y.max(b.rect.y);
                assert!(
                    overlap_x <= 1e-9 || overlap_y <= 1e-9,
                    "{a:?} overlaps {b:?}"
                );
            }
            let ratio = a.rect.width.max(a.rect.height) / a.rect.width.min(a.rect.height);
            assert!(ratio < 4.0, "{a:?} is a sliver");
        }
    }

    #[test]
    fn missing_and_negative_values_take_no_space() {
        let map = layout(
            &items(&[3.0, f64::NAN, -2.0, 0.0, 1.0]),
            100.0,
            100.0,
            0.0,
            0.0,
            0,
        );
        let indices: Vec<usize> = map.cells.iter().map(|c| c.index).collect();
        assert_eq!(indices, vec![0, 4]);
        assert_eq!(map.total, 4.0);
    }

    #[test]
    fn groups_are_placed_together_with_headers() {
        let mut list = items(&[40.0, 20.0, 30.0, 10.0]);
        for (item, group) in list
            .iter_mut()
            .zip(["Fruit", "Vegetables", "Fruit", "Vegetables"])
        {
            item.group = group.into();
        }
        let map = layout(&list, 400.0, 300.0, 20.0, 4.0, 0);
        assert_eq!(
            map.groups
                .iter()
                .map(|g| g.label.as_str())
                .collect::<Vec<_>>(),
            ["Fruit", "Vegetables"]
        );
        for cell in &map.cells {
            let group = &map.groups[cell.group];
            assert!(group.header);
            assert!(
                cell.rect.y >= group.rect.y + 20.0 - 1e-9,
                "below the header"
            );
            assert!(
                cell.rect.x >= group.rect.x - 1e-9
                    && cell.rect.x + cell.rect.width <= group.rect.x + group.rect.width + 1e-9
            );
        }
        let group_of = |index: usize| map.cells.iter().find(|c| c.index == index).unwrap().group;
        assert_eq!(
            (group_of(0), group_of(1), group_of(2), group_of(3)),
            (0, 1, 0, 1)
        );
        // Group by group, largest first: the keyboard order.
        let order: Vec<usize> = map.cells.iter().map(|c| c.index).collect();
        assert_eq!(order, vec![0, 2, 1, 3]);
    }

    #[test]
    fn hidden_groups_take_no_space() {
        let mut list = items(&[40.0, 20.0, 30.0]);
        for (item, group) in list.iter_mut().zip(["A", "B", "A"]) {
            item.group = group.into();
        }
        let map = layout(&list, 400.0, 300.0, 20.0, 0.0, 0b01);
        assert!(map.groups[0].hidden);
        assert_eq!(map.total, 20.0);
        assert_eq!(
            map.cells.iter().map(|c| c.index).collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn hit_finds_the_cell() {
        let map = layout(&items(&[1.0, 1.0]), 200.0, 100.0, 0.0, 0.0, 0);
        let rects: Vec<Rect> = map.cells.iter().map(|c| c.rect).collect();
        assert_eq!(hit(&rects, 50.0, 50.0), Some(0));
        assert_eq!(hit(&rects, 150.0, 50.0), Some(1));
        assert_eq!(hit(&rects, 250.0, 50.0), None);
    }
}
