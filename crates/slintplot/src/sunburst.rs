//! Geometry for sunburst charts: a hierarchy drawn as rings, with the top
//! level innermost. Each item's angle is its share of its parent.
//!
//! Angles are in radians, clockwise from 12 o'clock, as in pie charts.

use std::f32::consts::TAU;

use crate::layout::{Rgba, resolve_color};
use crate::pie::{sector_bounds, slice_path};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    pub value: f64,
    /// The parent item, which must come earlier in the list; `None` for the
    /// top level.
    pub parent: Option<usize>,
    pub hidden: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arc {
    /// Index of the item in the input.
    pub index: usize,
    /// Ring, from 0 (innermost).
    pub depth: usize,
    pub start: f32,
    pub end: f32,
    pub inner: f32,
    pub outer: f32,
    /// The arc's bounding box `(x, y, width, height)`, and its outline
    /// relative to the box: drawing each arc only over its own box keeps
    /// rendering fast with many arcs.
    pub bounds: (f32, f32, f32, f32),
    pub path: String,
}

impl Arc {
    /// The middle of the arc, relative to the center.
    pub fn middle(&self) -> (f32, f32) {
        let (angle, r) = (
            (self.start + self.end) / 2.0,
            (self.inner + self.outer) / 2.0,
        );
        (r * angle.sin(), -r * angle.cos())
    }

