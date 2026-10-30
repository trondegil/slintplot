//! Gantt charts: one row per task, with a bar from its start to its end on a
//! time axis. Tasks with the same group are placed together under the
//! group's heading. Times are seconds after the chart's time origin.

use std::collections::HashMap;

use crate::layout::{Tick, TimeAxis, clamp_view};
use crate::locale::Locale;
use crate::time::time_ticks_in;

const DAY: f64 = 86_400.0;

/// A task of a Gantt chart.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Task {
    pub start: f64,
    /// A task that ends where it starts is a milestone.
    pub end: f64,
    /// Tasks with the same group are placed together. An empty group has no
    /// heading.
    pub group: String,
    /// The share done, from 0 to 1.
    pub progress: f64,
}

impl Task {
    /// The task's times in order, or the one finite time for both ends;
    /// `None` without a finite time.
    pub fn span(&self) -> Option<(f64, f64)> {
        match (self.start.is_finite(), self.end.is_finite()) {
            (true, true) => Some((self.start.min(self.end), self.start.max(self.end))),
            (true, false) => Some((self.start, self.start)),
            (false, true) => Some((self.end, self.end)),
            (false, false) => None,
        }
    }

    /// The share done, from 0 to 1; 0 when missing.
    pub fn done(&self) -> f64 {
        if self.progress.is_nan() {
            0.0
        } else {
            self.progress.clamp(0.0, 1.0)
        }
    }
}

/// A row of a Gantt chart, from the top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// The heading of a group, by its index in [`Rows::groups`].
    Heading(usize),
    /// A task, by its index in the input.
    Task(usize),
}

/// The rows of a Gantt chart. They depend only on the tasks, not on the
/// chart's size or zoom.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rows {
    /// Group labels in order of first appearance; "" holds the tasks without
    /// a group.
    pub groups: Vec<String>,
    /// The group of each task.
    pub group_of: Vec<usize>,
    pub rows: Vec<Row>,
    /// The time from the first start to the last end of each group's tasks.
    pub spans: Vec<Option<(f64, f64)>>,
}

/// Places the tasks group by group, in order of first appearance, each group
/// under its heading.
pub fn rows(tasks: &[Task]) -> Rows {
    let mut out = Rows::default();
    let mut lookup: HashMap<&str, usize> = HashMap::new();
    let mut members: Vec<Vec<usize>> = Vec::new();
    for (index, task) in tasks.iter().enumerate() {
        let group = *lookup.entry(task.group.as_str()).or_insert_with(|| {
            out.groups.push(task.group.clone());
            out.spans.push(None);
            members.push(Vec::new());
            out.groups.len() - 1
        });
        out.group_of.push(group);
        members[group].push(index);
        if let Some((a, b)) = task.span() {
            let span = &mut out.spans[group];
            *span = Some(span.map_or((a, b), |(lo, hi)| (lo.min(a), hi.max(b))));
        }
    }
    for (group, tasks) in members.into_iter().enumerate() {
        if !out.groups[group].is_empty() {
            out.rows.push(Row::Heading(group));
        }
        out.rows.extend(tasks.into_iter().map(Row::Task));
    }
    out
}

/// The times shown when not zoomed: all tasks, with a little room at both
/// ends. Without tasks, the day after the origin.
pub fn extent(tasks: &[Task]) -> (f64, f64) {
    let Some((min, max)) = tasks
        .iter()
        .filter_map(Task::span)
        .reduce(|(lo, hi), (a, b)| (lo.min(a), hi.max(b)))
    else {
        return (0.0, DAY);
    };
    let span = max - min;
    if span > 0.0 {
        (min - span * 0.02, max + span * 0.02)
    } else {
        // Half a day each way, or more where that is lost in a huge time.
        let widen = (min.abs() * 1e-9).max(DAY / 2.0);
        (min - widen, max + widen)
    }
}

/// The times at the left and right edges for the zoom window `view`
/// (fractions of the extent).
pub fn window(tasks: &[Task], view: (f32, f32)) -> (f64, f64) {
    let (min, max) = extent(tasks);
    let (start, end) = clamp_view(view);
    let span = max - min;
    (min + start as f64 * span, min + end as f64 * span)
}

/// A bar of a task, or of a group's span on its heading.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bar {
    /// The task's index in the input, or the group's in [`Rows::groups`].
    pub index: usize,
    /// The row, from the top.
    pub row: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// The width of the part done.
    pub done: f32,
    /// A diamond centered on `x + width / 2`, instead of a bar.
    pub milestone: bool,
}

