//! Converts datasets into pixel geometry for the Slint components.
//!
//! Everything here works in logical pixels relative to the plot area, with the
//! origin in the top-left corner.

use crate::path;
use crate::ticks::{Format, NiceScale, log_bounds, log_ticks};
use crate::time::time_ticks_in;

/// RGBA color, 8 bits per channel.
pub type Rgba = [u8; 4];

/// The dataset colors used when no palette is given: the light colors of the
/// `modern` theme.
pub const PALETTE: [Rgba; 8] = [
    [0x3b, 0x82, 0xf6, 255],
    [0x10, 0xb9, 0x81, 255],
    [0xf9, 0x73, 0x16, 255],
    [0x63, 0x66, 0xf1, 255],
    [0x84, 0xcc, 0x16, 255],
    [0x06, 0xb6, 0xd4, 255],
    [0x8b, 0x5c, 0xf6, 255],
    [0xf4, 0x3f, 0x5e, 255],
];

/// Returns `color`, or the `palette` entry for `index` when `color` is fully
/// transparent (the Slint default for an unset color). An empty palette means
/// [`PALETTE`].
pub fn resolve_color(color: Rgba, index: usize, palette: &[Rgba]) -> Rgba {
    let palette = if palette.is_empty() {
        &PALETTE[..]
    } else {
        palette
    };
    if color[3] == 0 {
        palette[index % palette.len()]
    } else {
        color
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Line,
    Bar,
    /// Numeric x and y axes; datasets use `points` instead of `values`.
    Scatter,
    /// A scatter chart whose points have their own radius.
    Bubble,
}

impl Kind {
    /// Whether datasets provide (x, y) `points` rather than `values`.
    pub fn uses_points(self) -> bool {
        matches!(self, Kind::Scatter | Kind::Bubble)
    }
}

/// A point of a scatter or bubble dataset.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DataPoint {
    pub x: f64,
    pub y: f64,
    /// Bubble radius in pixels; ignored by scatter charts.
    pub r: f64,
}

/// The points of a scatter, bubble or time series dataset. `Lazy` reads them
/// on demand, so that a zoomed-in time series reads only the points it shows.
#[derive(Clone)]
pub enum Points {
    Owned(Vec<DataPoint>),
    Lazy {
        len: usize,
        get: std::rc::Rc<dyn Fn(usize) -> DataPoint>,
    },
}

/// A point with no position, for out-of-range reads.
const NO_POINT: DataPoint = DataPoint {
    x: f64::NAN,
    y: f64::NAN,
    r: 0.0,
};

