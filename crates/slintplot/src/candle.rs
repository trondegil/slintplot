//! Geometry for candlestick and OHLC charts: candles on a time axis, merged
//! when there are more of them than room, with volume bars and lines over
//! them.

use std::rc::Rc;

use crate::layout::{
    DataPoint, Dataset, Rgba, Series, Tick, TimeAxis, ValueScale, clamp_view, columns,
    resolve_color,
};
use crate::locale::Locale;
use crate::path;
use crate::time::{civil_from_days, days_from_civil, time_ticks_in};

/// One period: its time, its first, highest, lowest and last value, and the
/// volume traded in it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Candle {
    pub x: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Candle {
    /// The candle as it can be drawn, or `None` without a finite time or any
    /// finite value. A missing open or close takes the other; the high and
    /// low span all four values, so a high below the low, or below the
    /// close, still draws a sensible candle. A volume that isn't a positive
    /// number is 0.
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
            volume: if self.volume.is_finite() && self.volume > 0.0 {
                self.volume
            } else {
                0.0
            },
        })
    }

    pub fn rising(&self) -> bool {
        self.close >= self.open
    }
}

/// A candle with no values, for out-of-range reads.
const NO_CANDLE: Candle = Candle {
    x: f64::NAN,
    open: f64::NAN,
    high: f64::NAN,
    low: f64::NAN,
    close: f64::NAN,
    volume: f64::NAN,
};

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
            NO_CANDLE
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

    /// The drawable candles at `indices`.
    fn at(&self, indices: std::ops::Range<usize>) -> impl Iterator<Item = (usize, Candle)> + '_ {
        indices.filter_map(|i| Some((i, self.get(i).normalized()?)))
    }

    /// Where the time `t` falls on an axis of candle indices: between the
    /// candles before and after it, or `gap` seconds per candle beyond the
    /// first and last.
    fn index_at(&self, t: f64, gap: f64) -> f64 {
        let n = self.len;
        if n == 0 || !t.is_finite() {
            return f64::NAN;
        }
        let j = self.partition(|x| x < t);
        let pos = if j == 0 {
            (t - self.get(0).x) / gap
        } else if j == n {
            (n - 1) as f64 + (t - self.get(n - 1).x) / gap
        } else {
            let (a, b) = (self.get(j - 1).x, self.get(j).x);
            (j - 1) as f64 + ((t - a) / (b - a)).clamp(0.0, 1.0)
        };
        if pos.is_finite() { pos } else { j as f64 }
    }

    /// The time at a position on an axis of candle indices: the inverse of
    /// `index_at`.
    fn time_at(&self, pos: f64, gap: f64) -> f64 {
        let n = self.len;
        if n == 0 || !pos.is_finite() {
            return f64::NAN;
        }
        let last = (n - 1) as f64;
        if pos <= 0.0 {
            self.get(0).x + pos * gap
        } else if pos >= last {
            self.get(n - 1).x + (pos - last) * gap
        } else {
            let i = pos.floor();
            let (a, b) = (self.get(i as usize).x, self.get(i as usize + 1).x);
            a + (b - a) * (pos - i)
        }
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
const WEEK: f64 = 7.0 * DAY;
/// Gaps between candles examined to find the usual one; more are sampled.
const MEASURED_GAPS: usize = 1000;
/// The narrowest slot a candle gets, in pixels; closer candles are merged.
const MIN_SLOT: f32 = 3.0;
/// The share of its slot a candle's body fills.
const BODY: f32 = 0.7;
/// The widest body, in pixels; wider slots leave more room between candles.
const MAX_BODY: f32 = 24.0;
/// The largest time, in seconds either side of 1970, merged into calendar
/// periods; beyond it, into a whole number of usual gaps.
const CALENDAR_LIMIT: f64 = 1e13;
/// The calendar periods candles are merged into, shortest first.
const PERIODS: &[Period] = &[
    Period::Seconds(60.0),
    Period::Seconds(300.0),
    Period::Seconds(900.0),
    Period::Seconds(1800.0),
    Period::Seconds(3600.0),
    Period::Seconds(4.0 * 3600.0),
    Period::Seconds(DAY),
    Period::Seconds(WEEK),
    Period::Months(1),
    Period::Months(3),
    Period::Months(12),
    Period::Months(24),
    Period::Months(60),
    Period::Months(120),
    Period::Months(300),
    Period::Months(600),
    Period::Months(1200),
];

/// The part of the time axis a chart shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    /// The positions at the left and right edges: times, or candle indices
    /// when `indexed`.
    pub x0: f64,
    pub x1: f64,
    /// The usual distance between two candles: a time, or 1 when `indexed`.
    pub slot: f64,
    /// The usual time between two candles.
    pub gap: f64,
    /// Candles sit at their index, side by side, skipping gaps in time.
    pub indexed: bool,
}