    /// Whether a horizontal label of `width` by `height`, centered on the
    /// middle of the arc, lies inside it.
    pub fn fits(&self, width: f32, height: f32) -> bool {
        let (x, y) = self.middle();
        let full = self.end - self.start >= TAU - 1e-4;
        [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
            .iter()
            .all(|&(sx, sy)| {
                let (cx, cy) = (x + sx * width / 2.0, y + sy * height / 2.0);
                let r = (cx * cx + cy * cy).sqrt();
                let angle = cx.atan2(-cy).rem_euclid(TAU);
                r >= self.inner
                    && r <= self.outer
                    && (full || (angle >= self.start && angle <= self.end))
            })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sunburst {
    /// The arcs, parents before their children (depth-first).
    pub arcs: Vec<Arc>,
    pub center: (f32, f32),
    /// The radius of the center circle, and of the whole chart.
    pub hole: f32,
    pub radius: f32,
    /// The total of the shown root: all items, or the zoomed-in item.
    pub total: f64,
}

/// The parent of each item; invalid parents (not earlier in the list) make
/// an item top-level.
fn parents(items: &[Item]) -> Vec<Option<usize>> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| item.parent.filter(|&p| p < i))
        .collect()
}

/// The value of every item: its own value, or the sum of its children if that
/// is larger. Hidden items and their children count as zero.
pub fn totals(items: &[Item]) -> Vec<f64> {
    let parents = parents(items);
    let mut hidden: Vec<bool> = items.iter().map(|i| i.hidden).collect();
    for i in 0..items.len() {
        if let Some(p) = parents[i] {
            hidden[i] |= hidden[p];
        }
    }
    let mut sums = vec![0.0; items.len()];
    let mut totals = vec![0.0; items.len()];
    for i in (0..items.len()).rev() {
        if hidden[i] {
            continue;
        }
        let own = items[i].value;
        let own = if own.is_finite() && own > 0.0 {
            own
        } else {
            0.0
        };
        totals[i] = own.max(sums[i]);
        if let Some(p) = parents[i] {
            sums[p] += totals[i];
        }
    }
    totals
}

/// Whether the item has visible children, so that it can be zoomed into.
pub fn has_children(items: &[Item], index: usize) -> bool {
    let totals = totals(items);
    parents(items)
        .iter()
        .enumerate()
        .any(|(i, p)| *p == Some(index) && totals[i] > 0.0)
}

/// The parent of an item, if it is valid.
pub fn parent(items: &[Item], index: usize) -> Option<usize> {
    parents(items).get(index).copied().flatten()
}

/// The top-level items, in order.
pub fn roots(items: &[Item]) -> Vec<usize> {
    let parents = parents(items);
    (0..items.len()).filter(|&i| parents[i].is_none()).collect()
}

/// The color of every item: its own, or the next palette color for top-level
/// items, or its parent's color, lightened, for the others.
pub fn colors(items: &[Item], own: &[Rgba], palette: &[Rgba]) -> Vec<Rgba> {
    let parents = parents(items);
    let mut out: Vec<Rgba> = Vec::with_capacity(items.len());
    let mut top = 0;
    for i in 0..items.len() {
        let color = match parents[i] {
            _ if own.get(i).is_some_and(|c| c[3] > 0) => own[i],
            None => {
                top += 1;
                resolve_color([0; 4], top - 1, palette)
            }
            Some(p) => lighten(out[p], 0.25),
        };
        out.push(color);
    }
    out
}

fn lighten([r, g, b, a]: Rgba, amount: f32) -> Rgba {
    let up = |c: u8| (c as f32 + (255.0 - c as f32) * amount).round() as u8;
    [up(r), up(g), up(b), a]
}

/// Lays out the items below `root` (all items when `None`, or when `root`
/// has no visible children). `cutout` is the size of the center circle as a
/// share of the radius; `progress` the entry animation, which sweeps the
/// rings in from 12 o'clock.
pub fn layout(
    items: &[Item],
    root: Option<usize>,
    width: f32,
    height: f32,
    cutout: f32,
    progress: f32,
) -> Sunburst {
    let totals = totals(items);
    let parents = parents(items);
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); items.len()];
    let mut top = Vec::new();
    for i in 0..items.len() {
        if totals[i] <= 0.0 {
            continue;
        }
        match parents[i] {
            Some(p) => children[p].push(i),
            None => top.push(i),
        }
    }
    let root = root.filter(|&r| r < items.len() && !children[r].is_empty());
    let first = root.map_or(&top, |r| &children[r]);
    let total = root.map_or_else(|| first.iter().map(|&i| totals[i]).sum(), |r| totals[r]);
    fn levels(children: &[Vec<usize>], nodes: &[usize]) -> usize {
        nodes
            .iter()
            .map(|&n| 1 + levels(children, &children[n]))
            .max()
            .unwrap_or(0)
    }
    let depth = levels(&children, first);
    let center = (width / 2.0, height / 2.0);
    let radius = (width.min(height) / 2.0 - 2.0).max(0.0);
    let hole = radius
        * if cutout.is_nan() {
            0.0
        } else {
            cutout.clamp(0.0, 0.9)
        };
    let mut out = Sunburst {
        arcs: Vec::new(),
        center,
        hole,
        radius,
        total,
    };
    if depth == 0 || total <= 0.0 {
        return out;
    }
    let ring = (radius - hole) / depth as f32;
    let sweep = TAU * progress.clamp(0.0, 1.0);
    struct Ctx<'a> {
        children: &'a [Vec<usize>],
        totals: &'a [f64],
        center: (f32, f32),
        hole: f32,
        ring: f32,
        arcs: Vec<Arc>,
    }
    fn place(
        ctx: &mut Ctx,
        nodes: &[usize],
        parent_total: f64,
        start: f32,
        sweep: f32,
        depth: usize,
    ) {
        let mut angle = start;
        for &n in nodes {
            let span = (ctx.totals[n] / parent_total) as f32 * sweep;
            let (inner, outer) = (
                ctx.hole + depth as f32 * ctx.ring,
                ctx.hole + (depth + 1) as f32 * ctx.ring,
            );
            let bounds = sector_bounds(ctx.center, inner, outer, angle, angle + span);
            ctx.arcs.push(Arc {
                index: n,
                depth,
                start: angle,
                end: angle + span,
                inner,
                outer,
                bounds,
                path: slice_path(
                    (ctx.center.0 - bounds.0, ctx.center.1 - bounds.1),
                    outer,
                    inner.max(0.01),
                    angle,
                    angle + span,
                ),
            });
            let kids = ctx.children[n].clone();
            place(ctx, &kids, ctx.totals[n], angle, span, depth + 1);
            angle += span;
        }
    }
    let mut ctx = Ctx {
        children: &children,
        totals: &totals,
        center,
        hole,
        ring,
        arcs: Vec::new(),
    };
    let first = first.clone();
    place(&mut ctx, &first, total, 0.0, sweep, 0);
    out.arcs = ctx.arcs;
    out
}