impl Points {
    pub fn len(&self) -> usize {
        match self {
            Points::Owned(v) => v.len(),
            Points::Lazy { len, .. } => *len,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The point at `index`, or one with NaN coordinates when out of range.
    pub fn get(&self, index: usize) -> DataPoint {
        match self {
            Points::Owned(v) => v.get(index).copied().unwrap_or(NO_POINT),
            Points::Lazy { len, get } if index < *len => get(index),
            Points::Lazy { .. } => NO_POINT,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = DataPoint> + '_ {
        (0..self.len()).map(|i| self.get(i))
    }

    /// The indices of the points with `x0 <= x <= x1`, by binary search:
    /// the points must be in x order, as in a time series.
    pub fn x_window(&self, x0: f64, x1: f64) -> std::ops::Range<usize> {
        let first = self.partition(|x| x < x0);
        let end = self.partition(|x| x <= x1);
        first..end.max(first)
    }

    /// The first index whose x doesn't satisfy `before`.
    fn partition(&self, before: impl Fn(f64) -> bool) -> usize {
        let (mut lo, mut hi) = (0, self.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if before(self.get(mid).x) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// The smallest and largest finite x: from the ends when the points are
    /// in x order, or else from every point.
    fn x_extent(&self, ordered: bool) -> Option<(f64, f64)> {
        if ordered {
            let (first, last) = (self.get(0).x, self.get(self.len().saturating_sub(1)).x);
            if first.is_finite() && last.is_finite() {
                return Some((first.min(last), first.max(last)));
            }
        }
        self.iter()
            .map(|p| p.x)
            .filter(|x| x.is_finite())
            .fold(None, |acc, x| {
                Some(acc.map_or((x, x), |(lo, hi): (f64, f64)| (lo.min(x), hi.max(x))))
            })
    }

    /// The points with x inside `[x0, x1]` (with a tiny margin for rounding),
    /// with their indices: found by binary search when `ordered`, or else by
    /// checking every point.
    pub fn in_window(
        &self,
        x0: f64,
        x1: f64,
        ordered: bool,
    ) -> Box<dyn Iterator<Item = (usize, DataPoint)> + '_> {
        let edge = (x1 - x0).abs() * 1e-6;
        let (x0, x1) = (x0 - edge, x1 + edge);
        if ordered {
            // Checked too: unsorted points, or NaN, can fall in the search.
            Box::new(
                self.x_window(x0, x1)
                    .map(|i| (i, self.get(i)))
                    .filter(move |(_, p)| p.x >= x0 && p.x <= x1),
            )
        } else {
            Box::new(
                self.iter()
                    .enumerate()
                    .filter(move |(_, p)| p.x >= x0 && p.x <= x1),
            )
        }
    }
}

impl Default for Points {
    fn default() -> Self {
        Points::Owned(Vec::new())
    }
}

impl FromIterator<DataPoint> for Points {
    fn from_iter<I: IntoIterator<Item = DataPoint>>(points: I) -> Self {
        Points::Owned(points.into_iter().collect())
    }
}

impl From<Vec<DataPoint>> for Points {
    fn from(points: Vec<DataPoint>) -> Self {
        Points::Owned(points)
    }
}

impl std::fmt::Debug for Points {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Points::Owned(v) => v.fmt(f),
            Points::Lazy { len, .. } => write!(f, "Lazy({len} points)"),
        }
    }
}

/// The values of a dataset. `Lazy` reads them on demand, so that a zoomed-in
/// view of a huge dataset reads only the values it shows.
#[derive(Clone)]
pub enum Values {
    Owned(Vec<f64>),
    Lazy {
        len: usize,
        get: std::rc::Rc<dyn Fn(usize) -> f64>,
    },
}

impl Values {
    pub fn len(&self) -> usize {
        match self {
            Values::Owned(v) => v.len(),
            Values::Lazy { len, .. } => *len,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The value at `index`, or NaN when out of range.
    pub fn get(&self, index: usize) -> f64 {
        match self {
            Values::Owned(v) => v.get(index).copied().unwrap_or(f64::NAN),
            Values::Lazy { len, get } if index < *len => get(index),
            Values::Lazy { .. } => f64::NAN,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        (0..self.len()).map(|i| self.get(i))
    }
}

impl Default for Values {
    fn default() -> Self {
        Values::Owned(Vec::new())
    }
}

impl From<Vec<f64>> for Values {
    fn from(values: Vec<f64>) -> Self {
        Values::Owned(values)
    }
}

impl std::fmt::Debug for Values {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Values::Owned(v) => v.fmt(f),
            Values::Lazy { len, .. } => write!(f, "Lazy({len} values)"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Dataset {
    /// Draw this dataset as a line or bars, regardless of the chart type
    /// (mixed charts). `None` follows the chart.
    pub render_as: Option<Kind>,
    pub values: Values,
    /// Points of scatter, bubble and time series charts.
    pub points: Points,
    pub color: Rgba,
    pub hidden: bool,
    pub fill: bool,
    pub tension: f32,
    /// Draw the line as steps: vertical at each point, then horizontal to
    /// the next. Overrides `tension`.
    pub stepped: bool,
    /// Plot against the second value axis, on the right.
    pub right_axis: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tick {
    /// Position along the axis, in pixels.
    pub pos: f32,
    pub label: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Axis {
    pub min: f64,
    pub max: f64,
    pub ticks: Vec<Tick>,
    /// The longest tick label, so the UI can measure the axis width.
    pub widest_label: String,
}

/// The type each dataset is drawn as: its own `render_as`, or the chart's.
pub fn dataset_kinds(datasets: &[Dataset], chart: Kind) -> Vec<Kind> {
    datasets
        .iter()
        .map(|d| match d.render_as {
            Some(kind @ (Kind::Line | Kind::Bar)) if !chart.uses_points() => kind,
            _ => chart,
        })
        .collect()
}

/// Number of categories: the longest dataset, or the number of labels.
pub fn category_count(datasets: &[Dataset], label_count: usize) -> usize {
    datasets
        .iter()
        .map(|d| d.values.len())
        .chain(std::iter::once(label_count))
        .max()
        .unwrap_or(0)
}

/// The visible part of a category axis.
#[derive(Clone, Debug, PartialEq)]
pub struct CategoryWindow {
    /// Axis position at the left and right edges, in categories.
    pub start: f32,
    pub end: f32,
    /// Whether categories sit in the middle of their band (any bars), or on
    /// band edges (lines only).
    pub offset: bool,
    /// The categories to lay out: the visible ones, plus one more on each
    /// side so lines continue to the edges.
    pub indices: std::ops::Range<usize>,
}

/// Clamps a zoom window to `0 <= start < end <= 1`; an empty window, such as
/// the all-zero default from Slint, means everything.
pub fn clamp_view((start, end): (f32, f32)) -> (f32, f32) {
    // NaN or infinite ends, which an app's own zoom code can produce, show
    // everything; `clamp` would panic on a NaN bound.
    let start = if start.is_finite() {
        start.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let end = if end.is_finite() {
        end.clamp(start, 1.0)
    } else {
        1.0
    };
    if end - start < 1e-6 {
        (0.0, 1.0)
    } else {
        (start, end)
    }
}

/// The categories shown for the zoom window `view` (fractions of the axis).
pub fn category_window(count: usize, kinds: &[Kind], view: (f32, f32)) -> CategoryWindow {
    let offset = kinds.contains(&Kind::Bar) || count <= 1;
    let length = if offset {
        count as f32
    } else {
        count.saturating_sub(1) as f32
    };
    let (view_start, view_end) = clamp_view(view);
    let (start, end) = (view_start * length, view_end * length);
    let centre = if offset { 0.5 } else { 0.0 };
    let first = ((start - centre).floor() - 1.0).max(0.0) as usize;
    let last = (((end - centre).ceil() + 2.0).max(0.0) as usize).min(count);
    CategoryWindow {
        start,
        end,
        offset,
        indices: first.min(last)..last,
    }
}

/// Cumulative sums of the visible datasets, for stacked charts. Positive and
/// negative values stack separately, and so do datasets drawn as lines and as
/// bars, and datasets on the left and right value axes.
pub struct Stacks {
    start: usize,
    /// Per dataset, for each index in the range: the (base, top) of its
    /// segment. Empty for hidden datasets.
    segments: Vec<Vec<(f64, f64)>>,
}

impl Stacks {
    pub fn new(datasets: &[Dataset], kinds: &[Kind], indices: std::ops::Range<usize>) -> Self {
        let len = indices.len();
        // [right axis][drawn as bars][negative] running sums.
        let mut sums: [[[Vec<f64>; 2]; 2]; 2] = Default::default();
        for sum in sums.iter_mut().flatten().flatten() {
            *sum = vec![0.0; len];
        }
        let segments = datasets
            .iter()
            .zip(kinds)
            .map(|(d, &kind)| {
                if d.hidden {
                    return Vec::new();
                }
                let group = &mut sums[usize::from(d.right_axis)][usize::from(kind == Kind::Bar)];
                indices
                    .clone()
                    .map(|i| {
                        let v = d.values.get(i);
                        let sum = &mut group[usize::from(v < 0.0)][i - indices.start];
                        let base = *sum;
                        if v.is_finite() {
                            *sum += v;
                        }
                        (base, base + v)
                    })
                    .collect()
            })
            .collect();
        Self {
            start: indices.start,
            segments,
        }
    }

    /// The (base, top) of a value's segment.
    pub fn segment(&self, dataset: usize, index: usize) -> (f64, f64) {
        index
            .checked_sub(self.start)
            .and_then(|i| self.segments.get(dataset)?.get(i).copied())
            .unwrap_or((0.0, f64::NAN))
    }

    /// The segments of the datasets on one value axis, or on any (`None`).
    fn on_axis<'a>(
        &'a self,
        datasets: &'a [Dataset],
        side: Option<bool>,
    ) -> impl Iterator<Item = &'a (f64, f64)> {
        self.segments
            .iter()
            .zip(datasets)
            .filter(move |(_, d)| side.is_none_or(|right| d.right_axis == right))
            .flat_map(|(segments, _)| segments)
    }
}

/// The value at which `datasets[dataset].values[index]` is drawn: the value
/// itself, or the top of its segment when stacked.
pub fn plotted_value(
    datasets: &[Dataset],
    kinds: &[Kind],
    stacked: bool,
    dataset: usize,
    index: usize,
) -> f64 {
    let value = |d: &Dataset| d.values.get(index);
    let Some(own) = datasets.get(dataset).map(value) else {
        return f64::NAN;
    };
    if !stacked {
        return own;
    }
    let axis = datasets[dataset].right_axis;
    let below: f64 = (0..dataset)
        .filter(|&i| {
            !datasets[i].hidden
                && kinds.get(i) == kinds.get(dataset)
                && datasets[i].right_axis == axis
        })
        .map(|i| value(&datasets[i]))
        .filter(|v| v.is_finite() && (*v < 0.0) == (own < 0.0))
        .sum();
    below + own
}

pub struct AxisRequest<'a> {
    pub datasets: &'a [Dataset],
    pub kind: Kind,
    pub stacked: bool,
    pub label_count: usize,
    /// Axis length in pixels.
    pub length: f32,
    pub begin_at_zero: bool,
    /// Minimum distance between ticks, in pixels.
    pub tick_spacing: f32,
    /// Zoom window along the x axis, as fractions; the value axis fits the
    /// data inside it.
    pub view: (f32, f32),
    /// Fixed ends of the axis, instead of fitting the data.
    pub fixed: Option<(f64, f64)>,
    /// A logarithmic axis; values that aren't positive are left out.
    pub logarithmic: bool,
    pub format: &'a Format,
    /// 100% stacked: each category is split into the datasets' shares.
    pub percent: bool,
    /// Extra room beyond the data at both ends, as a share of its range.
    pub grace: f64,
    /// The datasets the axis is for: those on the right axis
    /// (`Some(true)`), on the left one (`Some(false)`), or all of them in
    /// charts with one value axis (`None`).
    pub side: Option<bool>,
    /// Give a linear axis exactly this many ticks, to line them up with
    /// another axis' ticks; 0 chooses freely.
    pub align_ticks: usize,
    /// Scatter charts: every dataset's points are in x order (time series),
    /// so the visible ones are found by binary search.
    pub ordered: bool,
}

/// Computes the value axis for the visible datasets. Tick positions are
/// measured from the far end of the axis (the top of a vertical axis).
pub fn value_axis(req: &AxisRequest) -> Axis {
    if req.percent {
        let shares = percentages(req.datasets);
        let format = Format {
            suffix: req.format.locale.percent.clone(),
            ..req.format.clone()
        };
        return value_axis(&AxisRequest {
            datasets: &shares,
            stacked: true,
            percent: false,
            fixed: Some((0.0, 100.0)),
            logarithmic: false,
            format: &format,
            ..*req
        });
    }
    let extent = |(lo, hi): (f64, f64), v: f64| {
        if v.is_finite() {
            (lo.min(v), hi.max(v))
        } else {
            (lo, hi)
        }
    };
    let empty = (f64::INFINITY, f64::NEG_INFINITY);
    let visible = || {
        req.datasets
            .iter()
            .filter(|d| !d.hidden && req.side.is_none_or(|right| d.right_axis == right))
    };
    let (mut min, mut max) = if visible().next().is_none() {
        // Nothing on this axis: skip the scan of every point below.
        empty
    } else if req.kind.uses_points() {
        let (x0, x1) = scatter_x_window(req.datasets, req.view, req.ordered);
        visible()
            .flat_map(|d| d.points.in_window(x0, x1, req.ordered))
            .fold(empty, |acc, (_, p)| extent(acc, p.y))
    } else {
        let kinds = dataset_kinds(req.datasets, req.kind);
        let count = category_count(req.datasets, req.label_count);
        let indices = category_window(count, &kinds, req.view).indices;
        if req.stacked {
            Stacks::new(req.datasets, &kinds, indices)
                .on_axis(req.datasets, req.side)
                .fold(empty, |acc, &(base, top)| extent(extent(acc, base), top))
        } else {
            visible()
                .flat_map(|d| indices.clone().map(|i| d.values.get(i)))
                .fold(empty, extent)
        }
    };
    if req.logarithmic {
        // Only positive values have a place on a logarithmic axis.
        (min, max) = visible_positive_extent(req).unwrap_or((1.0, 10.0));
    } else if min > max {
        (min, max) = (0.0, 1.0);
    } else if req.grace > 0.0 {
        let room = (max - min) * req.grace;
        (min, max) = (min - room, max + room);
    }
    // Bubbles at the ends of the axis stay inside the plot.
    if req.kind == Kind::Bubble && !req.logarithmic {
        (min, max) = pad_for_radius(min, max, bubble_radius(visible()), req.length);
    }
    if req.begin_at_zero && !req.logarithmic {
        min = min.min(0.0);
        max = max.max(0.0);
    }
    // At least three ticks, even in little room: a range across zero needs
    // them, or the step jumps far past the data (-500 to 500 for ±90).
    let max_ticks = (req.length / req.tick_spacing.max(1.0))
        .floor()
        .clamp(3.0, 11.0) as usize;
    let (min, max, ticks) = if req.logarithmic {
        let (lo, hi) = match req.fixed {
            Some((lo, hi)) if lo > 0.0 && hi > lo => (lo, hi),
            _ => log_bounds(min, max),
        };
        let scale = ValueScale {
            min: lo,
            max: hi,
            logarithmic: true,
        };
        let ticks: Vec<Tick> = log_ticks(lo, hi, max_ticks)
            .into_iter()
            .map(|v| Tick {
                pos: scale.pos(v, req.length),
                label: req.format.value(v),
            })
            .collect();
        (lo, hi, ticks)
    } else {
        let (lo, hi) = match req.fixed {
            Some((lo, hi)) if hi > lo => (lo, hi),
            _ => (min, max),
        };
        let nice = if req.align_ticks >= 2 && req.fixed.is_none() {
            NiceScale::aligned(lo, hi, req.align_ticks)
        } else {
            NiceScale::new(lo, hi, max_ticks)
        };
        // A fixed range ends exactly at its bounds, with the ticks inside.
        let (lo, hi) = if req.fixed.is_some() {
            (lo, hi)
        } else {
            (nice.min, nice.max)
        };
        let scale = ValueScale {
            min: lo,
            max: hi,
            logarithmic: false,
        };
        let span = hi - lo;
        let ticks: Vec<Tick> = nice
            .ticks()
            .into_iter()
            .filter(|&v| v >= lo - span * 1e-9 && v <= hi + span * 1e-9)
            .map(|v| Tick {
                pos: scale.pos(v, req.length),
                label: req.format.tick(v, nice.step),
            })
            .collect();
        (lo, hi, ticks)
    };
    let widest_label = ticks
        .iter()
        .map(|t| &t.label)
        .max_by_key(|l| l.chars().count())
        .cloned();
    Axis {
        min,
        max,
        ticks,
        widest_label: widest_label.unwrap_or_default(),
    }
}

/// The largest bubble radius among the visible datasets, in pixels.
fn bubble_radius<'a>(datasets: impl Iterator<Item = &'a Dataset>) -> f64 {
    datasets
        .flat_map(|d| d.points.iter())
        .map(|p| p.r)
        .filter(|r| r.is_finite())
        .fold(0.0, f64::max)
}

/// Widens `min..max` so a bubble of radius `r` pixels at either end stays
/// inside an axis `length` pixels long.
fn pad_for_radius(min: f64, max: f64, r: f64, length: f32) -> (f64, f64) {
    let room = length as f64 - 2.0 * r;
    if r <= 0.0 || max <= min || room <= 0.0 {
        return (min, max);
    }
    let per_pixel = (max - min) / room;
    (min - r * per_pixel, max + r * per_pixel)
}

/// The x range shown by a scatter or bubble chart for the zoom window `view`.
/// With `ordered` points (a time series), only the ends are read.
pub fn scatter_x_window(datasets: &[Dataset], view: (f32, f32), ordered: bool) -> (f64, f64) {
    let (min, max) = datasets
        .iter()
        .filter(|d| !d.hidden)
        .filter_map(|d| d.points.x_extent(ordered))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (a, b)| {
            (lo.min(a), hi.max(b))
        });
    let (min, max) = if min > max { (0.0, 1.0) } else { (min, max) };
    let (start, end) = clamp_view(view);
    if (start, end) == (0.0, 1.0) {
        return (min, max);
    }
    let span = max - min;
    (min + start as f64 * span, min + end as f64 * span)
}

/// Each value as a share of its category: a percentage of the sum of the
/// absolute values of the visible datasets there. For 100% stacked charts.
pub fn percentages(datasets: &[Dataset]) -> Vec<Dataset> {
    let count = datasets.iter().map(|d| d.values.len()).max().unwrap_or(0);
    let totals: Vec<f64> = (0..count)
        .map(|i| {
            datasets
                .iter()
                .filter(|d| !d.hidden)
                .map(|d| d.values.get(i))
                .filter(|v| v.is_finite())
                .map(f64::abs)
                .sum()
        })
        .collect();
    datasets
        .iter()
        .map(|d| Dataset {
            values: (0..d.values.len())
                .map(|i| {
                    let v = d.values.get(i);
                    if totals[i] > 0.0 {
                        v.abs() / totals[i] * 100.0
                    } else {
                        0.0
                    }
                })
                .collect::<Vec<_>>()
                .into(),
            points: Points::default(),
            // Shares all go on the one percentage axis.
            right_axis: false,
            ..d.clone()
        })
        .collect()
}

/// Whole-number percentages that add up to exactly 100, using largest
/// remainder rounding: every share is rounded down, and the points left over
/// go to the shares with the largest remainders. Rounding each share on its
/// own can add up to 99 or 101. Non-finite shares count as 0.
pub fn round_percentages(shares: &[f64]) -> Vec<i64> {
    let shares: Vec<f64> = shares
        .iter()
        .map(|&v| if v.is_finite() { v.max(0.0) } else { 0.0 })
        .collect();
    let total: f64 = shares.iter().sum();
    if total <= 0.0 {
        return vec![0; shares.len()];
    }
    let exact: Vec<f64> = shares.iter().map(|v| v / total * 100.0).collect();
    let mut rounded: Vec<i64> = exact.iter().map(|v| v.floor() as i64).collect();
    let missing = 100 - rounded.iter().sum::<i64>();
    let mut by_remainder: Vec<usize> = (0..exact.len()).collect();
    by_remainder
        .sort_by(|&a, &b| (exact[b] - exact[b].floor()).total_cmp(&(exact[a] - exact[a].floor())));
    for &i in by_remainder.iter().take(missing.max(0) as usize) {
        rounded[i] += 1;
    }
    rounded
}

/// The smallest and largest positive values the value axis shows.
fn visible_positive_extent(req: &AxisRequest) -> Option<(f64, f64)> {
    let mut extent: Option<(f64, f64)> = None;
    let mut add = |v: f64| {
        if v.is_finite() && v > 0.0 {
            extent = Some(extent.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
        }
    };
    let visible = req
        .datasets
        .iter()
        .filter(|d| !d.hidden && req.side.is_none_or(|right| d.right_axis == right));
    // Nothing on this axis: skip the scan of every point below.
    visible.clone().next()?;
    if req.kind.uses_points() {
        let (x0, x1) = scatter_x_window(req.datasets, req.view, req.ordered);
        for (_, p) in visible.flat_map(|d| d.points.in_window(x0, x1, req.ordered)) {
            add(p.y);
        }
    } else {
        let kinds = dataset_kinds(req.datasets, req.kind);
        let count = category_count(req.datasets, req.label_count);
        let indices = category_window(count, &kinds, req.view).indices;
        if req.stacked {
            let stacks = Stacks::new(req.datasets, &kinds, indices);
            for &(_, top) in stacks.on_axis(req.datasets, req.side) {
                add(top);
            }
        } else {
            for d in visible {
                indices.clone().for_each(|i| add(d.values.get(i)));
            }
        }
    }
    extent
}

/// Maps values to positions along a value axis, linearly or logarithmically.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueScale {
    pub min: f64,
    pub max: f64,
    pub logarithmic: bool,
}

impl ValueScale {
    /// Where `v` lies between `min` (0) and `max` (1). On a logarithmic
    /// scale, values that aren't positive lie below `min`. Values far
    /// outside the range are kept within ten ranges of it: still off the
    /// plot, but in pixels the renderer can handle.
    pub fn fraction(&self, v: f64) -> f64 {
        let fraction = if self.logarithmic && self.min > 0.0 && self.max > self.min {
            let v = if v > 0.0 { v } else { self.min * 1e-3 };
            (v.ln() - self.min.ln()) / (self.max.ln() - self.min.ln())
        } else if !self.logarithmic && self.max > self.min {
            (v - self.min) / (self.max - self.min)
        } else {
            // An empty range, or a logarithmic one an app set below zero.
            0.5
        };
        if fraction.is_nan() {
            0.5
        } else {
            fraction.clamp(-10.0, 10.0)
        }
    }

    /// The position of `v` on an axis of `length` pixels, measured from the
    /// far end (the top of a vertical axis).
    pub fn pos(&self, v: f64, length: f32) -> f32 {
        length - self.fraction(v) as f32 * length
    }
}

/// Category labels, read on demand so that huge label lists aren't copied for
/// every layout.
pub trait Labels {
    fn count(&self) -> usize;
    fn get(&self, index: usize) -> String;
}

impl Labels for Vec<String> {
    fn count(&self) -> usize {
        self.len()
    }
    fn get(&self, index: usize) -> String {
        self[index].clone()
    }
}

/// Labels examined to estimate the widest one; more are sampled evenly.
const MAX_MEASURED_LABELS: usize = 1000;

/// The label that is likely widest, sampling large label lists. The caller
/// measures it exactly, so among labels of the same length it picks the one
/// with the widest letters: "Wed", not "Fri".
pub fn widest_label(labels: &dyn Labels) -> String {
    let count = labels.count();
    let every = count.div_ceil(MAX_MEASURED_LABELS).max(1);
    (0..count)
        .step_by(every)
        .chain(count.checked_sub(1))
        .map(|i| labels.get(i))
        .max_by(|a, b| relative_width(a).total_cmp(&relative_width(b)))
        .unwrap_or_default()
}

/// A rough width of `text` in units of an average letter, for comparing
/// labels: narrow letters such as "i" and "l" count less, and wide ones such
/// as "M" and "W" more.
fn relative_width(text: &str) -> f32 {
    text.chars()
        .map(|c| match c {
            'i' | 'j' | 'l' | 'I' | '.' | ',' | ':' | ';' | '\'' | '|' | '!' => 0.45,
            'f' | 't' | 'r' | ' ' | '(' | ')' | '-' => 0.6,
            'm' | 'w' | 'M' | 'W' => 1.5,
            c if c.is_uppercase() => 1.2,
            _ => 1.0,
        })
        .sum()
}

pub struct LayoutRequest<'a> {
    pub kind: Kind,
    pub datasets: &'a [Dataset],
    /// Stack the datasets on top of each other.
    pub stacked: bool,
    pub labels: &'a dyn Labels,
    pub width: f32,
    pub height: f32,
    pub y_min: f64,
    pub y_max: f64,
    /// The range of the right value axis, for datasets with `right_axis`.
    /// Without a range (`y2_max <= y2_min`), they use the left axis.
    pub y2_min: f64,
    pub y2_max: f64,
    /// Entry animation progress, from 0 (all values at the baseline) to 1.
    pub progress: f32,
    pub font_size: f32,
    /// Whether to compute point markers for line charts.
    pub show_points: bool,
    /// Reduce lines with many more points than pixels to the minimum and
    /// maximum of each pixel column (min-max decimation).
    pub decimate: bool,
    /// The category axis is vertical (horizontal bars): category labels are
    /// stacked, so they need one line of text each instead of their width.
    pub horizontal: bool,
    /// Zoom window along the x axis, as fractions of the full range.
    pub view: (f32, f32),
    /// Colors for datasets without their own.
    pub palette: &'a [Rgba],
    /// A logarithmic value axis.
    pub logarithmic: bool,
    /// A logarithmic right value axis.
    pub y2_logarithmic: bool,
    /// Scatter charts: x values are seconds after this time axis' origin.
    pub time: Option<TimeAxis>,
    /// Scatter charts: connect each dataset's points with a line.
    pub show_lines: bool,
    /// How the numbers of a numeric x axis are written.
    pub x_format: &'a Format,
    /// 100% stacked: each category is split into the datasets' shares.
    pub percent: bool,
}

/// The x axis of a time series: x values are seconds after `origin`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeAxis {
    /// Seconds since the Unix epoch.
    pub origin: f64,
    /// Offset of the displayed times from UTC.
    pub utc_offset_minutes: i32,
}

/// A line chart data point, in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    /// Index of the data value (and category) this point shows.
    pub index: usize,
    /// Bubble radius in pixels; 0 for regular point markers.
    pub r: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Series {
    /// How this dataset is drawn.
    pub kind: Kind,
    pub color: Rgba,
    pub line: String,
    pub area: String,
    /// Point markers; empty when disabled or when the line was decimated.
    pub points: Vec<Point>,
    /// Replaces `line` for decimated series: one vertical span per pixel
    /// column, which renders far faster than a path with the same shape.
    pub columns: Vec<Column>,
}

/// The vertical extent of a line within one pixel column.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Column {
    /// Center of the pixel column.
    pub x: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bar {
    pub dataset: usize,
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Rgba,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    /// One entry per dataset, in dataset order. Hidden datasets have empty geometry.
    pub series: Vec<Series>,
    pub bars: Vec<Bar>,
    pub x_ticks: Vec<Tick>,
    /// Number of categories (data points per dataset).
    pub count: usize,
    /// Distance between two categories, in pixels.
    pub step: f32,
    /// Whether categories sit in the middle of their band (bars) or on the
    /// band edges (lines).
    pub offset: bool,
    /// Axis position at the left edge, in categories (non-zero when zoomed).
    pub start: f32,
    /// Indices of the visible datasets.
    pub visible: Vec<usize>,
}

impl Layout {
    /// The x position of category `index`.
    pub fn category_x(&self, index: usize) -> f32 {
        let centre = if self.offset { 0.5 } else { 0.0 };
        (index as f32 + centre - self.start) * self.step
    }
}

/// One value axis of a layout: maps values to y pixels, moving them up from
/// the baseline during the entry animation.
struct YMap {
    scale: ValueScale,
    /// Bars and areas grow from 0, or from the bottom of a logarithmic axis.
    baseline_value: f64,
    height: f32,
    progress: f64,
}

impl YMap {
    fn new(min: f64, max: f64, logarithmic: bool, req: &LayoutRequest) -> Self {
        YMap {
            scale: ValueScale {
                min,
                max,
                logarithmic,
            },
            baseline_value: if logarithmic {
                min
            } else if min.is_nan() || max.is_nan() {
                // `clamp` panics on a NaN bound.
                0.0
            } else {
                0f64.clamp(min, max.max(min))
            },
            height: req.height,
            progress: req.progress.clamp(0.0, 1.0) as f64,
        }
    }

    /// The left and right axes of a request.
    fn both(req: &LayoutRequest) -> [YMap; 2] {
        let left = YMap::new(req.y_min, req.y_max, req.logarithmic, req);
        let right = if req.y2_max > req.y2_min {
            YMap::new(req.y2_min, req.y2_max, req.y2_logarithmic, req)
        } else {
            YMap::new(req.y_min, req.y_max, req.logarithmic, req)
        };
        [left, right]
    }

    fn y(&self, v: f64) -> f32 {
        let from = self.scale.fraction(self.baseline_value);
        self.height - (from + (self.scale.fraction(v) - from) * self.progress) as f32 * self.height
    }

    /// The baseline, in pixels from the top.
    fn baseline(&self) -> f32 {
        self.scale.pos(self.baseline_value, self.height)
    }
}

pub fn layout(req: &LayoutRequest) -> Layout {
    if req.percent {
        let shares = percentages(req.datasets);
        return layout(&LayoutRequest {
            datasets: &shares,
            stacked: true,
            percent: false,
            ..*req
        });
    }
    if req.kind.uses_points() {
        return scatter_layout(req);
    }
    let kinds = dataset_kinds(req.datasets, req.kind);
    let count = category_count(req.datasets, req.labels.count());
    let window = category_window(count, &kinds, req.view);
    let span = window.end - window.start;
    // A category is at most the plot's width: an app can zoom in to a
    // sliver of one, which would put the bars and highlight millions of
    // pixels off the plot.
    let step = if span > 0.0 {
        (req.width / span).min(req.width)
    } else {
        req.width
    };
    let visible: Vec<usize> = (0..req.datasets.len())
        .filter(|&i| !req.datasets[i].hidden)
        .collect();
    let mut out = Layout {
        count,
        step,
        offset: window.offset,
        start: window.start,
        visible,
        ..Default::default()
    };
    let indices = window.indices.clone();

    let axes = YMap::both(req);
    let axis_of = |ds: usize| &axes[usize::from(req.datasets[ds].right_axis)];
    let stacks = req
        .stacked
        .then(|| Stacks::new(req.datasets, &kinds, indices.clone()));
    // (base, top) of every value, as drawn.
    let segment = |ds: usize, index: usize, v: f64| match &stacks {
        Some(s) => s.segment(ds, index),
        None => (axis_of(ds).baseline_value, v),
    };
    // Curves stay inside the plot vertically.
    let bounds = (0.0, req.height);
    // The part of a dataset's values that is laid out.
    let window_of = |d: &Dataset| {
        let end = indices.end.min(d.values.len());
        indices.start.min(end)..end
    };

    out.series = req
        .datasets
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let kind = kinds[i];
            let color = resolve_color(d.color, i, req.palette);
            let axis = axis_of(i);
            let baseline = axis.baseline();
            if d.hidden || kind != Kind::Line {
                return Series {
                    kind,
                    color,
                    ..Default::default()
                };
            }
            let points: Vec<Point> = window_of(d)
                .map(|index| (index, d.values.get(index)))
                .filter(|(_, v)| v.is_finite())
                .map(|(index, v)| Point {
                    x: out.category_x(index),
                    y: axis.y(segment(i, index, v).1),
                    index,
                    r: 0.0,
                })
                .collect();
            let decimated = req.decimate && points.len() as f32 > req.width * 2.0;
            if decimated {
                // Curves through min/max zigzags look wrong; use straight segments.
                let area = if d.fill {
                    path::area(&decimate_min_max(&points), 0.0, baseline, bounds)
                } else {
                    String::new()
                };
                return Series {
                    kind,
                    color,
                    area,
                    columns: columns(&points),
                    ..Default::default()
                };
            }
            let shape = |coords: Vec<(f32, f32)>| {
                if d.stepped {
                    path::steps(&coords)
                } else {
                    coords
                }
            };
            let tension = if d.stepped { 0.0 } else { d.tension };
            let coords = shape(points.iter().map(|p| (p.x, p.y)).collect());
            let area = match (d.fill, &stacks) {
                (false, _) => String::new(),
                // Stacked areas fill down to the dataset below, not the axis.
                (true, Some(s)) => {
                    let bases = shape(
                        points
                            .iter()
                            .map(|p| (p.x, axis.y(s.segment(i, p.index).0)))
                            .collect(),
                    );
                    path::band(&coords, &bases, tension, bounds)
                }
                (true, None) => path::area(&coords, tension, baseline, bounds),
            };
            Series {
                kind,
                color,
                line: path::line(&coords, tension, bounds),
                area,
                points: if req.show_points { points } else { Vec::new() },
                columns: Vec::new(),
            }
        })
        .collect();

    let bar_sets: Vec<usize> = out
        .visible
        .iter()
        .copied()
        .filter(|&i| kinds[i] == Kind::Bar)
        .collect();
    if !bar_sets.is_empty() {
        // Bars take 0.8 of their category, and each group of bars 0.9 of
        // that, leaving gaps between categories and between bars. Stacked
        // bars share one slot per category.
        let group = step * 0.8;
        let reach = req.width.max(req.height).max(0.0);
        let clip = |y: f32| y.clamp(-reach, 2.0 * reach);
        let slots = if req.stacked { 1 } else { bar_sets.len() };
        let slot = group / slots as f32;
        for (k, &ds) in bar_sets.iter().enumerate() {
            let color = out.series[ds].color;
            let k = if req.stacked { 0 } else { k };
            let values = &req.datasets[ds].values;
            for index in window_of(&req.datasets[ds]) {
                let v = values.get(index);
                if !v.is_finite() {
                    continue;
                }
                let (base, top) = segment(ds, index, v);
                // Clipped to a plot's size beyond its edges: off the plot
                // either way, but within the 16-bit pixels Slint's software
                // renderer draws rectangles with.
                let (y0, y1) = (clip(axis_of(ds).y(base)), clip(axis_of(ds).y(top)));
                let band_left = out.category_x(index) - step / 2.0;
                out.bars.push(Bar {
                    dataset: ds,
                    index,
                    x: band_left + step * 0.1 + k as f32 * slot + slot * 0.05,
                    y: y0.min(y1),
                    width: slot * 0.9,
                    height: (y1 - y0).abs(),
                    color,
                });
            }
        }
    }

    // Skip category labels that would overlap.
    let label_width = if req.horizontal {
        req.font_size * 1.4
    } else {
        widest_label(req.labels).chars().count() as f32 * req.font_size * 0.6 + 8.0
    };
    let every = if step > 0.0 {
        (label_width / step).ceil().max(1.0) as usize
    } else {
        1
    };
    // Start at a multiple of `every`, so labels don't change while panning.
    let first = indices.start.div_ceil(every) * every;
    let last = indices.end.min(req.labels.count());
    out.x_ticks = (first.min(last)..last)
        .step_by(every)
        .map(|i| (i, out.category_x(i)))
        .filter(|&(_, pos)| pos >= -0.5 && pos <= req.width + 0.5)
        .map(|(i, pos)| Tick {
            pos,
            label: req.labels.get(i),
        })
        .collect();
    out
}

/// Scatter and bubble charts: a numeric x axis and one marker per point.
fn scatter_layout(req: &LayoutRequest) -> Layout {
    let visible: Vec<usize> = (0..req.datasets.len())
        .filter(|&i| !req.datasets[i].hidden)
        .collect();
    // Time series points are in time order: the visible ones are found by
    // binary search, so a zoomed-in view reads only what it shows.
    let ordered = req.time.is_some();
    let (min, max) = scatter_x_window(req.datasets, req.view, ordered);
    // Bubbles at the ends of the axis stay inside the plot, unless zoomed.
    let (min, max) = if req.kind == Kind::Bubble && clamp_view(req.view) == (0.0, 1.0) {
        let visible = visible.iter().map(|&i| &req.datasets[i]);
        pad_for_radius(min, max, bubble_radius(visible), req.width)
    } else {
        (min, max)
    };
    // Room for labels like "-1,000.5" between ticks.
    let max_ticks = (req.width / (req.font_size.max(1.0) * 6.0))
        .floor()
        .clamp(2.0, 11.0) as usize;
    let scale = NiceScale::new(min, max, max_ticks);
    // Unzoomed, a numeric axis ends on round numbers; zoomed, and on a time
    // axis, it shows exactly the window, with the ticks inside it.
    let (x0, x1) = if clamp_view(req.view) == (0.0, 1.0) && req.time.is_none() {
        (scale.min, scale.max)
    } else if max > min {
        (min, max)
    } else {
        // One each way, or more where 1 is lost in a huge value.
        let widen = (min.abs().max(max.abs()) * 1e-9).max(1.0);
        (min - widen, max + widen)
    };
    // A window of no width, such as NaN or huge ends, puts everything in
    // the middle rather than at NaN.
    let x_of = |x: f64| {
        let f = (x - x0) / (x1 - x0);
        if f.is_finite() {
            f as f32 * req.width
        } else {
            req.width / 2.0
        }
    };
    // The window comes from f32 fractions; don't drop points on its edges.
    let edge = (x1 - x0) * 1e-6;
    let progress = req.progress.clamp(0.0, 1.0) as f64;
    let max_radius = req.width.min(req.height).max(0.0) / 2.0;
    let axes = YMap::both(req);
    let bounds = (0.0, req.height);
    let series = req
        .datasets
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let color = resolve_color(d.color, i, req.palette);
            let axis = &axes[usize::from(d.right_axis)];
            let y_of = |y: f64| axis.y(y);
            if d.hidden {
                return Series {
                    kind: req.kind,
                    color,
                    ..Default::default()
                };
            }
            let points: Vec<Point> = d
                .points
                .in_window(x0, x1, ordered)
                .filter(|(_, p)| p.y.is_finite())
                .map(|(index, p)| Point {
                    x: x_of(p.x),
                    y: y_of(p.y),
                    index,
                    r: match req.kind {
                        // No bigger than the plot: a huge radius can't be drawn.
                        Kind::Bubble if p.r.is_finite() => {
                            (p.r.max(0.0) as f32).min(max_radius) * progress as f32
                        }
                        _ => 0.0,
                    },
                })
                .collect();
            if !req.show_lines {
                return Series {
                    kind: req.kind,
                    color,
                    points,
                    ..Default::default()
                };
            }
            // Time series: a line through the points in order, decimated like
            // a line chart when there are many more points than pixels.
            if req.decimate && points.len() as f32 > req.width * 2.0 {
                return Series {
                    kind: req.kind,
                    color,
                    columns: columns(&points),
                    ..Default::default()
                };
            }
            let mut coords: Vec<_> = points.iter().map(|p| (p.x, p.y)).collect();
            // Zoomed in, the line continues to the edges of the plot, towards
            // the nearest points outside the window, so a window narrower
            // than the spacing of the points still shows the line.
            let outside = |before: bool| -> Option<DataPoint> {
                if ordered {
                    let window = d.points.x_window(x0 - edge, x1 + edge);
                    let index = if before {
                        window.start.checked_sub(1)?
                    } else {
                        window.end
                    };
                    return Some(d.points.get(index)).filter(|p| p.y.is_finite());
                }
                d.points
                    .iter()
                    .filter(|p| {
                        p.y.is_finite()
                            && if before {
                                p.x < x0 - edge
                            } else {
                                p.x > x1 + edge
                            }
                    })
                    .min_by(|a, b| {
                        if before {
                            b.x.total_cmp(&a.x)
                        } else {
                            a.x.total_cmp(&b.x)
                        }
                    })
            };
            let at = |x: f64, a: &DataPoint, b: &DataPoint| {
                let t = if b.x != a.x {
                    (x - a.x) / (b.x - a.x)
                } else {
                    0.0
                };
                (x_of(x), y_of(a.y + (b.y - a.y) * t))
            };
            let zoomed = clamp_view(req.view) != (0.0, 1.0);
            let (before, after) = if zoomed {
                (outside(true), outside(false))
            } else {
                (None, None)
            };
            let first = points.first().map(|p| d.points.get(p.index)).or(after);
            if let (Some(a), Some(b)) = (before, first) {
                coords.insert(0, at(x0, &a, &b));
            }
            let last = points.last().map(|p| d.points.get(p.index)).or(before);
            if let (Some(a), Some(b)) = (last, after) {
                coords.push(at(x1, &a, &b));
            }
            let coords = if d.stepped {
                path::steps(&coords)
            } else {
                coords
            };
            let tension = if d.stepped { 0.0 } else { d.tension };
            Series {
                kind: req.kind,
                color,
                line: path::line(&coords, tension, bounds),
                area: if d.fill {
                    path::area(&coords, tension, axis.baseline(), bounds)
                } else {
                    String::new()
                },
                // Kept for hovering even when the markers are hidden.
                points,
                columns: Vec::new(),
            }
        })
        .collect();
    let x_ticks = match req.time {
        // Half a pixel either side, so the rounding of the zoom window
        // doesn't drop a tick on the edge.
        Some(time) => time_ticks_in(
            time.origin + x0 - (x1 - x0) / req.width.max(1.0) as f64 * 0.5,
            time.origin + x1 + (x1 - x0) / req.width.max(1.0) as f64 * 0.5,
            max_ticks,
            time.utc_offset_minutes,
            &req.x_format.locale,
        )
        .into_iter()
        .map(|(t, label)| Tick {
            pos: x_of(t - time.origin),
            label,
        })
        .collect(),
        None => scale
            .ticks()
            .into_iter()
            .map(|v| Tick {
                pos: x_of(v),
                label: req.x_format.tick(v, scale.step),
            })
            .collect::<Vec<_>>(),
    };
    Layout {
        series,
        x_ticks: x_ticks
            .into_iter()
            .filter(|t| t.pos >= -0.5 && t.pos <= req.width + 0.5)
            .collect(),
        visible,
        ..Default::default()
    }
}

