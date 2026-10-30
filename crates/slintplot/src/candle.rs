//! Geometry for candlestick and OHLC charts: candles on a time axis, merged
//! when there are more of them than room, and lines over them.

use std::rc::Rc;

use crate::layout::{
    DataPoint, Dataset, Rgba, Series, Tick, TimeAxis, ValueScale, clamp_view, columns,
    resolve_color,
};
use crate::locale::Locale;
use crate::path;
use crate::time::time_ticks_in;

/// One period: its time, and its first, highest, lowest and last value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Candle {
    pub x: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

impl Candle {
    /// The candle as it can be drawn, or `None` without a finite time or any
    /// finite value. A missing open or close takes the other; the high and
    /// low span all four values, so a high below the low, or below the
    /// close, still draws a sensible candle.
    pub fn normalized(self) -> Option<Candle> {
        if !self.x.is_finite() {
            return None;
        }
        let values = [self.open, self.high, self.low, self.close];
        let finite = || values.iter().copied().filter(|v| v.is_finite());
        let first = |order: [f64; 4]| order.into_iter().find(|v| v.is_finite());
        Some(Candle {
            x: self.x,
            open: first([self.open, self.close, self.low, self.high])?,
            close: first([self.close, self.open, self.high, self.low])?,
            high: finite().fold(f64::NEG_INFINITY, f64::max),
            low: finite().fold(f64::INFINITY, f64::min),
        })
    }

    pub fn rising(&self) -> bool {
        self.close >= self.open
    }
}

/// The candles of a chart, read on demand so that a zoomed-in chart reads
/// only those it shows. They must be in time order.
#[derive(Clone)]
pub struct Candles {
    len: usize,
    get: Rc<dyn Fn(usize) -> Candle>,
}

impl Candles {
    pub fn new(len: usize, get: Rc<dyn Fn(usize) -> Candle>) -> Self {
        Candles { len, get }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The candle at `index`, or one with NaN values when out of range.
    pub fn get(&self, index: usize) -> Candle {
        if index < self.len {
            (self.get)(index)
        } else {
            Candle {
                x: f64::NAN,
                open: f64::NAN,
                high: f64::NAN,
                low: f64::NAN,
                close: f64::NAN,
            }
        }
    }

    /// The first index whose time isn't before `x`, by binary search.
    fn partition(&self, before: impl Fn(f64) -> bool) -> usize {
        let (mut lo, mut hi) = (0, self.len);
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

    /// The indices of the candles with `x0 <= x <= x1`.
    pub fn x_window(&self, x0: f64, x1: f64) -> std::ops::Range<usize> {
        let first = self.partition(|x| x < x0);
        let end = self.partition(|x| x <= x1);
        first..end.max(first)
    }

    /// The candles that can be drawn with `x0 <= x <= x1`, with their indices.
    fn in_window(&self, x0: f64, x1: f64) -> impl Iterator<Item = (usize, Candle)> + '_ {
        self.x_window(x0, x1)
            .filter_map(|i| Some((i, self.get(i).normalized()?)))
            // Checked too: unsorted candles can fall in the search.
            .filter(move |(_, c)| c.x >= x0 && c.x <= x1)
    }
}

impl From<Vec<Candle>> for Candles {
    fn from(candles: Vec<Candle>) -> Self {
        let len = candles.len();
        Candles::new(len, Rc::new(move |i| candles[i]))
    }
}

impl std::fmt::Debug for Candles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Candles({})", self.len)
    }
}

const DAY: f64 = 86_400.0;
/// Gaps between candles examined to find the usual one; more are sampled.
const MEASURED_GAPS: usize = 1000;
/// The narrowest slot a candle gets, in pixels; closer candles are merged.
const MIN_SLOT: f32 = 3.0;
/// The share of its slot a candle's body fills.
const BODY: f32 = 0.7;

/// The part of the time axis a chart shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    /// The times at the left and right edges.
    pub x0: f64,
    pub x1: f64,
    /// The usual time between two candles.
    pub slot: f64,
}