/// The time axis for the zoom window `view`. Unzoomed, it runs from half a
/// slot before the first candle to half a slot after the last, so that both
/// are drawn whole. With `skip_gaps`, positions are candle indices.
pub fn window(candles: &Candles, view: (f32, f32), skip_gaps: bool) -> Window {
    let n = candles.len();
    let gap = usual_gap(candles);
    let (lo, hi, slot) = if skip_gaps {
        (-0.5, n.max(1) as f64 - 0.5, 1.0)
    } else {
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
        let slot = gap.unwrap_or(if max > min { max - min } else { DAY });
        let (lo, hi) = (min - slot / 2.0, max + slot / 2.0);
        let (lo, hi) = if hi > lo {
            (lo, hi)
        } else {
            // Lost in a huge time: one each way, or more.
            let widen = (min.abs() * 1e-9).max(1.0);
            (min - widen, min + widen)
        };
        (lo, hi, slot)
    };
    let (start, end) = clamp_view(view);
    let span = hi - lo;
    Window {
        x0: lo + start as f64 * span,
        x1: lo + end as f64 * span,
        slot,
        gap: gap.unwrap_or(DAY),
        indexed: skip_gaps,
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

/// A calendar period of local time.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Period {
    /// A fixed number of seconds from midnight; weeks start on Monday.
    Seconds(f64),
    Months(u32),
}

impl Period {
    fn approx_seconds(self) -> f64 {
        match self {
            Period::Seconds(s) => s,
            Period::Months(m) => m as f64 * 30.44 * DAY,
        }
    }

    /// The start and end of the period that holds the local time `t`.
    fn around(self, t: f64) -> (f64, f64) {
        match self {
            Period::Seconds(u) => {
                // 1970-01-01 was a Thursday.
                let shift = if u == WEEK { 3.0 * DAY } else { 0.0 };
                let start = ((t + shift) / u).floor() * u - shift;
                (start, start + u)
            }
            Period::Months(m) => {
                let (year, month, _) = civil_from_days((t / DAY).floor() as i64);
                let step = m as i64;
                let index = (year * 12 + month as i64 - 1).div_euclid(step) * step;
                let date = |i: i64| {
                    days_from_civil(i.div_euclid(12), i.rem_euclid(12) as u32 + 1, 1) as f64 * DAY
                };
                (date(index), date(index + step))
            }
        }
    }
}

/// How candles are merged into the drawn ones.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Group {
    /// Every candle on its own.
    Single,
    /// Every `n` candles, counted from the first: on an index axis.
    Every(usize),
    /// Periods of local time, `offset` seconds from the axis' times: on a
    /// time axis.
    Period(Period, f64),
}

impl Group {
    /// The start and end of the period that holds the time `x`.
    fn around(self, x: f64) -> (f64, f64) {
        match self {
            Group::Period(p, offset) => {
                let (start, end) = p.around(x + offset);
                (start - offset, end - offset)
            }
            _ => (x, x),
        }
    }
}

/// Everything about the candles that doesn't depend on the value axis.
struct Plan {
    w: Window,
    /// Axis units per pixel.
    per_pixel: f64,
    group: Group,
    /// Width of a drawn candle's slot, in pixels.
    slot: f32,
}

fn plan(req: &CandleRequest) -> Plan {
    let w = window(req.candles, req.view, req.skip_gaps);
    let per_pixel = (w.x1 - w.x0) / req.width as f64;
    let slot_px = (w.slot / per_pixel) as f32;
    let merge = req.decimate && slot_px < MIN_SLOT;
    // The fewest candles in a slot of MIN_SLOT pixels.
    let factor = (MIN_SLOT / slot_px.max(1e-9)).ceil().min(1e15) as f64;
    let group = if !merge {
        Group::Single
    } else if w.indexed {
        Group::Every((factor as usize).clamp(2, req.candles.len().max(2)))
    } else {
        // Merged into the shortest calendar period that's wide enough, in
        // local time, so that panning keeps the same groups.
        let offset = req.time.origin + req.time.utc_offset_minutes as f64 * 60.0;
        let needed = w.slot * factor;
        let calendar = (w.x0 + offset).abs() < CALENDAR_LIMIT
            && (w.x1 + offset).abs() < CALENDAR_LIMIT
            && needed >= 60.0;
        let period = PERIODS
            .iter()
            .copied()
            .find(|p| calendar && p.approx_seconds() >= needed)
            .unwrap_or(Period::Seconds(needed));
        Group::Period(period, offset)
    };
    let span = match group {
        Group::Single => w.slot,
        Group::Every(n) => n as f64,
        Group::Period(p, _) => p.approx_seconds(),
    };
    Plan {
        w,
        per_pixel,
        group,
        slot: (span / per_pixel) as f32,
    }
}

/// A drawn candle before it's placed in pixels.
struct Merged {
    first: usize,
    last: usize,
    /// The middle of its slot on the axis.
    pos: f64,
    value: Candle,
    end: f64,
}