/// A keyboard move through the points of a scatter, bubble or time series
/// chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointKey {
    /// The next or previous point of the dataset, in x order.
    Next,
    Previous,
    First,
    Last,
    /// The next or previous dataset with visible points, at the point
    /// closest in x.
    NextSeries,
    PreviousSeries,
}

/// The visible point of `dataset` whose data index is `index`.
pub fn locate_point(series: &[Series], dataset: usize, index: usize) -> Option<Point> {
    series
        .get(dataset)?
        .points
        .iter()
        .find(|p| p.index == index)
        .copied()
}

/// Where a keyboard move from the point `current` (dataset, data index)
/// lands, as (dataset, point). Without a current point, or when it is no
/// longer visible, the move starts at the leftmost point of the first
/// dataset with visible points.
pub fn key_point(
    series: &[Series],
    current: Option<(usize, usize)>,
    key: PointKey,
) -> Option<(usize, Point)> {
    let sorted = |dataset: usize| {
        let mut points = series[dataset].points.clone();
        points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.index.cmp(&b.index)));
        points
    };
    let start = current.and_then(|(d, i)| Some((d, locate_point(series, d, i)?)));
    let Some((dataset, point)) = start else {
        let first = series.iter().position(|s| !s.points.is_empty())?;
        return sorted(first).first().map(|&p| (first, p));
    };
    match key {
        PointKey::Next | PointKey::Previous | PointKey::First | PointKey::Last => {
            let points = sorted(dataset);
            let at = points.iter().position(|p| p.index == point.index)?;
            let to = match key {
                PointKey::Next => (at + 1).min(points.len() - 1),
                PointKey::Previous => at.saturating_sub(1),
                PointKey::First => 0,
                _ => points.len() - 1,
            };
            Some((dataset, points[to]))
        }
        PointKey::NextSeries | PointKey::PreviousSeries => {
            let n = series.len();
            let next = (1..=n)
                .map(|step| {
                    if key == PointKey::NextSeries {
                        (dataset + step) % n
                    } else {
                        (dataset + n * 2 - step) % n
                    }
                })
                .find(|&d| !series[d].points.is_empty())?;
            let closest = series[next].points.iter().min_by(|a, b| {
                (a.x - point.x)
                    .abs()
                    .total_cmp(&(b.x - point.x).abs())
                    .then(a.index.cmp(&b.index))
            })?;
            Some((next, *closest))
        }
    }
}