/// The time axis for the zoom window `view`. Unzoomed, it runs from half a
/// slot before the first candle to half a slot after the last, so that both
/// are drawn whole.
pub fn window(candles: &Candles, view: (f32, f32)) -> Window {
    let n = candles.len();
    let (first, last) = (candles.get(0).x, candles.get(n.saturating_sub(1)).x);
    let (min, max) = if first.is_finite() && last.is_finite() {
        (first.min(last), first.max(last))
    } else {
        // Unsorted, or missing times at the ends: look at every candle.
        (0..n)
            .map(|i| candles.get(i).x)
            .filter(|x| x.is_finite())
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
                (lo.min(x), hi.max(x))
            })
    };
    let (min, max) = if min > max { (0.0, 0.0) } else { (min, max) };
    let slot = usual_gap(candles).unwrap_or(if max > min { max - min } else { DAY });
    let (lo, hi) = (min - slot / 2.0, max + slot / 2.0);
    let (lo, hi) = if hi > lo {
        (lo, hi)
    } else {
        // Lost in a huge time: one each way, or more.
        let widen = (min.abs() * 1e-9).max(1.0);
        (min - widen, min + widen)
    };
    let (start, end) = clamp_view(view);
    let span = hi - lo;
    Window {
        x0: lo + start as f64 * span,
        x1: lo + end as f64 * span,
        slot,
    }
}

/// The median time between neighboring candles, from a sample of the gaps.
fn usual_gap(candles: &Candles) -> Option<f64> {
    let gaps = candles.len().saturating_sub(1);
    let every = gaps.div_ceil(MEASURED_GAPS).max(1);
    let mut sample: Vec<f64> = (0..gaps)
        .step_by(every)
        .map(|i| (candles.get(i + 1).x - candles.get(i).x).abs())
        .filter(|g| g.is_finite() && *g > 0.0)
        .collect();
    if sample.is_empty() {
        return None;
    }
    let middle = sample.len() / 2;
    let (_, median, _) = sample.select_nth_unstable_by(middle, f64::total_cmp);
    Some(*median)
}

/// The lowest and highest value shown in `view`: of the candles, and of the
/// visible lines' points. With `positive`, only positive values count (a
/// logarithmic axis).
pub fn value_extent(
    candles: &Candles,
    lines: &[Dataset],
    view: (f32, f32),
    positive: bool,
) -> Option<(f64, f64)> {
    let w = window(candles, view);
    let mut extent: Option<(f64, f64)> = None;
    let mut add = |v: f64| {
        if v.is_finite() && (!positive || v > 0.0) {
            extent = Some(extent.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
        }
    };
    // Candles cut by the edges count too.
    let margin = w.slot / 2.0;
    for (_, c) in candles.in_window(w.x0 - margin, w.x1 + margin) {
        [c.open, c.high, c.low, c.close]
            .into_iter()
            .for_each(&mut add);
    }
    for d in lines.iter().filter(|d| !d.hidden) {
        for (_, p) in d.points.in_window(w.x0, w.x1, true) {
            add(p.y);
        }
    }
    extent
}

pub struct CandleRequest<'a> {
    pub candles: &'a Candles,
    /// Lines drawn over the candles, such as a moving average: their points'
    /// x values are times, like the candles'.
    pub lines: &'a [Dataset],
    pub width: f32,
    pub height: f32,
    pub scale: ValueScale,
    /// Zoom window along the time axis, as fractions of the full range.
    pub view: (f32, f32),
    /// Entry animation progress: candles grow from their middle.
    pub progress: f32,
    /// Merge candles closer than a few pixels.
    pub decimate: bool,
    pub time: TimeAxis,
    pub font_size: f32,
    pub locale: &'a Locale,
    pub palette: &'a [Rgba],
}