/// The position in `arcs` of the arc at `(x, y)` relative to the top left.
pub fn hit(arcs: &[Arc], center: (f32, f32), x: f32, y: f32) -> Option<usize> {
    let (dx, dy) = (x - center.0, y - center.1);
    let distance = (dx * dx + dy * dy).sqrt();
    let angle = dx.atan2(-dy).rem_euclid(TAU);
    arcs.iter().position(|a| {
        angle >= a.start && angle < a.end && distance >= a.inner && distance <= a.outer
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(value: f64, parent: Option<usize>) -> Item {
        Item {
            value,
            parent,
            hidden: false,
        }
    }

    /// Europe (Germany, France), Asia (Japan), and an orphan pointing forwards.
    fn tree() -> Vec<Item> {
        vec![
            item(0.0, None),
            item(30.0, Some(0)),
            item(10.0, Some(0)),
            item(0.0, None),
            item(40.0, Some(3)),
            item(20.0, Some(9)),
        ]
    }

    #[test]
    fn parents_sum_their_children() {
        let totals = totals(&tree());
        assert_eq!(totals, [40.0, 30.0, 10.0, 40.0, 40.0, 20.0]);
        assert_eq!(
            roots(&tree()),
            [0, 3, 5],
            "an invalid parent makes an item top-level"
        );
        let mut hidden = tree();
        hidden[0].hidden = true;
        assert_eq!(
            super::totals(&hidden)[..3],
            [0.0, 0.0, 0.0],
            "hiding hides the children"
        );
    }

    #[test]
    fn rings_split_by_share() {
        let sun = layout(&tree(), None, 220.0, 200.0, 0.2, 1.0);
        assert_eq!((sun.radius, sun.hole, sun.total), (98.0, 19.6, 100.0));
        let order: Vec<_> = sun.arcs.iter().map(|a| (a.index, a.depth)).collect();
        assert_eq!(order, [(0, 0), (1, 1), (2, 1), (3, 0), (4, 1), (5, 0)]);
        let germany = &sun.arcs[1];
        assert!((germany.end - TAU * 0.3).abs() < 1e-5);
        assert!((germany.inner - 58.8).abs() < 1e-4 && germany.outer == 98.0);
        // From 12 o'clock to 108°: the top and right of the circle, and past 3.
        assert_eq!(germany.bounds, (110.0, 2.0, 98.0, 129.0));
        assert!(
            germany.path.starts_with("M 0.00 0.00 A"),
            "the top of the outer arc, at the box's top left"
        );
        assert_eq!(
            hit(&sun.arcs, sun.center, 170.0, 40.0),
            Some(1),
            "Germany, upper right"
        );
        assert_eq!(hit(&sun.arcs, sun.center, 110.0, 100.0), None, "center");
        assert!(germany.fits(30.0, 12.0));
        assert!(!germany.fits(80.0, 12.0), "wider than the ring");
        assert!(
            sun.arcs[2].fits(30.0, 12.0) && !sun.arcs[2].fits(45.0, 12.0),
            "France is narrower"
        );
    }

    #[test]
    fn zooming_in_shows_one_branch() {
        let sun = layout(&tree(), Some(0), 200.0, 200.0, 0.2, 1.0);
        assert_eq!(sun.total, 40.0);
        let order: Vec<_> = sun.arcs.iter().map(|a| a.index).collect();
        assert_eq!(order, [1, 2]);
        assert!((sun.arcs[0].end - TAU * 0.75).abs() < 1e-5);
        let leaf = layout(&tree(), Some(1), 200.0, 200.0, 0.2, 1.0);
        assert_eq!(leaf.arcs.len(), 6, "a leaf can't be zoomed into");
        assert!(has_children(&tree(), 0) && !has_children(&tree(), 1));
    }

    #[test]
    fn children_take_a_lighter_parent_color() {
        let palette = [[100, 0, 0, 255], [0, 0, 100, 255]];
        let colors = colors(&tree(), &[], &palette);
        assert_eq!(colors[0], [100, 0, 0, 255]);
        assert_eq!(colors[1], [139, 64, 64, 255]);
        assert_eq!(colors[3], [0, 0, 100, 255]);
    }
}