/// A row in view, with its top relative to the plot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibleRow {
    pub row: usize,
    pub y: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gantt {
    pub rows: Vec<VisibleRow>,
    pub bars: Vec<Bar>,
    /// The groups' spans, on their headings.
    pub summaries: Vec<Bar>,
    pub ticks: Vec<Tick>,
    /// The position of the "now" line, if it is in view.
    pub now: Option<f32>,
}

pub struct GanttRequest<'a> {
    pub tasks: &'a [Task],
    pub rows: &'a Rows,
    /// The times at the left and right edges, from [`window`].
    pub window: (f64, f64),
    pub width: f32,
    pub height: f32,
    /// The height of a row, and how far the rows are scrolled up.
    pub row_height: f32,
    pub scroll: f32,
    /// The entry animation, from 0 to 1: bars grow from their start.
    pub progress: f32,
    pub font_size: f32,
    pub time: TimeAxis,
    /// The time of the "now" line, in seconds after the origin.
    pub now: Option<f64>,
    pub locale: &'a Locale,
}

/// Lays out the rows in view, and the bars on them that are in the window.
pub fn layout(req: &GanttRequest) -> Gantt {
    let width = req.width.max(0.0);
    let height = req.height.max(0.0);
    if !width.is_finite() {
        return Gantt::default();
    }
    let step = req.row_height;
    let (x0, x1) = req.window;
    let mut out = Gantt::default();
    if !(step > 0.0 && step.is_finite()) || width <= 0.0 || x1.partial_cmp(&x0).is_none() {
        return out;
    }
    let x_of = |t: f64| {
        let f = (t - x0) / (x1 - x0);
        if f.is_finite() {
            // Bars far outside the window are cut a little past its edges.
            ((f * width as f64) as f32).clamp(-step, width + step)
        } else {
            width / 2.0
        }
    };
    let scroll = if req.scroll.is_finite() {
        req.scroll.max(0.0)
    } else {
        0.0
    };
    let count = req.rows.rows.len();
    let first = ((scroll / step).floor() as usize).min(count);
    let last = (((scroll + height) / step).ceil().max(0.0) as usize).min(count);
    let grow = req.progress.clamp(0.0, 1.0);
    let bar = |index: usize, row: usize, (a, b): (f64, f64), thickness: f32, done: f64| {
        let y = row as f32 * step - scroll + (step - thickness) / 2.0;
        let (left, right) = (x_of(a), x_of(b));
        if a == b {
            let size = thickness * grow;
            return (left >= -size && left <= width + size).then_some(Bar {
                index,
                row,
                x: left - size / 2.0,
                y: y + (thickness - size) / 2.0,
                width: size,
                height: size,
                done: 0.0,
                milestone: true,
            });
        }
        let full = (right - left).max(0.0);
        // Out of view on either side.
        if right < 0.0 || left > width {
            return None;
        }
        let shown = full * grow;
        Some(Bar {
            index,
            row,
            x: left,
            y,
            width: shown,
            height: thickness,
            done: shown * done as f32,
            milestone: false,
        })
    };
    let thickness = (step * 0.62).min(req.font_size.max(1.0) * 2.4);
    for row in first..last {
        out.rows.push(VisibleRow {
            row,
            y: row as f32 * step - scroll,
        });
        match req.rows.rows[row] {
            Row::Task(index) => {
                let task = &req.tasks[index];
                if let Some(bar) = task
                    .span()
                    .and_then(|s| bar(index, row, s, thickness, task.done()))
                {
                    out.bars.push(bar);
                }
            }
            Row::Heading(group) => {
                if let Some(span) = req.rows.spans[group]
                    && span.1 > span.0
                    && let Some(bar) = bar(group, row, span, thickness * 0.35, 0.0)
                {
                    out.summaries.push(bar);
                }
            }
        }
    }
    // Room for labels like "Oct 25" between ticks, so a plan of a few months
    // has weekly ticks. Half a pixel either side, so the rounding of the zoom
    // window doesn't drop a tick on the edge.
    let max_ticks = (width / (req.font_size.max(1.0) * 4.0))
        .floor()
        .clamp(2.0, 20.0) as usize;
    let pad = (x1 - x0) / width as f64 * 0.5;
    out.ticks = time_ticks_in(
        req.time.origin + x0 - pad,
        req.time.origin + x1 + pad,
        max_ticks,
        req.time.utc_offset_minutes,
        req.locale,
    )
    .into_iter()
    .map(|(t, label)| Tick {
        pos: ((t - req.time.origin - x0) / (x1 - x0) * width as f64) as f32,
        label,
    })
    .filter(|t| t.pos >= -0.5 && t.pos <= width + 0.5)
    .collect();
    out.now = req
        .now
        .filter(|t| *t >= x0 && *t <= x1)
        .map(|t| ((t - x0) / (x1 - x0) * width as f64) as f32)
        .filter(|x| x.is_finite());
    out
}

