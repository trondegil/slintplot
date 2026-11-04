//! Benchmark for Slint's Skia renderer: renders the same configurations as
//! `--bench` through the GPU and prints, for each:
//!
//! - **frame**: from changing the data until the frame is drawn and its
//!   pixels are back in memory, so the chart's layout and the GPU's work
//!   are included;
//! - **redraw**: the median of drawing the same data again, as when
//!   hovering.
//!
//! Frames are rendered with `Window::take_snapshot`, which draws the window
//! with its renderer and reads the pixels back, and returns only when that's
//! done; that way the GPU's work is measured, not just the commands sent to
//! it. Reading the pixels back costs about the same in every frame: the
//! gauge's redraw, printed first, is that cost plus drawing the controls and
//! one gauge, the least a frame can take.
//!
//! `--bench-gpu <dir>` also saves each first frame as a PNG in `dir`, to
//! check what was drawn.
//!
//! The renderer is chosen as usual with `SLINT_BACKEND`, for example
//! `SLINT_BACKEND=winit-skia`; run with `SLINT_SCALE_FACTOR=1` for the same
//! number of pixels as `--bench`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use slint::{ComponentHandle, LogicalSize, Timer};

use crate::bench::{MODES, Skip, machine, skip};
use crate::{POINT_COUNTS, Stress, setup};

/// The window's size in logical pixels, as in `--bench`.
const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 800.0;
/// Redraws per configuration, unless they take longer than `REDRAW_TIME`.
const REDRAWS: usize = 15;
const REDRAW_TIME: Duration = Duration::from_secs(2);
/// The gauge's chart type (see `Stress.chart-type`).
const GAUGE: i32 = 8;

pub fn run(png_dir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = &png_dir {
        std::fs::create_dir_all(dir)?;
    }
    let app = Stress::new()?;
    app.set_dataset_count(3);
    // The entry animation would make the first frame of every chart a flat line.
    app.set_animated(false);
    let data = setup(&app);
    app.window().set_size(LogicalSize::new(WIDTH, HEIGHT));
    app.show()?;

    let weak = app.as_weak();
    let result = std::rc::Rc::new(std::cell::RefCell::new(Ok(())));
    let out = result.clone();
    // Once the window is up and the event loop runs.
    Timer::single_shot(Duration::from_millis(500), move || {
        if let Some(app) = weak.upgrade() {
            *out.borrow_mut() = sweep(&app, &data, png_dir.as_deref());
        }
        let _ = slint::quit_event_loop();
    });
    slint::run_event_loop()?;
    result.replace(Ok(()))
}

type Pixels = slint::SharedPixelBuffer<slint::Rgba8Pixel>;

/// Draws the window and reads it back; returns how long that took, and the
/// pixels.
fn snapshot(app: &Stress) -> Result<(Duration, Pixels), Box<dyn std::error::Error>> {
    // Animations are off, so there's nothing to advance between frames (and
    // the sweep runs inside a timer, where timers can't be updated).
    let started = Instant::now();
    let pixels = app.window().take_snapshot()?;
    Ok((started.elapsed(), pixels))
}

fn frame(app: &Stress) -> Result<Duration, Box<dyn std::error::Error>> {
    Ok(snapshot(app)?.0)
}

fn save(pixels: &Pixels, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, pixels.width(), pixels.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()?
        .write_image_data(pixels.as_bytes())?;
    Ok(())
}

/// The median of up to `REDRAWS` redraws of the same frame.
fn redraw(app: &Stress) -> Result<Duration, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let mut times = Vec::new();
    while times.len() < REDRAWS && (times.len() < 3 || started.elapsed() < REDRAW_TIME) {
        times.push(frame(app)?);
    }
    times.sort();
    Ok(times[times.len() / 2])
}

fn sweep(
    app: &Stress,
    data: &std::cell::RefCell<crate::Data>,
    png_dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = app.window().size();
    println!(
        "Skia/GPU benchmark\nMachine:  {}\nRenderer: SLINT_BACKEND={}\nWindow:   {}x{} pixels, scale factor {}\n",
        machine(),
        std::env::var("SLINT_BACKEND").unwrap_or_else(|_| "(default)".into()),
        size.width,
        size.height,
        app.window().scale_factor(),
    );

    // Warm up (shaders, caches) on the gauge, and print its redraw: the
    // least a frame takes, read-back included.
    app.set_bench_count(100);
    app.set_chart_type(GAUGE);
    data.borrow_mut().regenerate(app);
    for _ in 0..5 {
        frame(app)?;
    }
    println!("least frame (gauge, read back): {:.1?}\n", redraw(app)?);

    println!(
        "{:>10}  {:<22}  {:>10}  {:>10}  {:>6}",
        "points", "mode", "frame", "redraw", "calls"
    );
    for &points in &POINT_COUNTS {
        app.set_bench_count(points as i32);
        for (label, chart_type, show_points, smooth, decimate) in MODES {
            match skip(points, chart_type, show_points) {
                Some(Skip::Silently) => continue,
                Some(Skip::Say) => {
                    println!("{points:>10}  {label:<22}  {:>10}", "skipped");
                    continue;
                }
                None => {}
            }
            app.set_chart_type(chart_type);
            data.borrow_mut().regenerate(app);
            app.set_show_points(show_points);
            app.set_smooth(smooth);
            app.invoke_restyle();
            app.set_decimate(decimate);
            slintplot::profile::take_totals();
            let (first, pixels) = snapshot(app)?;
            let (calls, _) = slintplot::profile::take_totals();
            if let Some(dir) = png_dir {
                save(
                    &pixels,
                    &dir.join(format!("{points}-{}.png", label.replace([',', ' '], ""))),
                )?;
            }
            println!(
                "{points:>10}  {label:<22}  {:>10.1?}  {:>10.1?}  {calls:>6}",
                first,
                redraw(app)?
            );
        }
    }
    Ok(())
}
