//! Feeds every chart type random and malformed data, renders it and moves
//! the mouse and keyboard over it, to check that nothing panics:
//! `cargo run --release -p slintplot-fuzz -- [rounds] [seed]`.
//!
//! `-- soak [minutes] [seed]` instead repeats the same rounds for that long
//! (10 minutes by default) and checks that memory in use levels off.

use std::cell::Cell;
use std::rc::Rc;

use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Key, Platform, PointerEventButton, WindowAdapter, WindowEvent};
use slint::{Color, LogicalPosition, ModelRc, PhysicalSize, SharedString, VecModel};

slint::include_modules!();

const CHARTS: i32 = 18;

/// More memory than any chart should need: past it the fuzzer stops, rather
/// than fill the machine's RAM and swap.
const MEMORY_LIMIT: usize = 2 << 30;

// Which round and chart is running, to reproduce a panic or a runaway
// allocation.
thread_local!(static AT: Cell<(u64, u64, i32)> = const { Cell::new((0, 0, 0)) });

/// When the current chart started, in milliseconds since the run did, and
/// which one it is, for the watchdog.
static STARTED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static CURRENT: std::sync::Mutex<(u64, u64, i32)> = std::sync::Mutex::new((0, 0, 0));

/// Stops the run when one chart takes longer than `seconds`: a hang, such as
/// a loop that never ends without allocating, which the memory limit misses.
fn watchdog(seconds: u64) {
    let epoch = std::time::Instant::now();
    STARTED.store(0, std::sync::atomic::Ordering::Relaxed);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let now = epoch.elapsed().as_millis() as u64;
            let started = STARTED.load(std::sync::atomic::Ordering::Relaxed);
            if now.saturating_sub(started) > seconds * 1000 {
                let (seed, round, chart) = *CURRENT.lock().unwrap();
                eprintln!("HANG: over {seconds} s in round {round}, chart {chart}, seed {seed}");
                std::process::exit(2);
            }
        }
    });
    WATCH_EPOCH.set(epoch).ok();
}

static WATCH_EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// The system allocator, refusing to go past `MEMORY_LIMIT` in all.
struct Capped(std::sync::atomic::AtomicUsize);

impl Capped {
    fn reserve(&self, size: usize) -> bool {
        use std::sync::atomic::Ordering::Relaxed;
        let used = self.0.fetch_add(size, Relaxed) + size;
        if used <= MEMORY_LIMIT {
            return true;
        }
        self.0.fetch_sub(size, Relaxed);
        // Formatted on the stack: allocating here would recurse.
        use std::io::Write;
        let mut buf = [0u8; 160];
        let mut out = &mut buf[..];
        let (seed, round, chart) = AT.with(Cell::get);
        let _ = writeln!(
            out,
            "OUT OF MEMORY: {size} more bytes in round {round}, chart {chart}, seed {seed}"
        );
        let len = 160 - out.len();
        let _ = std::io::stderr().write_all(&buf[..len]);
        false
    }
}

unsafe impl std::alloc::GlobalAlloc for Capped {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        if !self.reserve(layout.size()) {
            return std::ptr::null_mut();
        }
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        self.0
            .fetch_sub(layout.size(), std::sync::atomic::Ordering::Relaxed);
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        let grow = size.saturating_sub(layout.size());
        if !self.reserve(grow) {
            return std::ptr::null_mut();
        }
        let new = unsafe { std::alloc::System.realloc(ptr, layout, size) };
        // Give back the shrink, or the reservation if nothing moved.
        let back = if new.is_null() {
            grow
        } else {
            layout.size().saturating_sub(size)
        };
        self.0.fetch_sub(back, std::sync::atomic::Ordering::Relaxed);
        new
    }
}

#[global_allocator]
static ALLOCATOR: Capped = Capped(std::sync::atomic::AtomicUsize::new(0));

struct Headless(Rc<MinimalSoftwareWindow>);

impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

