//! Stress test for SlintPlot: line, bar, time series and scatter charts with
//! up to 8 datasets × 1,000,000 points, waterfall, dumbbell, funnel and
//! sunburst charts with as many steps, rows, stages or items, a gauge, and
//! optional 60 Hz streaming.
//! `cargo run --release -p stress`.
//!
//! `--bench [dir]` renders headlessly with the software renderer and prints
//! timings, optionally saving every rendered configuration as a PNG in `dir`.
//! `--bench-stream` runs only its streaming part.

mod bench;
mod dock_icon;
mod stream_model;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use stream_model::StreamModel;

slint::include_modules!();

pub const POINT_COUNTS: [usize; 5] = [100, 1_000, 10_000, 100_000, 1_000_000];

/// Small xorshift generator, so the example needs no extra dependencies.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Seconds between the points of a time series: one point per minute.
const MINUTE: f32 = 60.0;

/// Random walks, one per dataset, that can be extended for streaming.
struct Data {
    rng: Rng,
    /// Line and bar charts: the values of each dataset.
    series: Vec<Rc<StreamModel<f32>>>,
    /// Time series and scatter charts: the points of each dataset.
    points: Vec<Rc<StreamModel<ChartXY>>>,
    /// Line, bar and dumbbell charts: the category labels, which move on
    /// with the values when streaming.
    labels: Rc<StreamModel<SharedString>>,
    /// The number of the next label.
    next_label: usize,
    last: Vec<f32>,
    /// Time series: the time of the newest point.
    last_time: f32,
    /// The chart type the data was made for (see `Stress.chart-type`).
    chart_type: i32,
    /// Waterfall steps, funnel stages and sunburst items.
    steps: Rc<VecModel<ChartWaterfallStep>>,
    stages: Rc<VecModel<ChartSlice>>,
    sun: Rc<VecModel<ChartSunburstItem>>,
    gauge: f32,
    /// Streaming ticks, to pick the funnel stage or sunburst item to change.
    tick: usize,
}

impl Data {
    fn step(&mut self, last: f32, dataset: usize) -> f32 {
        let center = 100.0 * dataset as f32;
        // Mean-reverting walk, so the series stay apart and on screen.
        last + (self.rng.next() - 0.5) * 8.0 + (center - last) * 0.002
    }

    fn regenerate(&mut self, app: &Stress) {
        let count = POINT_COUNTS[app.get_point_index().clamp(0, 4) as usize];
        let datasets = app.get_dataset_count().clamp(1, 8) as usize;
        let tension = if app.get_smooth() { 0.4 } else { 0.0 };
        let chart_type = app.get_chart_type();
        self.chart_type = chart_type;
        self.series.clear();
        self.points.clear();
        self.last.clear();
        self.last_time = (count.max(1) - 1) as f32 * MINUTE;
        let mut rows = Vec::new();
        for d in 0..datasets {
            let mut v = 100.0 * d as f32;
            let values: Vec<f32> = (0..count)
                .map(|_| {
                    v = self.step(v, d);
                    v
                })
                .collect();
            self.last.push(v);
            let mut row = ChartDataset {
                label: format!("Series {}", d + 1).into(),
                tension,
                ..Default::default()
            };
            match chart_type {
                // Time series: one point per minute.
                2 => {
                    let points: Vec<ChartXY> = values
                        .iter()
                        .enumerate()
                        .map(|(i, &y)| ChartXY {
                            x: i as f32 * MINUTE,
                            y,
                            r: 0.0,
                        })
                        .collect();
                    let model = Rc::new(StreamModel::new(count, points));
                    self.points.push(model.clone());
                    row.points = model.into();
                }
                // Scatter: a cloud around each dataset's walk.
                3 => {
                    let points: Vec<ChartXY> = values
                        .iter()
                        .map(|&y| ChartXY {
                            x: (self.rng.next() - 0.5) * 200.0 + 60.0 * d as f32,
                            y: y + (self.rng.next() - 0.5) * 60.0,
                            r: 0.0,
                        })
                        .collect();
                    let model = Rc::new(StreamModel::new(count, points));
                    self.points.push(model.clone());
                    row.points = model.into();
                }
                _ => {
                    let model = Rc::new(StreamModel::new(count, values));
                    self.series.push(model.clone());
                    row.values = model.into();
                }
            }
            rows.push(row);
        }
        let labels = (0..count).map(|i| SharedString::from(i.to_string()));
        self.labels = Rc::new(StreamModel::new(count, labels));
        self.next_label = count;
        app.set_labels(self.labels.clone().into());
        // The chart types that don't take datasets.
        let flat = matches!(chart_type, 4 | 6 | 7);
        app.set_datasets(ModelRc::new(VecModel::from(if flat {
            Vec::new()
        } else {
            rows
        })));
        self.steps.set_vec(if chart_type == 4 {
            // Ups and downs, with a total every 10 steps.
            (0..count)
                .map(|i| ChartWaterfallStep {
                    label: i.to_string().into(),
                    value: ((self.rng.next() - 0.45) * 100.0).round(),
                    total: i % 10 == 9,
                })
                .collect()
        } else {
            Vec::new()
        });
        self.stages.set_vec(if chart_type == 6 {
            let mut value = 1_000_000.0;
            (0..count)
                .map(|i| {
                    value *= 0.5 + 0.5 * self.rng.next();
                    ChartSlice {
                        label: format!("Stage {}", i + 1).into(),
                        value: value.round().max(1.0),
                        ..Default::default()
                    }
                })
                .collect()
        } else {
            Vec::new()
        });
        self.sun.set_vec(if chart_type == 7 {
            // One top-level item per dataset, and the leaves spread over them.
            let tops = (0..datasets).map(|d| ChartSunburstItem {
                label: format!("Group {}", d + 1).into(),
                parent: -1,
                ..Default::default()
            });
            let leaves = (0..count).map(|i| ChartSunburstItem {
                label: format!("Item {}", i + 1).into(),
                parent: (i % datasets) as i32,
                value: (self.rng.next() * 100.0).round() + 1.0,
                ..Default::default()
            });
            tops.collect::<Vec<_>>()
                .into_iter()
                .chain(leaves.collect::<Vec<_>>())
                .collect()
        } else {
            Vec::new()
        });
        self.gauge = 50.0;
        app.set_gauge_value(self.gauge);
    }