impl Plan {
    /// The candles in view, merged as planned. Merged candles at the edges
    /// hold all their candles, so that they don't change while panning.
    fn merged(&self, candles: &Candles) -> Vec<Merged> {
        let (x0, x1) = (self.w.x0, self.w.x1);
        let n = candles.len();
        let index = |pos: f64| pos.clamp(0.0, n as f64) as usize;
        let found: Box<dyn Iterator<Item = (usize, Candle)>> = match self.group {
            // Candles cut by the edges count too.
            Group::Single if self.w.indexed => {
                Box::new(candles.at(index((x0 - 0.5).ceil())..index((x1 + 0.5).floor() + 1.0)))
            }
            Group::Single => {
                let margin = self.w.slot / 2.0;
                Box::new(candles.in_window(x0 - margin, x1 + margin))
            }
            Group::Every(k) => {
                let k = k as f64;
                let first = ((x0 + 0.5) / k).floor() * k;
                let end = (((x1 + 0.5) / k).floor() + 1.0) * k;
                Box::new(candles.at(index(first)..index(end)))
            }
            Group::Period(..) => {
                let (start, _) = self.group.around(x0);
                let (_, end) = self.group.around(x1);
                Box::new(
                    candles
                        .in_window(start, end)
                        // The period after the window starts at `end`; but
                        // huge times can't tell `end` from `start`.
                        .filter(move |(_, c)| c.x < end || end <= start),
                )
            }
        };
        let mut out: Vec<Merged> = Vec::new();
        let mut key = f64::NAN;
        for (i, c) in found {
            let (k, pos) = match self.group {
                Group::Single if self.w.indexed => (i as f64, i as f64),
                Group::Single => (i as f64, c.x),
                Group::Every(n) => {
                    let k = (i / n * n) as f64;
                    (k, k + (n - 1) as f64 / 2.0)
                }
                Group::Period(..) => {
                    let (start, end) = self.group.around(c.x);
                    (start, (start + end) / 2.0)
                }
            };
            match out.last_mut() {
                Some(m) if k == key => {
                    m.last = i;
                    m.value.high = m.value.high.max(c.high);
                    m.value.low = m.value.low.min(c.low);
                    m.value.close = c.close;
                    // Kept finite, to scale the volume bars by.
                    m.value.volume = (m.value.volume + c.volume).min(f64::MAX);
                    m.end = c.x;
                }
                _ => {
                    key = k;
                    out.push(Merged {
                        first: i,
                        last: i,
                        pos,
                        value: c,
                        end: c.x,
                    });
                }
            }
        }
        out
    }

    /// The position of a time on the axis.
    fn pos_of(&self, candles: &Candles, t: f64) -> f64 {
        if self.w.indexed {
            candles.index_at(t, self.w.gap)
        } else {
            t
        }
    }

    /// The time at a position on the axis.
    fn time_of(&self, candles: &Candles, pos: f64) -> f64 {
        if self.w.indexed {
            candles.time_at(pos, self.w.gap)
        } else {
            pos
        }
    }
}