/// A drawn candle, in pixels: one period, or several merged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CandleBar {
    /// The first and last candle merged into this one.
    pub first: usize,
    pub last: usize,
    /// The middle of the candle, and the width of its body.
    pub x: f32,
    pub width: f32,
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
    /// The values: the first open, highest high, lowest low and last close,
    /// at the first candle's time.
    pub value: Candle,
    /// The last merged candle's time.
    pub end: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CandleLayout {
    pub bars: Vec<CandleBar>,
    /// One entry per line, in order; hidden lines have no geometry.
    pub series: Vec<Series>,
    pub x_ticks: Vec<Tick>,
    /// Width of a candle's slot, in pixels.
    pub slot: f32,
}

/// Lays out the candles and lines inside the zoom window.
pub fn layout(req: &CandleRequest) -> CandleLayout {
    if req.width <= 0.0 || req.height <= 0.0 || !(req.width.is_finite() && req.height.is_finite()) {
        return CandleLayout::default();
    }
    let w = window(req.candles, req.view);
    let (x0, x1, width) = (w.x0, w.x1, req.width);
    // A window of no width puts everything in the middle rather than at NaN.
    let x_of = |x: f64| {
        let f = (x - x0) / (x1 - x0);
        if f.is_finite() {
            (f as f32 * width).clamp(-width * 10.0, width * 11.0)
        } else {
            width / 2.0
        }
    };
    let per_pixel = (x1 - x0) / width as f64;
    let slot_px = (w.slot / per_pixel) as f32;
    // Too close to draw: merge them into wider periods, counted from the
    // time origin, so that panning keeps the same groups.
    let period = if req.decimate && slot_px < MIN_SLOT {
        w.slot * (MIN_SLOT / slot_px.max(1e-9)).ceil().min(1e15) as f64
    } else {
        w.slot
    };
    let merge = period > w.slot;
    let period_px = (period / per_pixel) as f32;
    let body = if period_px.is_finite() {
        (period_px * BODY).clamp(1.0, width.max(1.0))
    } else {
        1.0
    };
    let progress = req.progress.clamp(0.0, 1.0);
    let y_of = |v: f64| req.scale.pos(v, req.height);

    let margin = period / 2.0;
    let mut bars: Vec<CandleBar> = Vec::new();
    let mut push = |first: usize, last: usize, value: Candle, end: f64, x: f64| {
        let (high, low) = (y_of(value.high), y_of(value.low));
        let middle = (high + low) / 2.0;
        let grow = |y: f32| middle + (y - middle) * progress;
        bars.push(CandleBar {
            first,
            last,
            x: x_of(x),
            width: body,
            open: grow(y_of(value.open)),
            high: grow(high),
            low: grow(low),
            close: grow(y_of(value.close)),
            value,
            end,
        });
    };
    let mut group: Option<(i64, usize, usize, Candle, f64)> = None;
    for (i, c) in req.candles.in_window(x0 - margin, x1 + margin) {
        if !merge {
            push(i, i, c, c.x, c.x);
            continue;
        }
        // Saturates for huge times, which then share a group.
        let key = (c.x / period).floor() as i64;
        match &mut group {
            Some((k, _, last, m, end)) if *k == key => {
                *last = i;
                m.high = m.high.max(c.high);
                m.low = m.low.min(c.low);
                m.close = c.close;
                *end = c.x;
            }
            _ => {
                if let Some((k, first, last, m, end)) = group.take() {
                    push(first, last, m, end, (k as f64 + 0.5) * period);
                }
                group = Some((key, i, i, c, c.x));
            }
        }
    }
    if let Some((k, first, last, m, end)) = group {
        push(first, last, m, end, (k as f64 + 0.5) * period);
    }

    let bounds = (0.0, req.height);
    let series = req
        .lines
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let color = resolve_color(d.color, i, req.palette);
            if d.hidden {
                return Series {
                    color,
                    ..Default::default()
                };
            }
            line(d, x0, x1, &x_of, &y_of, width, bounds, color)
        })
        .collect();

    // Room for labels like "Sep 30" between ticks.
    let max_ticks = (width / (req.font_size.max(1.0) * 6.0))
        .floor()
        .clamp(2.0, 11.0) as usize;
    let origin = req.time.origin;
    // Half a pixel either side, so the rounding of the zoom window doesn't
    // drop a tick on the edge.
    let x_ticks = time_ticks_in(
        origin + x0 - per_pixel * 0.5,
        origin + x1 + per_pixel * 0.5,
        max_ticks,
        req.time.utc_offset_minutes,
        req.locale,
    )
    .into_iter()
    .map(|(t, label)| Tick {
        pos: x_of(t - origin),
        label,
    })
    .filter(|t| t.pos >= -0.5 && t.pos <= width + 0.5)
    .collect();
    CandleLayout {
        bars,
        series,
        x_ticks,
        slot: period_px.min(width * 10.0),
    }
}