    /// Appends `samples` values to every dataset, dropping as many of the
    /// oldest. Time series move on by a minute per sample; scatter charts
    /// replace points. Waterfalls move on by a step; funnels and sunbursts
    /// change one value, and the gauge moves.
    fn stream(&mut self, app: &Stress, samples: usize) {
        self.tick += 1;
        if self.steps.row_count() > 0 {
            self.steps.remove(0);
            self.steps.push(ChartWaterfallStep {
                label: self.tick.to_string().into(),
                value: ((self.rng.next() - 0.45) * 100.0).round(),
                total: false,
            });
        }
        let stages = self.stages.row_count();
        if stages > 0 {
            let row = self.tick % stages;
            if let Some(mut stage) = self.stages.row_data(row) {
                stage.value = (stage.value * (0.9 + 0.2 * self.rng.next()))
                    .round()
                    .max(1.0);
                self.stages.set_row_data(row, stage);
            }
        }
        let items = self.sun.row_count();
        if items > 0 {
            let row = self.tick % items;
            if let Some(mut item) = self.sun.row_data(row).filter(|i| i.parent >= 0) {
                item.value = (self.rng.next() * 100.0).round() + 1.0;
                self.sun.set_row_data(row, item);
            }
        }
        self.gauge = (self.gauge + (self.rng.next() - 0.5) * 6.0).clamp(0.0, 100.0);
        app.set_gauge_value(self.gauge);
        for d in 0..self.series.len() {
            let values: Vec<f32> = (0..samples)
                .map(|_| {
                    self.last[d] = self.step(self.last[d], d);
                    self.last[d]
                })
                .collect();
            self.series[d].extend(values);
        }
        if !self.series.is_empty() {
            let first = self.next_label;
            self.next_label += samples;
            self.labels
                .extend((first..self.next_label).map(|i| SharedString::from(i.to_string())));
        }
        let first_time = self.last_time;
        self.last_time += MINUTE * samples as f32;
        for d in 0..self.points.len() {
            let points: Vec<ChartXY> = (1..=samples)
                .map(|i| {
                    let y = self.step(self.last[d], d);
                    self.last[d] = y;
                    if self.chart_type == 2 {
                        ChartXY {
                            x: first_time + MINUTE * i as f32,
                            y,
                            r: 0.0,
                        }
                    } else {
                        ChartXY {
                            x: (self.rng.next() - 0.5) * 200.0 + 60.0 * d as f32,
                            y: y + (self.rng.next() - 0.5) * 60.0,
                            r: 0.0,
                        }
                    }
                })
                .collect();
            self.points[d].extend(points);
        }
    }
}

fn restyle(app: &Stress) {
    let tension = if app.get_smooth() { 0.4 } else { 0.0 };
    let datasets = app.get_datasets();
    for row in 0..datasets.row_count() {
        if let Some(mut ds) = datasets.row_data(row) {
            ds.tension = tension;
            datasets.set_row_data(row, ds);
        }
    }
}