/// A duration in days, hours, minutes or seconds, such as "14 d", "2 d 6 h"
/// or "3 h 20 min". The unit symbols are the same in every language.
pub fn duration(seconds: f64, locale: &Locale) -> String {
    if !seconds.is_finite() {
        return String::new();
    }
    let s = seconds.abs();
    let n = |v: f64| locale.number(v, 0);
    let (big, small, per, units) = if s >= DAY {
        (s / DAY, 24.0, 1.0 / 24.0, ("d", "h"))
    } else if s >= 3600.0 {
        (s / 3600.0, 60.0, 1.0 / 60.0, ("h", "min"))
    } else if s >= 60.0 {
        return format!("{} min", n((s / 60.0).round()));
    } else {
        return format!("{} s", n(s.round()));
    };
    // Past ten days or hours, the smaller unit says little.
    if big >= 10.0 {
        return format!("{} {}", n(big.round()), units.0);
    }
    let whole = big.floor();
    let rest = ((big - whole) / per).round();
    let (whole, rest) = if rest >= small {
        (whole + 1.0, 0.0)
    } else {
        (whole, rest)
    };
    if rest == 0.0 {
        format!("{} {}", n(whole), units.0)
    } else {
        format!("{} {} {} {}", n(whole), units.0, n(rest), units.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(start: f64, end: f64, group: &str) -> Task {
        Task {
            start,
            end,
            group: group.into(),
            progress: 0.0,
        }
    }

    fn request<'a>(tasks: &'a [Task], rows: &'a Rows, locale: &'a Locale) -> GanttRequest<'a> {
        GanttRequest {
            tasks,
            rows,
            window: (0.0, 100.0),
            width: 200.0,
            height: 100.0,
            row_height: 20.0,
            scroll: 0.0,
            progress: 1.0,
            font_size: 12.0,
            time: TimeAxis::default(),
            now: None,
            locale,
        }
    }

    #[test]
    fn tasks_are_placed_under_their_group() {
        let tasks = [
            task(0.0, 10.0, "Build"),
            task(5.0, 8.0, ""),
            task(10.0, 20.0, "Test"),
            task(2.0, 4.0, "Build"),
        ];
        let rows = rows(&tasks);
        assert_eq!(rows.groups, ["Build", "", "Test"]);
        assert_eq!(rows.group_of, [0, 1, 2, 0]);
        assert_eq!(
            rows.rows,
            [
                Row::Heading(0),
                Row::Task(0),
                Row::Task(3),
                Row::Task(1),
                Row::Heading(2),
                Row::Task(2),
            ],
            "tasks without a group have no heading"
        );
        assert_eq!(rows.spans[0], Some((0.0, 10.0)));
    }

    #[test]
    fn bars_run_from_start_to_end() {
        let mut tasks = [task(10.0, 60.0, "A"), task(50.0, 50.0, "A")];
        tasks[0].progress = 0.5;
        let rows = rows(&tasks);
        let locale = Locale::default();
        let gantt = layout(&request(&tasks, &rows, &locale));
        assert_eq!(gantt.rows.len(), 3);
        let bar = gantt.bars[0];
        assert_eq!((bar.x, bar.width, bar.done), (20.0, 100.0, 50.0));
        assert_eq!(bar.row, 1);
        assert!(bar.y > 20.0 && bar.y + bar.height < 40.0, "inside its row");
        let milestone = gantt.bars[1];
        assert!(milestone.milestone);
        assert_eq!(milestone.x + milestone.width / 2.0, 100.0);
        assert_eq!(gantt.summaries.len(), 1, "the group's span on its heading");
        assert_eq!(gantt.summaries[0].width, 100.0);
    }

    #[test]
    fn only_rows_in_view_are_laid_out() {
        let tasks: Vec<Task> = (0..1000)
            .map(|i| task(i as f64, i as f64 + 5.0, ""))
            .collect();
        let rows = rows(&tasks);
        let locale = Locale::default();
        let mut req = request(&tasks, &rows, &locale);
        req.window = (0.0, 1005.0);
        req.scroll = 400.0;
        let gantt = layout(&req);
        let shown: Vec<_> = gantt.rows.iter().map(|r| r.row).collect();
        assert_eq!(shown, (20..25).collect::<Vec<_>>());
        assert_eq!(gantt.rows[0].y, 0.0);
        req.window = (500.0, 600.0);
        assert!(
            layout(&req).bars.is_empty(),
            "the rows in view end before the window"
        );
    }

    #[test]
    fn bars_are_cut_at_the_edges() {
        let tasks = [task(-1e30, 1e30, ""), task(f64::NAN, f64::INFINITY, "")];
        let rows = rows(&tasks);
        let locale = Locale::default();
        let gantt = layout(&request(&tasks, &rows, &locale));
        assert_eq!(gantt.bars.len(), 1, "a task without a time has no bar");
        let bar = gantt.bars[0];
        assert!(bar.x >= -20.0 && bar.x + bar.width <= 220.0);
    }

    #[test]
    fn odd_tasks_and_settings_give_finite_shapes() {
        let tasks = [
            task(30.0, 10.0, "G"),
            task(f64::NAN, 40.0, "G"),
            Task {
                progress: f64::NAN,
                ..task(0.0, f64::MAX, "")
            },
            task(f64::MIN, f64::MIN, ""),
        ];
        let rows = rows(&tasks);
        assert_eq!(tasks[0].span(), Some((10.0, 30.0)), "end before start");
        assert_eq!(tasks[1].span(), Some((40.0, 40.0)));
        let locale = Locale::default();
        for (row_height, scroll, width, progress) in [
            (20.0, 0.0, 200.0, 1.0),
            (f32::NAN, 0.0, 200.0, 1.0),
            (0.0, 0.0, 200.0, 1.0),
            (1e-30, f32::MAX, 200.0, 0.5),
            (20.0, f32::NAN, -5.0, f32::NAN),
            (20.0, -1e30, f32::INFINITY, 1.0),
        ] {
            let mut req = request(&tasks, &rows, &locale);
            req.window = window(&tasks, (0.0, 1.0));
            req.row_height = row_height;
            req.scroll = scroll;
            req.width = width;
            req.progress = progress;
            req.now = Some(f64::NAN);
            let gantt = layout(&req);
            for b in gantt.bars.iter().chain(&gantt.summaries) {
                for v in [b.x, b.y, b.width, b.height, b.done] {
                    assert!(v.is_finite(), "{b:?}");
                }
            }
            assert!(gantt.ticks.iter().all(|t| t.pos.is_finite()));
            assert_eq!(gantt.now, None);
        }
    }

    #[test]
    fn the_window_pads_and_zooms() {
        let tasks = [task(0.0, 100.0, "")];
        assert_eq!(extent(&tasks), (-2.0, 102.0));
        assert_eq!(window(&tasks, (0.25, 0.75)), (24.0, 76.0));
        assert_eq!(window(&tasks, (f32::NAN, 0.0)), (-2.0, 102.0));
        assert_eq!(
            extent(&[task(5.0, 5.0, "")]),
            (5.0 - DAY / 2.0, 5.0 + DAY / 2.0)
        );
        assert_eq!(extent(&[]), (0.0, DAY));
    }

    #[test]
    fn the_now_line_shows_in_view() {
        let tasks = [task(0.0, 100.0, "")];
        let rows = rows(&tasks);
        let locale = Locale::default();
        let mut req = request(&tasks, &rows, &locale);
        req.now = Some(25.0);
        assert_eq!(layout(&req).now, Some(50.0));
        req.now = Some(150.0);
        assert_eq!(layout(&req).now, None);
    }

    #[test]
    fn day_ticks_over_weeks() {
        let tasks = [task(0.0, 14.0 * DAY, "")];
        let rows = rows(&tasks);
        let locale = Locale::default();
        let mut req = request(&tasks, &rows, &locale);
        req.window = (0.0, 14.0 * DAY);
        req.width = 800.0;
        req.time.origin = crate::time::unix_seconds(2026, 10, 5, 0, 0, 0);
        let ticks = layout(&req).ticks;
        let labels: Vec<_> = ticks.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels[..3], ["Oct 5", "Oct 6", "Oct 7"]);
        assert_eq!(ticks.len(), 15);
        assert_eq!(ticks[1].pos, 800.0 / 14.0);
    }

    #[test]
    fn durations() {
        let en = Locale::default();
        assert_eq!(duration(14.0 * DAY, &en), "14 d");
        assert_eq!(duration(2.25 * DAY, &en), "2 d 6 h");
        assert_eq!(duration(DAY - 60.0, &en), "24 h");
        assert_eq!(duration(3.0 * 3600.0 + 20.0 * 60.0, &en), "3 h 20 min");
        assert_eq!(duration(45.0 * 60.0, &en), "45 min");
        assert_eq!(duration(12.0, &en), "12 s");
        assert_eq!(duration(-DAY, &en), "1 d");
        assert_eq!(duration(f64::NAN, &en), "");
        assert_eq!(duration(1500.0 * DAY, &en), "1,500 d");
    }
}