/// A line through a dataset's points inside the window, continued to the
/// edges towards the nearest points outside it, and reduced to pixel columns
/// when it has many more points than pixels.
#[allow(clippy::too_many_arguments)]
fn line(
    d: &Dataset,
    x0: f64,
    x1: f64,
    x_of: &dyn Fn(f64) -> f32,
    y_of: &dyn Fn(f64) -> f32,
    width: f32,
    bounds: (f32, f32),
    color: Rgba,
) -> Series {
    let window = d.points.x_window(x0, x1);
    let inside: Vec<(usize, DataPoint)> = d
        .points
        .in_window(x0, x1, true)
        .filter(|(_, p)| p.y.is_finite())
        .collect();
    let points: Vec<crate::layout::Point> = inside
        .iter()
        .map(|&(index, p)| crate::layout::Point {
            x: x_of(p.x),
            y: y_of(p.y),
            index,
            r: 0.0,
        })
        .collect();
    if points.len() as f32 > width * 2.0 {
        return Series {
            color,
            columns: columns(&points),
            ..Default::default()
        };
    }
    let finite = |p: DataPoint| (p.x.is_finite() && p.y.is_finite()).then_some(p);
    let before = window
        .start
        .checked_sub(1)
        .and_then(|i| finite(d.points.get(i)));
    let after = finite(d.points.get(window.end));
    let at = |x: f64, a: &DataPoint, b: &DataPoint| {
        let t = if b.x != a.x {
            (x - a.x) / (b.x - a.x)
        } else {
            0.0
        };
        (x_of(x), y_of(a.y + (b.y - a.y) * t))
    };
    let mut coords: Vec<_> = points.iter().map(|p| (p.x, p.y)).collect();
    let first = inside.first().map(|p| p.1).or(after);
    if let (Some(a), Some(b)) = (before, first) {
        coords.insert(0, at(x0, &a, &b));
    }
    let last = inside.last().map(|p| p.1).or(before);
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
        color,
        line: path::line(&coords, tension, bounds),
        ..Default::default()
    }
}

/// The position in `bars` of the candle closest to `x`, at any distance.
pub fn hit(bars: &[f32], x: f32) -> Option<usize> {
    bars.iter()
        .enumerate()
        .filter(|(_, b)| b.is_finite())
        .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
        .map(|(i, _)| i)
}