/// The lowest and highest price shown: of the candles in view, merged ones
/// at the edges included, and of the visible lines' points. On a
/// logarithmic axis, only positive values count. The value axis isn't used.
pub fn value_extent(req: &CandleRequest) -> Option<(f64, f64)> {
    if !(req.width > 0.0 && req.width.is_finite()) {
        return None;
    }
    let positive = req.scale.logarithmic;
    let plan = plan(req);
    let mut extent: Option<(f64, f64)> = None;
    let mut add = |v: f64| {
        if v.is_finite() && (!positive || v > 0.0) {
            extent = Some(extent.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
        }
    };
    for m in plan.merged(req.candles) {
        let c = m.value;
        [c.open, c.high, c.low, c.close]
            .into_iter()
            .for_each(&mut add);
    }
    let t0 = plan.time_of(req.candles, plan.w.x0);
    let t1 = plan.time_of(req.candles, plan.w.x1);
    for d in req.lines.iter().filter(|d| !d.hidden) {
        for (_, p) in d.points.in_window(t0, t1, true) {
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
    /// The height of the price area.
    pub height: f32,
    /// The height of the volume bars' strip below it; 0 for none.
    pub volume_height: f32,
    pub scale: ValueScale,
    /// Zoom window along the time axis, as fractions of the full range.
    pub view: (f32, f32),
    /// Entry animation progress: candles grow from their middle.
    pub progress: f32,
    /// Merge candles closer than a few pixels.
    pub decimate: bool,
    /// Place candles side by side at their index, skipping gaps in time
    /// such as nights and weekends.
    pub skip_gaps: bool,
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
    /// The middle of the candle, and the width of its body: an odd number
    /// of pixels, so that the wick sits in its middle.
    pub x: f32,
    pub width: f32,
    /// Whole pixels, for sharp edges.
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
    /// The height of the volume bar.
    pub volume: f32,
    /// The values: the first open, highest high, lowest low, last close and
    /// total volume, at the first candle's time.
    pub value: Candle,
    /// The last merged candle's time.
    pub end: f64,
    /// The close of the candle before the first, or NaN.
    pub previous: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CandleLayout {
    pub bars: Vec<CandleBar>,
    /// One entry per line, in order; hidden lines have no geometry.
    pub series: Vec<Series>,
    pub x_ticks: Vec<Tick>,
    /// Width of a candle's slot, in pixels.
    pub slot: f32,
    /// The last candle that can be drawn, in view or not, and the position
    /// of its close.
    pub last: Option<Candle>,
    pub last_y: f32,
    /// The volume of a full-height volume bar.
    pub volume_max: f64,
}

/// The largest odd number of pixels up to `width`, and at least 1.
fn odd(width: f32) -> f32 {
    if width.is_nan() || width < 1.0 {
        return 1.0;
    }
    let w = width.min(1e6).floor();
    if w % 2.0 == 0.0 { w - 1.0 } else { w }
}

/// Lays out the candles and lines inside the zoom window.
pub fn layout(req: &CandleRequest) -> CandleLayout {
    if req.width <= 0.0 || req.height <= 0.0 || !(req.width.is_finite() && req.height.is_finite()) {
        return CandleLayout::default();
    }
    let plan = plan(req);
    let (x0, x1, width) = (plan.w.x0, plan.w.x1, req.width);
    // A window of no width puts everything in the middle rather than at NaN.
    let x_of = |x: f64| {
        let f = (x - x0) / (x1 - x0);
        if f.is_finite() {
            (f as f32 * width).clamp(-width * 10.0, width * 11.0)
        } else {
            width / 2.0
        }
    };
    // Room between candles: at least a pixel, and more when the body is at
    // its widest.
    let body = if plan.slot.is_finite() {
        odd((plan.slot * BODY).min(plan.slot - 1.0).min(MAX_BODY))
    } else {
        1.0
    };
    let progress = req.progress.clamp(0.0, 1.0);
    let y_of = |v: f64| req.scale.pos(v, req.height);

    let merged = plan.merged(req.candles);
    let volume_max = merged.iter().map(|m| m.value.volume).fold(0.0, f64::max);
    let volume_height = if req.volume_height.is_finite() {
        req.volume_height.max(0.0)
    } else {
        0.0
    };
    let bars = merged
        .into_iter()
        .map(|m| {
            let value = m.value;
            let (high, low) = (y_of(value.high), y_of(value.low));
            let middle = (high + low) / 2.0;
            let grow = |y: f32| (middle + (y - middle) * progress).round();
            let volume = if volume_max > 0.0 {
                (value.volume / volume_max) as f32 * volume_height * progress
            } else {
                0.0
            };
            let previous = m
                .first
                .checked_sub(1)
                .and_then(|i| req.candles.get(i).normalized())
                .map_or(f64::NAN, |c| c.close);
            CandleBar {
                first: m.first,
                last: m.last,
                x: x_of(m.pos).round(),
                width: body,
                open: grow(y_of(value.open)),
                high: grow(high),
                low: grow(low),
                close: grow(y_of(value.close)),
                volume: volume.round(),
                value,
                end: m.end,
                previous,
            }
        })
        .collect();

    let bounds = (0.0, req.height);
    let t0 = plan.time_of(req.candles, x0);
    let t1 = plan.time_of(req.candles, x1);
    let x_of_time = |t: f64| x_of(plan.pos_of(req.candles, t));
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
            line(d, t0, t1, &x_of_time, &y_of, width, bounds, color)
        })
        .collect();

    // The last candle, for a line at its close; not far back, so that a
    // long tail of empty candles costs little.
    let n = req.candles.len();
    let last = (n.saturating_sub(64)..n)
        .rev()
        .find_map(|i| req.candles.get(i).normalized());
    let last_y = last.map_or(0.0, |c| y_of(c.close).round());

    CandleLayout {
        bars,
        series,
        x_ticks: x_ticks(req, &plan, &x_of),
        slot: plan.slot.min(width * 10.0),
        last,
        last_y,
        volume_max,
    }
}

/// Ticks on round times. On an index axis, each tick sits at the candle
/// where its time falls, or between two close candles; one in a gap goes to
/// the first candle after it, if that's before the next tick.
fn x_ticks(req: &CandleRequest, plan: &Plan, x_of: &dyn Fn(f64) -> f32) -> Vec<Tick> {
    let width = req.width;
    // Room for labels like "Sep 30" between ticks.
    let label = req.font_size.max(1.0) * 6.0;
    let max_ticks = (width / label).floor().clamp(2.0, 11.0) as usize;
    let origin = req.time.origin;
    // Half a pixel either side, so the rounding of the zoom window doesn't
    // drop a tick on the edge.
    let half = plan.per_pixel * 0.5;
    let (start, end) = (
        plan.time_of(req.candles, plan.w.x0 - half),
        plan.time_of(req.candles, plan.w.x1 + half),
    );
    let ticks = time_ticks_in(
        origin + start,
        origin + end,
        max_ticks,
        req.time.utc_offset_minutes,
        req.locale,
    );
    let in_view = |pos: f32| pos >= -0.5 && pos <= width + 0.5;
    if !plan.w.indexed {
        return ticks
            .into_iter()
            .map(|(t, label)| Tick {
                pos: x_of(t - origin),
                label,
            })
            .filter(|t| in_view(t.pos))
            .collect();
    }
    let candles = req.candles;
    let n = candles.len();
    let step = match ticks.as_slice() {
        [a, b, ..] => b.0 - a.0,
        _ => f64::INFINITY,
    };
    let mut out: Vec<Tick> = Vec::new();
    for (k, (t, text)) in ticks.iter().enumerate() {
        let x = t - origin;
        let next = ticks.get(k + 1).map_or(t + step, |n| n.0) - origin;
        let j = candles.partition(|c| c < x);
        if j == n || (j == 0 && candles.get(0).x != x) {
            continue;
        }
        let (a, b) = (candles.get(j.saturating_sub(1)).x, candles.get(j).x);
        let pos = if b == x {
            j as f64
        } else if b - a <= plan.w.gap * 1.5 {
            (j - 1) as f64 + ((x - a) / (b - a)).clamp(0.0, 1.0)
        } else if b < next {
            j as f64
        } else {
            continue;
        };
        let pos = x_of(pos);
        // Gaps squeezed out can bring ticks too close for their labels.
        let room = out.last().is_none_or(|p| pos - p.pos >= label * 0.75);
        if pos.is_finite() && in_view(pos) && room {
            out.push(Tick {
                pos,
                label: text.clone(),
            });
        }
    }
    out
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

/// The value of a line at the time `x`: of the point there, or between the
/// points either side. `None` outside its points, or when hidden.
pub fn value_at(d: &Dataset, x: f64) -> Option<f64> {
    if d.hidden || !x.is_finite() {
        return None;
    }
    let j = d.points.x_window(x, f64::INFINITY).start;
    let b = d.points.get(j);
    if b.x == x && b.y.is_finite() {
        return Some(b.y);
    }
    let a = d.points.get(j.checked_sub(1)?);
    let y = a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x);
    (a.x < x && x < b.x && y.is_finite()).then_some(y)
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
            volume: 0.0,
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

    /// `n` trading days from Monday 5 January 2026, without weekends, each
    /// with a volume of 1000 times its number plus one.
    fn trading(n: usize) -> Candles {
        let monday = crate::time::unix_seconds(2026, 1, 5, 0, 0, 0);
        (0..n)
            .map(|i| {
                let day = i / 5 * 7 + i % 5;
                let v = 100.0 + i as f64;
                Candle {
                    volume: 1000.0 * (i + 1) as f64,
                    ..candle(monday + day as f64 * DAY, v, v + 2.0, v - 2.0, v + 1.0)
                }
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
            volume_height: 0.0,
            scale: ValueScale {
                min: 0.0,
                max: 200.0,
                logarithmic: false,
            },
            view,
            progress: 1.0,
            decimate: true,
            skip_gaps: false,
            time: TimeAxis::default(),
            font_size: 12.0,
            locale: &LOCALE,
            palette: &[],
        }
    }

    fn indexed<'a>(candles: &'a Candles, width: f32, view: (f32, f32)) -> CandleRequest<'a> {
        CandleRequest {
            skip_gaps: true,
            ..request(candles, width, view)
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
        for volume in [f64::NAN, -5.0, f64::INFINITY] {
            let c = Candle {
                volume,
                ..candle(0.0, 1.0, 1.0, 1.0, 1.0)
            };
            assert_eq!(c.normalized().unwrap().volume, 0.0);
        }
    }

    #[test]
    fn window_leaves_half_a_slot_at_the_ends() {
        let w = window(&daily(10), (0.0, 1.0), false);
        assert_eq!((w.x0, w.x1, w.slot), (-DAY / 2.0, 9.5 * DAY, DAY));
        let w = window(&daily(10), (0.5, 1.0), false);
        assert_eq!(w.x0, 4.5 * DAY);
        let one = window(&daily(1), (0.0, 1.0), false);
        assert_eq!((one.x0, one.x1), (-DAY / 2.0, DAY / 2.0));
        let none = window(&Vec::new().into(), (0.0, 1.0), false);
        assert!(none.x1 > none.x0);
        // On an index axis, from half a candle before the first.
        let w = window(&trading(10), (0.0, 1.0), true);
        assert_eq!((w.x0, w.x1, w.slot, w.gap), (-0.5, 9.5, 1.0, DAY));
        let none = window(&Vec::new().into(), (0.0, 1.0), true);
        assert!(none.x1 > none.x0);
    }

    #[test]
    fn one_bar_per_candle_when_there_is_room() {
        let candles = daily(10);
        let out = layout(&request(&candles, 200.0, (0.0, 1.0)));
        assert_eq!(out.bars.len(), 10);
        assert_eq!(out.slot, 20.0);
        let b = &out.bars[0];
        // 70% of the slot, as an odd number of pixels.
        assert_eq!((b.first, b.last, b.x, b.width), (0, 0, 10.0, 13.0));
        // 100 open, 101 close, 98 to 102 on an axis of 0 to 200, 100 px
        // high, in whole pixels.
        assert_eq!((b.open, b.close, b.high, b.low), (50.0, 50.0, 49.0, 51.0));
        assert!(b.value.rising());
        assert!(b.previous.is_nan());
        assert_eq!(out.bars[1].previous, 101.0);
        assert!(!out.x_ticks.is_empty());
    }

    #[test]
    fn bodies_have_a_widest_and_a_gap() {
        // One candle, alone in a wide plot, still looks like a candle.
        for skip_gaps in [false, true] {
            let one = daily(1);
            let out = layout(&CandleRequest {
                skip_gaps,
                ..request(&one, 800.0, (0.0, 1.0))
            });
            assert_eq!(out.bars.len(), 1);
            assert_eq!((out.bars[0].x, out.bars[0].width), (400.0, 23.0));
        }
        // Narrow slots keep a gap between bodies.
        for width in [300.0, 400.0, 500.0, 700.0] {
            let candles = daily(100);
            let out = layout(&request(&candles, width, (0.0, 1.0)));
            let b = out.bars[0].width;
            assert!(b % 2.0 == 1.0 && b < out.slot, "{b} in {}", out.slot);
        }
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
        assert_eq!(b.previous, candles.get(b.first - 1).close);
        // Every candle is in exactly one bar.
        let total: usize = out.bars.iter().map(|b| b.last - b.first + 1).sum();
        assert_eq!(total, 10_000);
    }

    #[test]
    fn merged_periods_follow_the_local_calendar() {
        // Ten years of days from 1970: merged into calendar periods.
        let candles = daily(3650);
        for (width, days) in [(3000.0, 7..=7), (500.0, 28..=31), (150.0, 90..=92)] {
            let out = layout(&request(&candles, width, (0.0, 1.0)));
            for b in &out.bars[1..out.bars.len() - 1] {
                let n = b.last - b.first + 1;
                assert!(days.contains(&n), "{n} days at {width} px");
                let (year, month, day) = civil_from_days((b.value.x / DAY) as i64);
                match n {
                    // Weeks start on Monday; 1970-01-05 was one.
                    7 => assert_eq!((b.value.x / DAY) as i64 % 7, 4),
                    28..=31 => assert_eq!(day, 1),
                    _ => assert_eq!((day, (month - 1) % 3), (1, 0), "{year}"),
                }
            }
        }
        // A day in UTC+2 starts at 22:00 UTC.
        let hourly: Candles = (0..2000)
            .map(|i| candle(i as f64 * 3600.0, 1.0, 2.0, 0.5, 1.5))
            .collect::<Vec<_>>()
            .into();
        let out = layout(&CandleRequest {
            time: TimeAxis {
                origin: 0.0,
                utc_offset_minutes: 120,
            },
            ..request(&hourly, 300.0, (0.0, 1.0))
        });
        assert_eq!(out.bars[1].value.x, 22.0 * 3600.0);
        assert_eq!(out.bars[1].last - out.bars[1].first + 1, 24);
    }

    #[test]
    fn merged_candles_at_the_edges_are_whole() {
        // While panning, a merged candle cut by an edge keeps its values.
        for (candles, skip_gaps) in [(daily(5000), false), (trading(5000), true)] {
            let bar_of = |start: f32| {
                let out = layout(&CandleRequest {
                    skip_gaps,
                    ..request(&candles, 400.0, (start, start + 0.3))
                });
                let b = out.bars.iter().find(|b| b.first <= 2500 && 2500 <= b.last);
                b.map(|b| (b.first, b.last, b.value))
            };
            let bars: Vec<_> = (0..40).map(|i| bar_of(0.21 + i as f32 * 0.002)).collect();
            assert!(bars.iter().flatten().count() > 30);
            for b in bars.iter().flatten() {
                assert_eq!(Some(b), bars[0].as_ref());
            }
        }
    }

    #[test]
    fn a_hundred_thousand_candles_fit_the_pixels() {
        for skip_gaps in [false, true] {
            let candles = daily(100_000);
            let out = layout(&CandleRequest {
                skip_gaps,
                ..request(&candles, 800.0, (0.0, 1.0))
            });
            assert!(out.bars.len() <= 800 / 3 + 2, "{} bars", out.bars.len());
        }
    }

    #[test]
    fn zoomed_in_reads_only_the_window() {
        let candles = daily(1000);
        let out = layout(&request(&candles, 500.0, (0.5, 0.51)));
        assert!(out.bars.len() <= 12, "{} bars", out.bars.len());
        assert!(out.bars.iter().all(|b| b.first >= 495 && b.last <= 515));
        let out = layout(&indexed(&candles, 500.0, (0.5, 0.51)));
        assert!(out.bars.len() <= 12, "{} bars", out.bars.len());
        assert!(out.bars.iter().all(|b| b.first >= 495 && b.last <= 515));
    }

    #[test]
    fn skipping_gaps_puts_candles_side_by_side() {
        let candles = trading(10);
        let out = layout(&indexed(&candles, 500.0, (0.0, 1.0)));
        assert_eq!(out.slot, 50.0);
        let xs: Vec<f32> = out.bars.iter().map(|b| b.x).collect();
        assert_eq!(
            xs,
            [
                25.0, 75.0, 125.0, 175.0, 225.0, 275.0, 325.0, 375.0, 425.0, 475.0
            ]
        );
        // Ticks fall on candles, Mondays included, not in the weekend.
        assert!(!out.x_ticks.is_empty());
        for t in &out.x_ticks {
            assert!(xs.contains(&t.pos), "{t:?}");
        }
        // On a time axis, the weekend is a gap.
        let out = layout(&request(&candles, 500.0, (0.0, 1.0)));
        assert!(out.bars[5].x - out.bars[4].x > 2.0 * (out.bars[4].x - out.bars[3].x));
    }

    #[test]
    fn skipping_gaps_merges_every_few_candles() {
        let candles = trading(1000);
        let out = layout(&indexed(&candles, 600.0, (0.0, 1.0)));
        let n = out.bars[0].last - out.bars[0].first + 1;
        assert_eq!(n, 5);
        assert!(out.bars.iter().all(|b| b.first % 5 == 0));
        let total: usize = out.bars.iter().map(|b| b.last - b.first + 1).sum();
        assert_eq!(total, 1000);
    }

    #[test]
    fn times_map_onto_the_index_axis() {
        let candles = trading(10);
        let monday = candles.get(0).x;
        assert_eq!(candles.index_at(monday, DAY), 0.0);
        assert_eq!(candles.index_at(monday + 0.5 * DAY, DAY), 0.5);
        // Saturday is between Friday and Monday.
        let saturday = monday + 5.0 * DAY;
        assert_eq!(candles.index_at(saturday, DAY), 4.0 + 1.0 / 3.0);
        assert_eq!(candles.index_at(monday - DAY, DAY), -1.0);
        for pos in [-2.0, 0.0, 0.25, 4.5, 9.0, 11.0] {
            let t = candles.time_at(pos, DAY);
            assert!((candles.index_at(t, DAY) - pos).abs() < 1e-9, "{pos}");
        }
        assert!(candles.index_at(f64::NAN, DAY).is_nan());
        let none: Candles = Vec::new().into();
        assert!(none.time_at(1.0, DAY).is_nan());
    }

    #[test]
    fn lines_follow_the_index_axis() {
        let candles = trading(10);
        let lines = [Dataset {
            points: moving_average(&candles, 3).into(),
            ..Default::default()
        }];
        let out = layout(&CandleRequest {
            lines: &lines,
            ..indexed(&candles, 500.0, (0.0, 1.0))
        });
        // The first average is at the third candle, and the line runs
        // straight across the weekend.
        assert!(
            out.series[0].line.starts_with("M 125.00 49.00"),
            "{}",
            out.series[0].line
        );
        assert_eq!(value_at(&lines[0], candles.get(5).x), Some(105.0));
        let middle = (candles.get(4).x + candles.get(5).x) / 2.0;
        assert_eq!(value_at(&lines[0], middle), Some(104.5));
        assert_eq!(value_at(&lines[0], candles.get(0).x), None);
        assert_eq!(value_at(&lines[0], f64::NAN), None);
        let hidden = Dataset {
            hidden: true,
            ..lines[0].clone()
        };
        assert_eq!(value_at(&hidden, candles.get(5).x), None);
    }

    #[test]
    fn volume_bars_fill_their_strip() {
        let candles = trading(10);
        let out = layout(&CandleRequest {
            volume_height: 20.0,
            ..indexed(&candles, 500.0, (0.0, 1.0))
        });
        assert_eq!(out.volume_max, 10_000.0);
        assert_eq!(out.bars[9].volume, 20.0);
        assert_eq!(out.bars[4].volume, 10.0);
        // Merged candles add up their volume.
        let candles = trading(1000);
        let out = layout(&CandleRequest {
            volume_height: 20.0,
            ..indexed(&candles, 600.0, (0.0, 1.0))
        });
        assert_eq!(out.bars[0].value.volume, 15_000.0);
        // Without a strip, no bars.
        let out = layout(&indexed(&candles, 600.0, (0.0, 1.0)));
        assert!(out.bars.iter().all(|b| b.volume == 0.0));
    }

    #[test]
    fn the_last_candle_is_found_in_view_or_not() {
        let candles = trading(10);
        let out = layout(&indexed(&candles, 500.0, (0.0, 0.2)));
        assert_eq!(out.last.map(|c| c.close), Some(110.0));
        assert_eq!(out.last_y, 45.0);
        let mut tail = vec![NO_CANDLE; 10];
        tail.insert(0, candle(0.0, 1.0, 2.0, 0.0, 1.5));
        let out = layout(&request(&tail.into(), 500.0, (0.0, 1.0)));
        assert_eq!(out.last.map(|c| c.close), Some(1.5));
        let out = layout(&request(&Vec::new().into(), 500.0, (0.0, 1.0)));
        assert_eq!(out.last, None);
    }

    #[test]
    fn extent_of_the_view() {
        let candles = daily(10);
        let extent = |candles: &Candles, lines: &[Dataset], view| {
            value_extent(&CandleRequest {
                lines,
                ..request(candles, 500.0, view)
            })
        };
        assert_eq!(extent(&candles, &[], (0.0, 1.0)), Some((98.0, 111.0)));
        assert_eq!(extent(&candles, &[], (0.95, 1.0)), Some((107.0, 111.0)));
        let line = Dataset {
            points: vec![DataPoint {
                x: DAY,
                y: 500.0,
                r: 0.0,
            }]
            .into(),
            ..Default::default()
        };
        assert_eq!(extent(&candles, &[line], (0.0, 1.0)), Some((98.0, 500.0)));
        assert_eq!(extent(&Vec::new().into(), &[], (0.0, 1.0)), None);
        let trading = trading(10);
        assert_eq!(
            value_extent(&indexed(&trading, 500.0, (0.95, 1.0))),
            Some((107.0, 111.0))
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
            Candle {
                volume: f64::MAX,
                ..candle(1e30, f64::MAX, f64::INFINITY, f64::MIN, 0.0)
            },
            Candle {
                volume: f64::MAX,
                ..candle(-5.0, 1.0, 1.0, 1.0, 1.0)
            },
            candle(-5.0, 1.0, 1.0, 1.0, 1.0),
            candle(-1e300, 1.0, 1.0, 1.0, 1.0),
        ]
        .into();
        let lines = [Dataset {
            points: vec![
                DataPoint {
                    x: nan,
                    y: 1.0,
                    r: 0.0,
                },
                DataPoint {
                    x: 1e30,
                    y: f64::MAX,
                    r: 0.0,
                },
            ]
            .into(),
            ..Default::default()
        }];
        for skip_gaps in [false, true] {
            for width in [0.0, 1.0, 3.0, 800.0, f32::NAN, f32::INFINITY] {
                for view in [(0.0, 1.0), (0.3, 0.30001), (f32::NAN, 2.0)] {
                    let req = CandleRequest {
                        skip_gaps,
                        lines: &lines,
                        volume_height: 20.0,
                        ..request(&candles, width, view)
                    };
                    let _ = value_extent(&req);
                    let out = layout(&req);
                    for b in &out.bars {
                        let all = [b.x, b.width, b.open, b.high, b.low, b.close, b.volume];
                        assert!(all.iter().all(|v| v.is_finite()), "{b:?}");
                    }
                    assert!(out.x_ticks.iter().all(|t| t.pos.is_finite()));
                    assert!(out.last_y.is_finite());
                }
            }
        }
        assert!(moving_average(&candles, 0).is_empty());
        assert!(moving_average(&candles, 100).is_empty());
        let huge: Candles = vec![candle(0.0, 0.0, 0.0, 0.0, f64::MAX); 4].into();
        assert!(moving_average(&huge, 2).iter().all(|p| p.y.is_finite()));
        // Huge times merge without trouble.
        let far: Candles = (0..5000)
            .map(|i| candle(1e20 + i as f64 * 1e6, 1.0, 2.0, 0.0, 1.0))
            .collect::<Vec<_>>()
            .into();
        let out = layout(&request(&far, 300.0, (0.0, 1.0)));
        assert!(out.bars.len() <= 5000);
    }

    #[test]
    fn hit_finds_the_closest_candle() {
        assert_eq!(hit(&[10.0, 20.0, 30.0], 24.0), Some(1));
        assert_eq!(hit(&[10.0, 20.0, 30.0], 100.0), Some(2));
        assert_eq!(hit(&[], 1.0), None);
    }
}