/// The point closest to `(x, y)` within `radius` pixels, or inside its own
/// bubble radius when that is larger, as (dataset, point). With `x_only`, the
/// point closest in x at any distance, preferring the closest in y on ties:
/// hovering anywhere above or below a time series finds the point at that time.
pub fn nearest_point(
    series: &[Series],
    x: f32,
    y: f32,
    radius: f32,
    x_only: bool,
) -> Option<(usize, Point)> {
    let mut best: Option<(f32, usize, Point)> = None;
    for (ds, s) in series.iter().enumerate() {
        for p in &s.points {
            let d = if x_only {
                (p.x - x).abs() * 1e4 + (p.y - y).abs()
            } else {
                (p.x - x).powi(2) + (p.y - y).powi(2)
            };
            let reach = radius.max(p.r);
            let close = x_only || d <= reach * reach;
            if close && best.is_none_or(|(b, _, _)| d < b) {
                best = Some((d, ds, *p));
            }
        }
    }
    best.map(|(_, ds, p)| (ds, p))
}

/// Keeps, for every pixel column, the first, minimum, maximum and last point,
/// in their original order. The drawn shape is unchanged at this resolution.
fn decimate_min_max(points: &[Point]) -> Vec<(f32, f32)> {
    let mut out = Vec::with_capacity(points.len().min(4096));
    for group in points.chunk_by(|a, b| a.x.floor() == b.x.floor()) {
        let (first, last) = (group[0], group[group.len() - 1]);
        let min = group
            .iter()
            .min_by(|a, b| a.y.total_cmp(&b.y))
            .unwrap_or(&first);
        let max = group
            .iter()
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .unwrap_or(&first);
        let mut keep = [first, *min, *max, last];
        keep.sort_by_key(|p| p.index);
        let mut previous = None;
        for p in keep {
            if previous != Some(p.index) {
                out.push((p.x, p.y));
                previous = Some(p.index);
            }
        }
    }
    out
}