/// The simple moving average of the closes over `period` candles, from the
/// first candle with `period` candles up to it. Candles without a finite
/// time or close are skipped.
pub fn moving_average(candles: &Candles, period: usize) -> Vec<DataPoint> {
    if period == 0 {
        return Vec::new();
    }
    let mut window = std::collections::VecDeque::with_capacity(period.min(4096));
    let mut sum = 0.0;
    let mut out = Vec::new();
    for i in 0..candles.len() {
        let c = candles.get(i);
        if !(c.x.is_finite() && c.close.is_finite()) {
            continue;
        }
        window.push_back(c.close);
        sum += c.close;
        if window.len() > period {
            sum -= window.pop_front().unwrap_or(0.0);
        }
        if window.len() == period {
            let y = sum / period as f64;
            // A sum of huge values can overflow; average them directly then.
            let y = if y.is_finite() {
                y
            } else {
                window.iter().map(|v| v / period as f64).sum()
            };
            out.push(DataPoint { x: c.x, y, r: 0.0 });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(x: f64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            x,
            open,
            high,
            low,
            close,
        }
    }

    /// `n` daily candles, rising by 1 a day.
    fn daily(n: usize) -> Candles {
        (0..n)
            .map(|i| {
                let v = 100.0 + i as f64;
                candle(i as f64 * DAY, v, v + 2.0, v - 2.0, v + 1.0)
            })
            .collect::<Vec<_>>()
            .into()
    }

    fn request<'a>(candles: &'a Candles, width: f32, view: (f32, f32)) -> CandleRequest<'a> {
        static LOCALE: std::sync::LazyLock<Locale> = std::sync::LazyLock::new(Locale::default);
        CandleRequest {
            candles,
            lines: &[],
            width,
            height: 100.0,
            scale: ValueScale {
                min: 0.0,
                max: 200.0,
                logarithmic: false,
            },
            view,
            progress: 1.0,
            decimate: true,
            time: TimeAxis::default(),
            font_size: 12.0,
            locale: &LOCALE,
            palette: &[],
        }
    }

    #[test]
    fn normalized_candles_span_their_values() {
        let c = candle(0.0, 10.0, 5.0, 12.0, 8.0).normalized().unwrap();
        assert_eq!((c.open, c.high, c.low, c.close), (10.0, 12.0, 5.0, 8.0));
        let c = candle(0.0, f64::NAN, f64::NAN, f64::NAN, 3.0)
            .normalized()
            .unwrap();
        assert_eq!((c.open, c.high, c.low, c.close), (3.0, 3.0, 3.0, 3.0));
        assert!(candle(f64::NAN, 1.0, 1.0, 1.0, 1.0).normalized().is_none());
        assert!(
            candle(0.0, f64::NAN, f64::INFINITY, f64::NAN, f64::NAN)
                .normalized()
                .is_none()
        );
    }

    #[test]
    fn window_leaves_half_a_slot_at_the_ends() {
        let w = window(&daily(10), (0.0, 1.0));
        assert_eq!((w.x0, w.x1, w.slot), (-DAY / 2.0, 9.5 * DAY, DAY));
        let w = window(&daily(10), (0.5, 1.0));
        assert_eq!(w.x0, 4.5 * DAY);
        let one = window(&daily(1), (0.0, 1.0));
        assert_eq!((one.x0, one.x1), (-DAY / 2.0, DAY / 2.0));
        let none = window(&Vec::new().into(), (0.0, 1.0));
        assert!(none.x1 > none.x0);
    }

    #[test]
    fn one_bar_per_candle_when_there_is_room() {
        let candles = daily(10);
        let out = layout(&request(&candles, 500.0, (0.0, 1.0)));
        assert_eq!(out.bars.len(), 10);
        assert_eq!(out.slot, 50.0);
        let b = &out.bars[0];
        assert_eq!((b.first, b.last, b.x, b.width), (0, 0, 25.0, 35.0));
        // 100 open, 101 close, 98 to 102 on an axis of 0 to 200, 100 px high.
        assert_eq!((b.open, b.close, b.high, b.low), (50.0, 49.5, 49.0, 51.0));
        assert!(b.value.rising());
        assert!(!out.x_ticks.is_empty());
    }

    #[test]
    fn dense_candles_merge_into_periods() {
        let candles = daily(10_000);
        let out = layout(&request(&candles, 300.0, (0.0, 1.0)));
        assert!(out.bars.len() <= 100 + 1, "{} bars", out.bars.len());
        assert!(out.slot >= MIN_SLOT);
        let b = &out.bars[1];
        let n = b.last - b.first + 1;
        assert!(n > 1);
        let first = candles.get(b.first);
        let last = candles.get(b.last);
        assert_eq!(b.value.open, first.open);
        assert_eq!(b.value.close, last.close);
        assert_eq!(b.value.low, first.low);
        assert_eq!(b.value.high, last.high);
        assert_eq!(b.end, last.x);
        // Every candle is in exactly one bar.
        let total: usize = out.bars.iter().map(|b| b.last - b.first + 1).sum();
        assert_eq!(total, 10_000);
    }

    #[test]
    fn a_hundred_thousand_candles_fit_the_pixels() {
        let candles = daily(100_000);
        let out = layout(&request(&candles, 800.0, (0.0, 1.0)));
        assert!(out.bars.len() <= 800 / 3 + 2, "{} bars", out.bars.len());
    }

    #[test]
    fn zoomed_in_reads_only_the_window() {
        let candles = daily(1000);
        let out = layout(&request(&candles, 500.0, (0.5, 0.51)));
        assert!(out.bars.len() <= 12, "{} bars", out.bars.len());
        assert!(out.bars.iter().all(|b| b.first >= 495 && b.last <= 515));
    }

    #[test]
    fn extent_of_the_view() {
        let candles = daily(10);
        assert_eq!(
            value_extent(&candles, &[], (0.0, 1.0), false),
            Some((98.0, 111.0))
        );
        assert_eq!(
            value_extent(&candles, &[], (0.95, 1.0), false),
            Some((107.0, 111.0))
        );
        let line = Dataset {
            points: vec![DataPoint {
                x: DAY,
                y: 500.0,
                r: 0.0,
            }]
            .into(),
            ..Default::default()
        };
        assert_eq!(
            value_extent(&candles, &[line], (0.0, 1.0), false),
            Some((98.0, 500.0))
        );
        assert_eq!(
            value_extent(&Vec::new().into(), &[], (0.0, 1.0), false),
            None
        );
    }

    #[test]
    fn lines_follow_the_time_axis() {
        let candles = daily(10);
        let average: Vec<DataPoint> = moving_average(&candles, 3);
        assert_eq!(average.len(), 8);
        assert_eq!(average[0].x, 2.0 * DAY);
        assert_eq!(average[0].y, 102.0);
        let lines = [Dataset {
            points: average.into(),
            ..Default::default()
        }];
        let out = layout(&CandleRequest {
            lines: &lines,
            ..request(&candles, 500.0, (0.0, 1.0))
        });
        assert!(out.series[0].line.starts_with("M 125.00 49.00"));
        // Zoomed in between two points, the line still crosses the plot.
        let out = layout(&CandleRequest {
            lines: &lines,
            ..request(&candles, 500.0, (0.52, 0.53))
        });
        assert!(out.series[0].line.starts_with("M 0.00"));
    }

    #[test]
    fn awkward_data_draws_nothing_strange() {
        let nan = f64::NAN;
        let candles: Candles = vec![
            candle(nan, 1.0, 2.0, 0.0, 1.0),
            candle(0.0, nan, nan, nan, nan),
            candle(1e30, f64::MAX, f64::INFINITY, f64::MIN, 0.0),
            candle(-5.0, 1.0, 1.0, 1.0, 1.0),
            candle(-5.0, 1.0, 1.0, 1.0, 1.0),
        ]
        .into();
        for width in [0.0, 1.0, 3.0, 800.0, f32::NAN, f32::INFINITY] {
            for view in [(0.0, 1.0), (0.3, 0.30001), (f32::NAN, 2.0)] {
                let out = layout(&request(&candles, width, view));
                for b in &out.bars {
                    let all = [b.x, b.width, b.open, b.high, b.low, b.close];
                    assert!(all.iter().all(|v| v.is_finite()), "{b:?}");
                }
            }
        }
        assert!(moving_average(&candles, 0).is_empty());
        assert!(moving_average(&candles, 100).is_empty());
        let huge: Candles = vec![candle(0.0, 0.0, 0.0, 0.0, f64::MAX); 4].into();
        assert!(moving_average(&huge, 2).iter().all(|p| p.y.is_finite()));
    }

    #[test]
    fn hit_finds_the_closest_candle() {
        assert_eq!(hit(&[10.0, 20.0, 30.0], 24.0), Some(1));
        assert_eq!(hit(&[10.0, 20.0, 30.0], 100.0), Some(2));
        assert_eq!(hit(&[], 1.0), None);
    }
}