/// xorshift: the same data for the same seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
    fn unit(&mut self) -> f32 {
        (self.next() % 10_000) as f32 / 10_000.0
    }
    /// Mostly ordinary values, often the awkward ones.
    fn number(&mut self) -> f32 {
        match self.below(20) {
            0 => f32::NAN,
            1 => f32::INFINITY,
            2 => f32::NEG_INFINITY,
            3 => 0.0,
            4 => -0.0,
            5 => f32::MAX,
            6 => f32::MIN,
            7 => f32::MIN_POSITIVE,
            8 => 1e-30,
            9 => -1e30,
            10 => -(self.unit() * 100.0),
            _ => self.unit() * 100.0,
        }
    }
    fn int(&mut self) -> i32 {
        match self.below(8) {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => -1,
            3 => 0,
            _ => self.below(40) as i32 - 10,
        }
    }
    /// Usually short, sometimes empty, sometimes long enough to decimate.
    fn len(&mut self) -> usize {
        match self.below(12) {
            0 => 0,
            1 => 1,
            2 => 5_000 + self.below(20_000) as usize,
            _ => self.below(30) as usize,
        }
    }
    fn text(&mut self) -> SharedString {
        match self.below(6) {
            0 => "".into(),
            1 => "A very long label that will not fit anywhere at all".into(),
            2 => "Ünïcødé 中文 ✓".into(),
            _ => format!("L{}", self.below(1000)).into(),
        }
    }
    fn color(&mut self) -> Color {
        if self.chance(70) {
            Color::from_argb_u8(0, 0, 0, 0)
        } else {
            Color::from_argb_u8(self.below(256) as u8, 200, 30, 90)
        }
    }
}

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}

/// Without decimation every point is drawn: a few thousand at most, as an
/// app would, or a dense filled line takes the software renderer minutes.
fn datasets(rng: &mut Rng, decimate: bool) -> Vec<ChartDataset> {
    let most = if decimate { usize::MAX } else { 2_000 };
    (0..rng.below(5))
        .map(|_| {
            let n = rng.len().min(most);
            let sorted = rng.chance(60);
            let mut x = 0.0;
            let points: Vec<ChartXY> = (0..n)
                .map(|_| {
                    x = if sorted {
                        x + rng.unit() * 10.0
                    } else {
                        rng.number()
                    };
                    ChartXY {
                        x,
                        y: rng.number(),
                        r: rng.number(),
                    }
                })
                .collect();
            ChartDataset {
                label: rng.text(),
                values: model((0..rng.len().min(most)).map(|_| rng.number()).collect()),
                points: model(points),
                color: rng.color(),
                hidden: rng.chance(15),
                fill: rng.chance(40),
                tension: rng.number(),
                stepped: rng.chance(20),
                series_type: match rng.below(3) {
                    0 => ChartSeriesType::Line,
                    1 => ChartSeriesType::Bar,
                    _ => ChartSeriesType::Auto,
                },
                right_axis: rng.chance(30),
            }
        })
        .collect()
}