/// Reduces a dense line to the span it covers in every pixel column. Each
/// span also reaches the previous column's last point, so the line stays
/// connected across columns.
fn columns(points: &[Point]) -> Vec<Column> {
    let mut out = Vec::with_capacity(points.len().min(4096));
    let mut previous: Option<f32> = None;
    for group in points.chunk_by(|a, b| a.x.floor() == b.x.floor()) {
        let (mut top, mut bottom) = group
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.y), hi.max(p.y))
            });
        if let Some(y) = previous {
            top = top.min(y);
            bottom = bottom.max(y);
        }
        previous = Some(group[group.len() - 1].y);
        out.push(Column {
            x: group[0].x.floor() + 0.5,
            top,
            bottom,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_LABELS: &Vec<String> = &Vec::new();
    static PLAIN: std::sync::LazyLock<Format> = std::sync::LazyLock::new(Format::default);

    #[test]
    fn widest_label_prefers_wide_letters() {
        let weekdays: Vec<String> = ["Mon", "Tue", "Wed", "Thu", "Fri"]
            .map(String::from)
            .to_vec();
        assert_eq!(widest_label(&weekdays), "Wed");
        let labels: Vec<String> = ["Jan", "September", "May"].map(String::from).to_vec();
        assert_eq!(widest_label(&labels), "September");
    }

    fn axis(
        datasets: &[Dataset],
        kind: Kind,
        stacked: bool,
        length: f32,
        begin_at_zero: bool,
    ) -> Axis {
        value_axis(&AxisRequest {
            datasets,
            kind,
            stacked,
            label_count: 0,
            length,
            begin_at_zero,
            tick_spacing: 36.0,
            view: (0.0, 1.0),
            fixed: None,
            logarithmic: false,
            format: &PLAIN,
            percent: false,
            grace: 0.0,
            side: None,
            align_ticks: 0,
            ordered: false,
        })
    }

    #[test]
    fn little_room_keeps_the_axis_close_to_the_data() {
        // Room for one tick only: still -100, 0 and 100, not -500 to 500.
        let a = axis(&[ds(&[-90.0, 85.0])], Kind::Bar, false, 40.0, true);
        assert_eq!((a.min, a.max, a.ticks.len()), (-100.0, 100.0, 3));
    }

    #[test]
    fn scatter_maps_both_axes_and_finds_nearest() {
        let data = [Dataset {
            points: [(0.0, 0.0), (5.0, 10.0), (10.0, 5.0)]
                .map(|(x, y)| DataPoint { x, y, r: 0.0 })
                .to_vec()
                .into(),
            ..Default::default()
        }];
        let axis = axis(&data, Kind::Scatter, false, 100.0, false);
        assert_eq!((axis.min, axis.max), (0.0, 10.0));
        let l = layout(&request(Kind::Scatter, &data, NO_LABELS));
        let points: Vec<_> = l.series[0].points.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(points, vec![(0.0, 100.0), (150.0, 0.0), (300.0, 50.0)]);
        assert_eq!(l.x_ticks.first().map(|t| t.label.as_str()), Some("0"));
        assert_eq!(l.x_ticks.last().map(|t| t.pos), Some(300.0));
        let hit = nearest_point(&l.series, 145.0, 6.0, 10.0, false);
        assert_eq!(hit.map(|(ds, p)| (ds, p.index)), Some((0, 1)));
        assert_eq!(nearest_point(&l.series, 75.0, 50.0, 10.0, false), None);
    }

    #[test]
    fn bubbles_stay_inside_the_plot() {
        let bubble = |x, y, r| DataPoint { x, y, r };
        let data = [Dataset {
            points: vec![
                bubble(20.0, 30.0, 15.0),
                bubble(48.0, 18.0, 24.0),
                bubble(85.0, 46.0, 18.0),
            ]
            .into(),
            ..Default::default()
        }];
        // The chart computes the y axis first, as the components do.
        let y = axis(&data, Kind::Bubble, false, 300.0, false);
        let req = LayoutRequest {
            height: 300.0,
            y_min: y.min,
            y_max: y.max,
            ..request(Kind::Bubble, &data, NO_LABELS)
        };
        let l = layout(&req);
        for p in &l.series[0].points {
            assert!(
                p.x - p.r >= 0.0 && p.x + p.r <= req.width,
                "x {} r {}",
                p.x,
                p.r
            );
            assert!(
                p.y - p.r >= 0.0 && p.y + p.r <= req.height,
                "y {} r {}",
                p.y,
                p.r
            );
        }
    }

    #[test]
    fn bubbles_are_hit_anywhere_inside() {
        let data = [Dataset {
            points: vec![
                DataPoint {
                    x: 0.0,
                    y: 0.0,
                    r: 30.0,
                },
                DataPoint {
                    x: 10.0,
                    y: 10.0,
                    r: 2.0,
                },
            ]
            .into(),
            ..Default::default()
        }];
        let l = layout(&request(Kind::Bubble, &data, NO_LABELS));
        let bubble = l.series[0].points[0];
        assert_eq!(bubble.r, 30.0);
        let hit = nearest_point(&l.series, bubble.x + 25.0, bubble.y, 5.0, false);
        assert_eq!(
            hit.map(|(_, p)| p.index),
            Some(0),
            "25px from a 30px bubble"
        );
        let scatter = layout(&request(Kind::Scatter, &data, NO_LABELS));
        assert_eq!(scatter.series[0].points[0].r, 0.0, "scatter ignores r");
    }

    #[test]
    fn stacks_split_positive_and_negative() {
        let data = [ds(&[2.0, -1.0]), ds(&[3.0, -2.0]), ds(&[1.0, 4.0])];
        let kinds = [Kind::Bar; 3];
        let s = Stacks::new(&data, &kinds, 0..2);
        assert_eq!(
            (s.segment(1, 0), s.segment(1, 1)),
            ((2.0, 5.0), (-1.0, -3.0))
        );
        assert_eq!((s.segment(2, 0), s.segment(2, 1)), ((5.0, 6.0), (0.0, 4.0)));
        assert_eq!(plotted_value(&data, &kinds, true, 2, 0), 6.0);
        assert_eq!(plotted_value(&data, &kinds, true, 1, 1), -3.0);
        assert_eq!(plotted_value(&data, &kinds, false, 1, 1), -2.0);
        let axis = axis(&data, Kind::Bar, true, 200.0, true);
        assert!(axis.min <= -3.0 && axis.max >= 6.0);
    }

    #[test]
    fn mixed_line_on_bars() {
        let line = Dataset {
            render_as: Some(Kind::Line),
            ..ds(&[5.0, 10.0])
        };
        let data = [ds(&[4.0, 6.0]), line];
        let l = layout(&request(Kind::Bar, &data, NO_LABELS));
        assert!(l.offset, "bars put categories in the middle of their band");
        assert_eq!(l.bars.len(), 2, "only the bar dataset makes bars");
        assert!(
            l.bars
                .iter()
                .all(|b| (b.width - 150.0 * 0.8 * 0.9).abs() < 1e-3),
            "one slot per category"
        );
        assert_eq!(l.series[1].kind, Kind::Line);
        let xs: Vec<_> = l.series[1].points.iter().map(|p| p.x).collect();
        assert_eq!(
            xs,
            vec![75.0, 225.0],
            "the line goes through the bar centers"
        );
        // Stacking keeps lines and bars apart.
        let kinds = dataset_kinds(&data, Kind::Bar);
        assert_eq!(plotted_value(&data, &kinds, true, 1, 0), 5.0);
    }

    #[test]
    fn zooming_lays_out_only_the_window() {
        let values: Vec<f64> = (0..1001).map(|i| i as f64).collect();
        let data = [ds(&values)];
        let mut req = request(Kind::Line, &data, NO_LABELS);
        req.view = (0.5, 0.6);
        let l = layout(&req);
        assert_eq!(l.start, 500.0);
        assert!((l.step - 3.0).abs() < 1e-4, "100 categories across 300px");
        let first = l.series[0].points.first().unwrap();
        let last = l.series[0].points.last().unwrap();
        assert_eq!(
            (first.index, last.index),
            (499, 601),
            "one extra point on each side"
        );
        assert!((l.category_x(500) - 0.0).abs() < 1e-3);
        // The value axis fits the visible values only.
        let a = value_axis(&AxisRequest {
            datasets: &data,
            kind: Kind::Line,
            stacked: false,
            label_count: 0,
            length: 200.0,
            begin_at_zero: false,
            tick_spacing: 36.0,
            view: (0.5, 0.6),
            fixed: None,
            logarithmic: false,
            format: &PLAIN,
            percent: false,
            grace: 0.0,
            side: None,
            align_ticks: 0,
            ordered: false,
        });
        assert!(a.min >= 400.0 && a.max <= 700.0, "{} to {}", a.min, a.max);
    }

    #[test]
    fn scatter_zoom_shows_the_window() {
        let data = [Dataset {
            points: (0..=10)
                .map(|i| DataPoint {
                    x: i as f64 * 10.0,
                    y: i as f64,
                    r: 0.0,
                })
                .collect(),
            ..Default::default()
        }];
        let mut req = request(Kind::Scatter, &data, NO_LABELS);
        req.view = (0.2, 0.5);
        let l = layout(&req);
        let xs: Vec<_> = l.series[0].points.iter().map(|p| p.index).collect();
        assert_eq!(xs, vec![2, 3, 4, 5]);
        assert!(l.series[0].points[0].x.abs() < 1e-3);
        assert!((l.series[0].points[3].x - 300.0).abs() < 1e-3);
    }

    #[test]
    fn empty_view_means_everything() {
        assert_eq!(clamp_view((0.0, 0.0)), (0.0, 1.0));
        assert_eq!(clamp_view((0.3, 2.0)), (0.3, 1.0));
    }

    #[test]
    fn fixed_and_logarithmic_axes() {
        let data = [ds(&[3.0, 40.0, 900.0])];
        let axis_with = |fixed, logarithmic| {
            value_axis(&AxisRequest {
                datasets: &data,
                kind: Kind::Line,
                stacked: false,
                label_count: 0,
                length: 300.0,
                begin_at_zero: false,
                tick_spacing: 20.0,
                view: (0.0, 1.0),
                fixed,
                logarithmic,
                format: &PLAIN,
                percent: false,
                grace: 0.0,
                side: None,
                align_ticks: 0,
                ordered: false,
            })
        };
        let fixed = axis_with(Some((0.0, 250.0)), false);
        assert_eq!((fixed.min, fixed.max), (0.0, 250.0));
        assert!(fixed.ticks.iter().all(|t| t.pos >= 0.0 && t.pos <= 300.0));
        let log = axis_with(None, true);
        assert_eq!((log.min, log.max), (1.0, 1000.0));
        let labels: Vec<_> = log.ticks.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels.first(), Some(&"1"));
        assert_eq!(labels.last(), Some(&"1,000"));
        // On a log scale, 10 sits a third of the way up from 1 to 1,000.
        let scale = ValueScale {
            min: 1.0,
            max: 1000.0,
            logarithmic: true,
        };
        assert!((scale.pos(10.0, 300.0) - 200.0).abs() < 1e-3);
    }

    #[test]
    fn money_format_on_ticks() {
        let money = Format {
            prefix: "$".into(),
            decimals: Some(0),
            ..Default::default()
        };
        let a = value_axis(&AxisRequest {
            datasets: &[ds(&[1200.0, 5300.0])],
            kind: Kind::Bar,
            stacked: false,
            label_count: 0,
            length: 300.0,
            begin_at_zero: true,
            tick_spacing: 36.0,
            view: (0.0, 1.0),
            fixed: None,
            logarithmic: false,
            format: &money,
            percent: false,
            grace: 0.0,
            side: None,
            align_ticks: 0,
            ordered: false,
        });
        assert_eq!(a.ticks.last().map(|t| t.label.as_str()), Some("$6,000"));
    }

    #[test]
    fn time_series_lines_and_ticks() {
        let origin = crate::time::unix_seconds(2026, 10, 25, 0, 0, 0);
        let points: Vec<_> = (0..=24)
            .map(|h| DataPoint {
                x: h as f64 * 3600.0,
                y: h as f64,
                r: 0.0,
            })
            .collect();
        let data = [Dataset {
            points: points.into(),
            ..Default::default()
        }];
        let mut req = request(Kind::Scatter, &data, NO_LABELS);
        req.time = Some(TimeAxis {
            origin,
            utc_offset_minutes: 0,
        });
        req.show_lines = true;
        let l = layout(&req);
        assert!(l.series[0].line.starts_with("M 0.00"));
        let labels: Vec<_> = l.x_ticks.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels.first(), Some(&"Oct 25"), "midnight shows the date");
        assert!(labels.contains(&"12:00"), "{labels:?}");
        assert!(
            (l.x_ticks.last().unwrap().pos - 300.0).abs() < 1e-3,
            "the window ends at the last point"
        );
    }

    #[test]
    fn zoomed_lines_reach_the_edges() {
        // A point every hour, y equal to the hour.
        let points: Vec<_> = (0..=24)
            .map(|h| DataPoint {
                x: h as f64 * 3600.0,
                y: h as f64,
                r: 0.0,
            })
            .collect();
        let data = [Dataset {
            points: points.into(),
            ..Default::default()
        }];
        let mut req = request(Kind::Scatter, &data, NO_LABELS);
        req.show_lines = true;
        req.y_max = 24.0;
        // 10:30 to 11:00, between two points: still a line, across the plot.
        req.view = (10.5 / 24.0, 11.0 / 24.0);
        let l = layout(&req);
        assert!(
            l.series[0].points.is_empty()
                || l.series[0]
                    .points
                    .iter()
                    .all(|p| p.x >= -0.01 && p.x <= 300.01)
        );
        let line = &l.series[0].line;
        assert!(line.starts_with("M 0.00"), "{line}");
        assert!(line.contains("300.00"), "{line}");
        // 10:00 to 11:00 by f32 fractions, which can miss the points on the
        // edges: the line still spans the plot.
        req.view = (10.0 / 24.0, 11.0 / 24.0);
        let line = layout(&req).series[0].line.clone();
        assert!(line.starts_with("M 0.00"), "{line}");
        assert!(line.contains("300.00"), "{line}");
    }

    #[test]
    fn keyboard_moves_through_points() {
        let point = |x: f32, index: usize| Point {
            x,
            y: 0.0,
            index,
            r: 0.0,
        };
        let series = vec![
            Series {
                points: vec![point(30.0, 0), point(10.0, 1), point(20.0, 2)],
                ..Default::default()
            },
            // Hidden: no visible points.
            Series::default(),
            Series {
                points: vec![point(5.0, 0), point(28.0, 1)],
                ..Default::default()
            },
        ];
        let at = |r: Option<(usize, Point)>| r.map(|(d, p)| (d, p.index));
        // Starts at the leftmost point of the first dataset.
        assert_eq!(at(key_point(&series, None, PointKey::Next)), Some((0, 1)));
        // Next and previous go in x order, and stop at the ends.
        assert_eq!(
            at(key_point(&series, Some((0, 1)), PointKey::Next)),
            Some((0, 2))
        );
        assert_eq!(
            at(key_point(&series, Some((0, 2)), PointKey::Next)),
            Some((0, 0))
        );
        assert_eq!(
            at(key_point(&series, Some((0, 0)), PointKey::Next)),
            Some((0, 0))
        );
        assert_eq!(
            at(key_point(&series, Some((0, 1)), PointKey::Previous)),
            Some((0, 1))
        );
        assert_eq!(
            at(key_point(&series, Some((0, 2)), PointKey::Last)),
            Some((0, 0))
        );
        // Another dataset skips the hidden one and keeps the x position.
        assert_eq!(
            at(key_point(&series, Some((0, 0)), PointKey::NextSeries)),
            Some((2, 1))
        );
        assert_eq!(
            at(key_point(&series, Some((2, 1)), PointKey::NextSeries)),
            Some((0, 0))
        );
        assert_eq!(
            at(key_point(&series, Some((0, 1)), PointKey::PreviousSeries)),
            Some((2, 0))
        );
        // A point scrolled out of view starts over.
        assert_eq!(
            at(key_point(&series, Some((0, 9)), PointKey::Next)),
            Some((0, 1))
        );
        assert_eq!(key_point(&[Series::default()], None, PointKey::Next), None);
    }

    #[test]
    fn rounded_percentages_add_up_to_100() {
        // Rounded on their own, these give 8 + 16 + 37 + 27 + 13 = 101.
        let shares = [7.83, 15.64, 36.75, 26.91, 12.87];
        let rounded = round_percentages(&shares);
        assert_eq!(rounded.iter().sum::<i64>(), 100);
        assert_eq!(
            rounded,
            vec![8, 15, 37, 27, 13],
            "the 4 missing points go to the largest remainders"
        );
        assert_eq!(round_percentages(&[1.0, 1.0, 1.0]).iter().sum::<i64>(), 100);
        assert_eq!(round_percentages(&[0.0, f64::NAN]), vec![0, 0]);
    }

    #[test]
    fn percent_stacks_fill_each_category() {
        let data = [ds(&[1.0, 30.0]), ds(&[3.0, 10.0])];
        let shares = percentages(&data);
        assert_eq!(shares[0].values.get(0), 25.0);
        assert_eq!(shares[1].values.get(1), 25.0);
        let mut req = request(Kind::Bar, &data, NO_LABELS);
        req.percent = true;
        req.y_max = 100.0;
        let l = layout(&req);
        for index in 0..2 {
            let total: f32 = l
                .bars
                .iter()
                .filter(|b| b.index == index)
                .map(|b| b.height)
                .sum();
            assert!((total - 100.0).abs() < 1e-3, "each category fills the axis");
        }
    }

    #[test]
    fn stacked_bars_sit_on_each_other() {
        let data = [ds(&[4.0]), ds(&[6.0])];
        let mut req = request(Kind::Bar, &data, NO_LABELS);
        req.stacked = true;
        let l = layout(&req);
        let (a, b) = (&l.bars[0], &l.bars[1]);
        assert_eq!(a.x, b.x);
        assert!(
            (a.y - (b.y + b.height)).abs() < 1e-3,
            "b starts where a ends"
        );
        assert!(
            (b.y - 0.0).abs() < 1e-3,
            "4 + 6 reaches the top of a 0..10 axis"
        );
    }

    fn ds(values: &[f64]) -> Dataset {
        Dataset {
            values: values.to_vec().into(),
            ..Default::default()
        }
    }

    fn request<'a>(
        kind: Kind,
        datasets: &'a [Dataset],
        labels: &'a Vec<String>,
    ) -> LayoutRequest<'a> {
        LayoutRequest {
            kind,
            datasets,
            stacked: false,
            labels,
            width: 300.0,
            height: 100.0,
            y_min: 0.0,
            y_max: 10.0,
            y2_min: 0.0,
            y2_max: 1.0,
            y2_logarithmic: false,
            progress: 1.0,
            font_size: 12.0,
            show_points: true,
            decimate: false,
            horizontal: false,
            view: (0.0, 1.0),
            palette: &[],
            logarithmic: false,
            time: None,
            show_lines: false,
            x_format: &PLAIN,
            percent: false,
        }
    }

    #[test]
    fn axis_ignores_hidden_datasets() {
        let mut big = ds(&[1000.0]);
        big.hidden = true;
        let axis = axis(&[ds(&[2.0, 8.0]), big], Kind::Line, false, 200.0, true);
        assert_eq!(axis.min, 0.0);
        assert!(axis.max < 1000.0);
        assert_eq!(axis.ticks.first().map(|t| t.pos), Some(200.0));
        assert_eq!(axis.ticks.last().map(|t| t.pos), Some(0.0));
    }

    #[test]
    fn line_points_span_full_width() {
        let data = [ds(&[0.0, 5.0, 10.0, 5.0])];
        let l = layout(&request(Kind::Line, &data, NO_LABELS));
        let points: Vec<_> = l.series[0].points.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(
            points,
            vec![(0.0, 100.0), (100.0, 50.0), (200.0, 0.0), (300.0, 50.0)]
        );
        assert!(!l.offset);
    }

    #[test]
    fn points_keep_their_data_index() {
        let data = [ds(&[1.0, f64::NAN, 3.0])];
        let l = layout(&request(Kind::Line, &data, NO_LABELS));
        let indices: Vec<_> = l.series[0].points.iter().map(|p| p.index).collect();
        assert_eq!(indices, vec![0, 2]);
    }

    #[test]
    fn decimation_bounds_path_size_and_keeps_extremes() {
        let values: Vec<f64> = (0..100_000)
            .map(|i| ((i * 7919) % 1000) as f64 / 100.0)
            .collect();
        let data = [ds(&values)];
        let mut req = request(Kind::Line, &data, NO_LABELS);
        req.decimate = true;
        let l = layout(&req);
        let series = &l.series[0];
        assert!(series.line.is_empty() && series.points.is_empty());
        assert!(
            series.columns.len() <= 301,
            "{} columns for 300px",
            series.columns.len()
        );
    }

    #[test]
    fn decimation_keeps_column_extremes_in_order() {
        let p = |x: f32, y: f32, index: usize| Point {
            x,
            y,
            index,
            r: 0.0,
        };
        let points = [
            p(0.1, 5.0, 0),
            p(0.4, 9.0, 1),
            p(0.6, 1.0, 2),
            p(0.9, 4.0, 3),
            p(1.2, 2.0, 4),
        ];
        assert_eq!(
            decimate_min_max(&points),
            vec![(0.1, 5.0), (0.4, 9.0), (0.6, 1.0), (0.9, 4.0), (1.2, 2.0)]
        );
        let many: Vec<_> = (0..10)
            .map(|i| p(0.05 * i as f32, (i % 3) as f32, i))
            .collect();
        let kept = decimate_min_max(&many);
        assert_eq!(kept.len(), 3, "first (also the minimum), maximum, last");
        assert_eq!(kept.first(), Some(&(0.0, 0.0)));
        assert!(kept.iter().any(|&(_, y)| y == 2.0));
    }

    #[test]
    fn columns_connect_to_the_previous_column() {
        let p = |x: f32, y: f32| Point {
            x,
            y,
            index: 0,
            r: 0.0,
        };
        let points = [
            p(0.2, 5.0),
            p(0.7, 9.0),
            p(1.1, 2.0),
            p(1.6, 3.0),
            p(2.3, 7.0),
        ];
        let spans: Vec<_> = columns(&points)
            .iter()
            .map(|c| (c.x, c.top, c.bottom))
            .collect();
        assert_eq!(
            spans,
            vec![(0.5, 5.0, 9.0), (1.5, 2.0, 9.0), (2.5, 3.0, 7.0)]
        );
    }

    #[test]
    fn grouped_bars_stay_in_their_band() {
        let data = [ds(&[10.0, 5.0]), ds(&[2.0, 4.0])];
        let l = layout(&request(Kind::Bar, &data, NO_LABELS));
        assert_eq!(l.bars.len(), 4);
        for b in &l.bars {
            let band = b.index as f32 * l.step;
            assert!(b.x >= band && b.x + b.width <= band + l.step);
            assert!(
                (b.y + b.height - 100.0).abs() < 1e-3,
                "bars grow from the baseline"
            );
        }
    }

    #[test]
    fn labels_are_skipped_when_crowded() {
        let labels: Vec<String> = (0..100).map(|i| format!("Label {i}")).collect();
        let l = layout(&request(Kind::Line, &[ds(&[1.0; 100])], &labels));
        assert!(l.x_ticks.len() < 20);
        assert_eq!(l.x_ticks[0].label, "Label 0");
    }

    #[test]
    fn progress_starts_at_baseline() {
        let data = [ds(&[10.0])];
        let mut req = request(Kind::Bar, &data, NO_LABELS);
        req.progress = 0.0;
        assert_eq!(layout(&req).bars[0].height, 0.0);
    }

    fn right(values: &[f64]) -> Dataset {
        Dataset {
            right_axis: true,
            ..ds(values)
        }
    }

    fn side_axis(
        datasets: &[Dataset],
        kind: Kind,
        stacked: bool,
        right: bool,
        align: usize,
    ) -> Axis {
        value_axis(&AxisRequest {
            datasets,
            kind,
            stacked,
            label_count: 0,
            length: 200.0,
            begin_at_zero: true,
            tick_spacing: 36.0,
            view: (0.0, 1.0),
            fixed: None,
            logarithmic: false,
            format: &PLAIN,
            percent: false,
            grace: 0.0,
            side: Some(right),
            align_ticks: align,
            ordered: false,
        })
    }

    #[test]
    fn each_axis_fits_its_own_datasets() {
        let data = [ds(&[2.0, 8.0]), right(&[300.0, 900.0])];
        let left = side_axis(&data, Kind::Line, false, false, 0);
        let right_axis = side_axis(&data, Kind::Line, false, true, 0);
        assert!(
            left.max < 20.0 && right_axis.max >= 900.0,
            "{left:?} {right_axis:?}"
        );
    }

    #[test]
    fn aligned_right_axis_has_the_left_axis_ticks() {
        let data = [ds(&[0.0, 23.0]), right(&[0.0, 410.0])];
        let left = side_axis(&data, Kind::Bar, false, false, 0);
        let aligned = side_axis(&data, Kind::Bar, false, true, left.ticks.len());
        assert_eq!(aligned.ticks.len(), left.ticks.len());
        assert!(aligned.min <= 0.0 && aligned.max >= 410.0, "{aligned:?}");
        for (a, b) in left.ticks.iter().zip(&aligned.ticks) {
            assert!((a.pos - b.pos).abs() < 1e-3, "{left:?} {aligned:?}");
        }
    }

    #[test]
    fn aligned_scales_cover_the_data_with_round_steps() {
        for (min, max, count) in [
            (0.0, 410.0, 6),
            (-3.0, 7.0, 5),
            (0.0, 0.9, 4),
            (12.0, 12.0, 3),
        ] {
            let s = NiceScale::aligned(min, max, count);
            assert_eq!(s.ticks().len(), count, "{s:?}");
            assert!(s.min <= min && s.max >= max, "{s:?}");
        }
        // Rainfall up to 112 on 12 ticks: steps of 15, not 20.
        assert_eq!(NiceScale::aligned(0.0, 112.0, 12).max, 165.0);
        // Steps of 2.5 keep their decimal.
        let s = NiceScale::aligned(0.0, 10.0, 5);
        assert_eq!(s.step, 2.5);
        assert_eq!(PLAIN.tick(7.5, s.step), "7.5");
    }

    #[test]
    fn datasets_stack_per_axis() {
        let data = [ds(&[1.0]), right(&[100.0]), ds(&[2.0])];
        let kinds = [Kind::Bar; 3];
        assert_eq!(plotted_value(&data, &kinds, true, 2, 0), 3.0);
        assert_eq!(plotted_value(&data, &kinds, true, 1, 0), 100.0);
        let left = side_axis(&data, Kind::Bar, true, false, 0);
        assert!(left.max < 10.0, "{left:?}");
    }

    #[test]
    fn right_datasets_use_the_right_scale() {
        let data = [ds(&[5.0]), right(&[50.0])];
        let mut req = request(Kind::Bar, &data, NO_LABELS);
        req.y2_max = 100.0;
        let bars = layout(&req).bars;
        // Both are half way up their axes.
        assert_eq!(bars[0].height, 50.0);
        assert_eq!(bars[1].height, 50.0);
    }

    #[test]
    fn scatter_points_use_their_axis_and_one_axis_charts_ignore_it() {
        let point = |y: f64| {
            vec![
                DataPoint { x: 0.0, y, r: 0.0 },
                DataPoint { x: 1.0, y, r: 0.0 },
            ]
        };
        let data = [
            Dataset {
                points: point(5.0).into(),
                ..Default::default()
            },
            Dataset {
                points: point(50.0).into(),
                right_axis: true,
                ..Default::default()
            },
        ];
        let mut req = request(Kind::Scatter, &data, NO_LABELS);
        req.y2_max = 100.0;
        let out = layout(&req);
        assert_eq!(out.series[0].points[0].y, 50.0);
        assert_eq!(out.series[1].points[0].y, 50.0);
        // Without a right range, the left axis.
        req.y2_max = 0.0;
        assert_eq!(layout(&req).series[1].points[0].y, -400.0);
        // And an axis for all datasets includes both.
        assert!(axis(&data, Kind::Scatter, false, 200.0, true).max >= 50.0);
    }

    #[test]
    fn zoomed_time_series_read_only_the_visible_points() {
        use std::cell::Cell;
        use std::rc::Rc;
        let reads = Rc::new(Cell::new(0usize));
        let counted = reads.clone();
        let len = 1_000_000;
        let data = [Dataset {
            points: Points::Lazy {
                len,
                get: Rc::new(move |i| {
                    counted.set(counted.get() + 1);
                    DataPoint {
                        x: i as f64 * 60.0,
                        y: (i % 100) as f64,
                        r: 0.0,
                    }
                }),
            },
            ..Default::default()
        }];
        let mut req = request(Kind::Scatter, &data, NO_LABELS);
        req.time = Some(TimeAxis::default());
        req.show_lines = true;
        // The newest 1,000 points.
        req.view = (0.999, 1.0);
        let out = layout(&req);
        let shown = out.series[0].points.len();
        assert!((999..=1001).contains(&shown), "{shown}");
        assert!(reads.get() < 10_000, "{} reads", reads.get());
    }

    #[test]
    fn ordered_and_unordered_windows_agree() {
        let points: Points = (0..500)
            .map(|i| DataPoint {
                x: i as f64 * 7.0,
                y: ((i * 37) % 101) as f64,
                r: 0.0,
            })
            .collect();
        for (x0, x1) in [
            (0.0, 3500.0),
            (100.0, 200.0),
            (-50.0, 3.0),
            (3490.0, 9000.0),
            (10.5, 10.6),
        ] {
            let fast: Vec<_> = points.in_window(x0, x1, true).map(|(i, _)| i).collect();
            let slow: Vec<_> = points.in_window(x0, x1, false).map(|(i, _)| i).collect();
            assert_eq!(fast, slow, "{x0}..{x1}");
        }
    }
}