fn setup(app: &Stress) -> Rc<RefCell<Data>> {
    slintplot::register!(app);
    let data = Rc::new(RefCell::new(Data {
        rng: Rng(0x9e37_79b9_7f4a_7c15),
        series: Vec::new(),
        points: Vec::new(),
        last: Vec::new(),
        labels: Rc::new(StreamModel::new(0, [])),
        next_label: 0,
        last_time: 0.0,
        chart_type: 0,
        steps: Rc::new(VecModel::default()),
        stages: Rc::new(VecModel::default()),
        sun: Rc::new(VecModel::default()),
        gauge: 50.0,
        tick: 0,
    }));
    {
        let d = data.borrow();
        app.set_steps(d.steps.clone().into());
        app.set_stages(d.stages.clone().into());
        app.set_sun(d.sun.clone().into());
    }
    data.borrow_mut().regenerate(app);

    let weak = app.as_weak();
    let d = data.clone();
    app.on_regenerate(move || {
        if let Some(app) = weak.upgrade() {
            let started = Instant::now();
            d.borrow_mut().regenerate(&app);
            eprintln!("generated data in {:.0?}", started.elapsed());
        }
    });
    let weak = app.as_weak();
    app.on_restyle(move || {
        if let Some(app) = weak.upgrade() {
            restyle(&app);
        }
    });
    data
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("--bench"), dir) => return bench::run(dir.map(Into::into)),
        (Some("--bench-stream"), None) => return bench::run_stream(),
        (Some("--snapshot"), Some(dir)) => return bench::snapshot(dir.as_ref()),
        (None, _) => {}
        _ => {
            return Err(
                "usage: slintplot-stress [--bench [<dir>] | --bench-stream | --snapshot <dir>]"
                    .into(),
            );
        }
    }

    let app = Stress::new()?;
    let data = setup(&app);
    // Once the app has finished launching, which would reset it.
    Timer::single_shot(Duration::ZERO, dock_icon::set);

    // Streaming: samples arrive at the chosen rate, and a timer adds those
    // that are due, at most 60 times a second: drawing more often than the
    // screen refreshes is wasted. When drawing is slow, ticks come later and
    // carry more samples, so the data rate holds and the frame rate drops.
    let samples = Rc::new(Cell::new(0usize));
    let stream_timer = Timer::default();
    {
        let weak = app.as_weak();
        let samples = samples.clone();
        let mut last = Instant::now();
        let mut owed = 0.0;
        stream_timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
            let now = Instant::now();
            let elapsed = now.duration_since(last).as_secs_f64();
            last = now;
            let Some(app) = weak.upgrade().filter(|app| app.get_streaming()) else {
                owed = 0.0;
                return;
            };
            owed += app.get_stream_rate() as f64 * elapsed;
            let due = owed.floor();
            owed -= due;
            // More than a whole dataset at once replaces all of it.
            let count = POINT_COUNTS[app.get_point_index().clamp(0, 4) as usize];
            let due = (due as usize).min(count);
            if due > 0 {
                data.borrow_mut().stream(&app, due);
                samples.set(samples.get() + due);
            }
        });
    }

    // Count rendered frames, when the renderer supports notifications.
    let frames = Rc::new(Cell::new(0u32));
    {
        let frames = frames.clone();
        let _ = app.window().set_rendering_notifier(move |state, _| {
            if let slint::RenderingState::AfterRendering = state {
                frames.set(frames.get() + 1);
            }
        });
    }

    let stats_timer = Timer::default();
    {
        let weak = app.as_weak();
        let mut since = Instant::now();
        stats_timer.start(TimerMode::Repeated, Duration::from_millis(500), move || {
            let Some(app) = weak.upgrade() else { return };
            let elapsed = since.elapsed().as_secs_f32();
            since = Instant::now();
            let points = POINT_COUNTS[app.get_point_index().clamp(0, 4) as usize];
            let total = points * app.get_dataset_count().max(1) as usize;
            let rate = samples.replace(0) as f64 / elapsed as f64;
            let streaming = if app.get_streaming() {
                format!(
                    "{} samples/s per dataset",
                    slintplot::ticks::format_value(rate.round())
                )
            } else {
                "not streaming".into()
            };
            app.set_stats(
                format!(
                    "{:>9} points   layout {:>7.1} ms   {:>4.0} frames/s   {streaming}",
                    slintplot::ticks::format_value(total as f64),
                    slintplot::profile::last_layout_time().as_secs_f64() * 1000.0,
                    frames.replace(0) as f32 / elapsed,
                )
                .into(),
            );
        });
    }

    app.run()?;
    Ok(())
}
