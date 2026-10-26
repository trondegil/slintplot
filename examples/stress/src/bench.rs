//! Headless benchmark: renders every configuration with the software renderer
//! and prints how long the layout and a full frame take.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{Platform, WindowAdapter, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, PhysicalSize};

use crate::{POINT_COUNTS, Stress, setup};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 800;

struct Headless(Rc<MinimalSoftwareWindow>);

impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

/// Renders one frame and returns how long it took, including layout.
fn frame(window: &MinimalSoftwareWindow, pixels: &mut [PremultipliedRgbaColor]) -> Duration {
    slint::platform::update_timers_and_animations();
    slintplot::profile::take_totals();
    window.request_redraw();
    let started = Instant::now();
    window.draw_if_needed(|renderer| {
        renderer.render(pixels, WIDTH as usize);
    });
    started.elapsed()
}

fn save(pixels: &[PremultipliedRgbaColor], path: &Path) -> std::io::Result<()> {
    let bytes: Vec<u8> = pixels
        .iter()
        .flat_map(|p| [p.red, p.green, p.blue])
        .collect();
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, WIDTH, HEIGHT);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&bytes)?;
    Ok(())
}

pub fn run(png_dir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = &png_dir {
        std::fs::create_dir_all(dir)?;
    }
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    window.set_size(PhysicalSize::new(WIDTH, HEIGHT));
    slint::platform::set_platform(Box::new(Headless(window.clone())))?;
    let mut pixels = vec![PremultipliedRgbaColor::default(); (WIDTH * HEIGHT) as usize];

    let app = Stress::new()?;
    app.set_dataset_count(3);
    // The entry animation would make the first frame of every chart a flat line.
    app.set_animated(false);
    let data = setup(&app);
    app.show()?;

    println!(
        "{} datasets, {WIDTH}x{HEIGHT}, software renderer\n",
        app.get_dataset_count()
    );
    println!(
        "{:>10}  {:<22}  {:>10}  {:>10}  {:>6}  {:>10}  {:>12}  {:>6}",
        "points", "mode", "generate", "engine", "calls", "frame", "hover frame", "calls"
    );
    let modes: [(&str, i32, bool, bool, bool); 12] = [
        // label, chart type (see `Stress.chart-type`), show points, smooth,
        // decimate
        ("line, points, smooth", 0, true, true, false),
        ("line, no points", 0, false, false, false),
        ("line, decimated", 0, false, false, true),
        ("bars", 1, false, false, false),
        ("time series", 2, false, false, false),
        ("time series, decimated", 2, false, false, true),
        ("scatter", 3, true, false, false),
        ("waterfall", 4, false, false, false),
        ("dumbbell", 5, false, false, false),
        ("funnel", 6, false, false, false),
        ("sunburst", 7, false, false, false),
        ("gauge", 8, true, false, false),
    ];
    let mut zoom = Vec::new();
    for (index, &points) in POINT_COUNTS.iter().enumerate() {
        app.set_point_index(index as i32);
        for (label, chart_type, show_points, smooth, decimate) in modes {
            // Markers and bars are one element each, and undecimated lines
            // of a million points take seconds to draw in software: skip the
            // sizes that would take minutes, and say so.
            // The gauge shows one value, so the point count doesn't matter.
            if chart_type == 8 && points > 100 {
                continue;
            }
            let markers = (show_points && chart_type != 8) || chart_type == 1;
            let per_item = (4..=7).contains(&chart_type);
            if (markers && points > 100_000) || ((chart_type == 3 || per_item) && points > 10_000) {
                println!("{points:>10}  {label:<22}  {:>10}", "skipped");
                continue;
            }
            app.set_chart_type(chart_type);
            let started = Instant::now();
            data.borrow_mut().regenerate(&app);
            let generate = started.elapsed();
            app.set_show_points(show_points);
            app.set_smooth(smooth);
            app.invoke_restyle();
            app.set_decimate(decimate);
            let full = frame(&window, &mut pixels);
            let (calls, engine) = slintplot::profile::take_totals();
            window.dispatch_event(WindowEvent::PointerMoved {
                position: LogicalPosition::new(640.0, 400.0),
            });
            let hover = frame(&window, &mut pixels);
            if let Some(dir) = &png_dir {
                save(
                    &pixels,
                    &dir.join(format!("{points}-{}.png", label.replace([',', ' '], ""))),
                )?;
            }
            let (hover_calls, _) = slintplot::profile::take_totals();
            window.dispatch_event(WindowEvent::PointerExited);
            if decimate && chart_type == 0 {
                // Zoom into 0.1% of the data and measure a frame there.
                app.set_view_start(0.5);
                app.set_view_end(0.501);
                let zoomed = frame(&window, &mut pixels);
                let (_, engine) = slintplot::profile::take_totals();
                zoom.push((points, engine, zoomed));
                if let Some(dir) = &png_dir {
                    save(&pixels, &dir.join(format!("{points}-zoomed.png")))?;
                }
                app.set_view_start(0.0);
                app.set_view_end(1.0);
            }
            println!(
                "{points:>10}  {label:<22}  {:>10.1?}  {:>10.1?}  {:>6}  {:>10.1?}  {:>12.1?}  {:>6}",
                generate, engine, calls, full, hover, hover_calls
            );
        }
    }
    println!("\nZoomed to 0.1% of the data (line, decimation on):\n");
    println!("{:>10}  {:>10}  {:>10}", "points", "engine", "frame");
    for (points, engine, frame) in zoom {
        println!("{points:>10}  {engine:>10.1?}  {frame:>10.1?}");
    }
    Ok(())
}