fn fill(ui: &Fuzz, rng: &mut Rng) {
    let decimate = rng.chance(70);
    ui.set_decimate(decimate);
    let labels: Vec<SharedString> = (0..rng.len().min(200)).map(|_| rng.text()).collect();
    ui.set_labels(model(labels));
    ui.set_row_labels(model((0..rng.below(12)).map(|_| rng.text()).collect()));
    ui.set_datasets(model(datasets(rng, decimate)));
    let slices: Vec<ChartSlice> = (0..rng.below(12))
        .map(|_| ChartSlice {
            label: rng.text(),
            value: rng.number(),
            color: rng.color(),
            hidden: rng.chance(15),
        })
        .collect();
    ui.set_slices(model(slices));
    ui.set_values(model(
        (0..rng.len().min(800)).map(|_| rng.number()).collect(),
    ));
    let count = rng.below(40) as i32;
    let sunburst: Vec<ChartSunburstItem> = (0..count)
        .map(|_| ChartSunburstItem {
            label: rng.text(),
            value: rng.number(),
            parent: if rng.chance(70) {
                rng.below(count as u64) as i32 - 1
            } else {
                rng.int()
            },
            color: rng.color(),
            hidden: rng.chance(10),
        })
        .collect();
    ui.set_sunburst_items(model(sunburst));
    let treemap: Vec<ChartTreemapItem> = (0..rng.below(40))
        .map(|_| ChartTreemapItem {
            label: rng.text(),
            value: rng.number(),
            group: if rng.chance(50) {
                "".into()
            } else {
                format!("G{}", rng.below(4)).into()
            },
            color: rng.color(),
        })
        .collect();
    ui.set_treemap_items(model(treemap));
    let steps: Vec<ChartWaterfallStep> = (0..rng.below(30))
        .map(|_| ChartWaterfallStep {
            label: rng.text(),
            value: rng.number(),
            total: rng.chance(25),
        })
        .collect();
    ui.set_steps(model(steps));
    let bands: Vec<ChartGaugeBand> = (0..rng.below(6))
        .map(|_| ChartGaugeBand {
            to: rng.number(),
            label: rng.text(),
            color: rng.color(),
        })
        .collect();
    ui.set_bands(model(bands));
    ui.set_value(rng.number());

    ui.set_stacked(rng.chance(40));
    ui.set_stacked_percent(rng.chance(25));
    ui.set_logarithmic(rng.chance(25));
    ui.set_fixed_range(rng.chance(30));
    ui.set_begin_at_zero(rng.chance(50));
    ui.set_show_values(rng.chance(40));
    ui.set_flag(rng.chance(50));
    ui.set_min_value(rng.number());
    ui.set_max_value(rng.number());
    ui.set_decimals(rng.int());
    ui.set_number(if rng.chance(50) {
        rng.unit()
    } else {
        rng.number()
    });
    ui.set_view_start(if rng.chance(50) { 0.0 } else { rng.number() });
    ui.set_view_end(if rng.chance(50) { 1.0 } else { rng.number() });
    ui.set_zoom_item(rng.int());
    ui.set_utc_offset(rng.int());
    ui.set_year(rng.int());
    // Lengths such as cell-gap are the app's own settings, not data: kept to
    // values an app could set, including negative ones.
    ui.set_size(rng.unit() * 55.0 - 5.0);
    ui.set_legend_position(match rng.below(4) {
        0 => ChartLegendPosition::Top,
        1 => ChartLegendPosition::Bottom,
        2 => ChartLegendPosition::Left,
        _ => ChartLegendPosition::Right,
    });
}

fn render(window: &MinimalSoftwareWindow, size: (u32, u32)) {
    slint::platform::update_timers_and_animations();
    let (w, h) = size;
    let mut buffer = vec![
        slint::platform::software_renderer::PremultipliedRgbaColor::default();
        (w * h) as usize
    ];
    window.draw_if_needed(|renderer| {
        renderer.render(&mut buffer, w as usize);
    });
}

/// One round: every chart, filled from `seed` and `round`, rendered and
/// poked with the mouse and keyboard.
fn round(ui: &Fuzz, window: &MinimalSoftwareWindow, seed: u64, round: u64) {
    let round_seed = seed.wrapping_add(round).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    for chart in 0..CHARTS {
        AT.set((seed, round, chart));
        *CURRENT.lock().unwrap() = (seed, round, chart);
        if let Some(epoch) = WATCH_EPOCH.get() {
            STARTED.store(
                epoch.elapsed().as_millis() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        let mut rng = Rng(round_seed ^ ((chart as u64 + 1) * 0x1234_5679));
        ui.set_chart(chart);
        fill(ui, &mut rng);
        let size = match rng.below(6) {
            0 => (1, 1),
            1 => (3, 2),
            _ => (50 + rng.below(1000) as u32, 40 + rng.below(700) as u32),
        };
        window.set_size(PhysicalSize::new(size.0, size.1));
        render(window, size);
        for _ in 0..6 {
            let position = LogicalPosition::new(
                rng.unit() * size.0 as f32 * 1.2 - 5.0,
                rng.unit() * size.1 as f32 * 1.2 - 5.0,
            );
            let button = PointerEventButton::Left;
            window.dispatch_event(WindowEvent::PointerMoved { position });
            match rng.below(5) {
                0 => {
                    window.dispatch_event(WindowEvent::PointerPressed { position, button });
                    window.dispatch_event(WindowEvent::PointerReleased { position, button });
                }
                1 => window.dispatch_event(WindowEvent::PointerScrolled {
                    position,
                    delta_x: 0.0,
                    delta_y: rng.unit() * 400.0 - 200.0,
                }),
                2 => {
                    let key: SharedString = match rng.below(5) {
                        0 => Key::LeftArrow.into(),
                        1 => Key::RightArrow.into(),
                        2 => Key::Home.into(),
                        3 => Key::End.into(),
                        _ => Key::Return.into(),
                    };
                    window.dispatch_event(WindowEvent::KeyPressed { text: key.clone() });
                    window.dispatch_event(WindowEvent::KeyReleased { text: key });
                }
                _ => {}
            }
            render(window, size);
        }
        window.dispatch_event(WindowEvent::PointerExited);
    }
}

/// The bytes allocated and not yet freed.
fn live() -> usize {
    ALLOCATOR.0.load(std::sync::atomic::Ordering::Relaxed)
}

/// The same few rounds over and over for `minutes`: with the same data
/// coming back, memory in use must level off. Fails if it keeps growing.
fn soak(ui: &Fuzz, window: &MinimalSoftwareWindow, minutes: f64, seed: u64) -> bool {
    const ROUNDS: u64 = 16;
    let started = std::time::Instant::now();
    let mut samples = Vec::new();
    let mut pass = 0;
    while pass < 3 || started.elapsed().as_secs_f64() < minutes * 60.0 {
        for r in 0..ROUNDS {
            round(ui, window, seed, r);
        }
        pass += 1;
        samples.push(live());
        println!(
            "pass {pass:>4}, {:>6.1} min: {:>8} KB in use",
            started.elapsed().as_secs_f64() / 60.0,
            live() / 1024
        );
    }
    // The first passes fill caches (glyphs, paths); after them, no growth.
    let settled = samples[2];
    let last = *samples.last().unwrap();
    let growth = last.saturating_sub(settled);
    println!(
        "{pass} passes of {ROUNDS} rounds: {} KB in use after pass 3, {} KB at the end",
        settled / 1024,
        last / 1024
    );
    // A leak of a few bytes per chart and render adds up to far more.
    growth < 256 * 1024
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1).peekable();
    let soak_minutes = if args.peek().is_some_and(|a| a == "soak") {
        args.next();
        Some(args.next().map_or(Ok(10.0), |a| a.parse::<f64>())?)
    } else {
        None
    };
    let rounds: u64 = if soak_minutes.is_some() {
        0
    } else {
        args.next().map_or(Ok(200), |a| a.parse())?
    };
    let seed: u64 = args.next().map_or(Ok(0x5eed_1234), |a| a.parse())?;

    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
    let ui = Fuzz::new()?;
    slintplot::register!(ui);
    ui.show()?;

    watchdog(30);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let (seed, round, chart) = AT.get();
        eprintln!("PANIC in round {round}, chart {chart}, seed {seed}");
        previous(info);
    }));

    if let Some(minutes) = soak_minutes {
        if soak(&ui, &window, minutes, seed) {
            println!("memory stayed level");
            return Ok(());
        }
        eprintln!("MEMORY GREW: a leak");
        std::process::exit(1);
    }
    for r in 0..rounds {
        round(&ui, &window, seed, r);
        if (r + 1) % 25 == 0 {
            println!("{} rounds, {} charts each: no panics", r + 1, CHARTS);
        }
    }
    println!("done: {rounds} rounds of {CHARTS} charts, seed {seed}");
    Ok(())
}
