//! The website's animation of the gallery: the mouse hovering, clicking and
//! scrolling through a few samples, rendered headlessly to PNG frames at 30
//! frames per second (`cargo run -p slintplot-gallery -- --video <dir>`).
//! `docs/make-video.sh` encodes them.

use std::path::Path;
use std::time::Duration;

use slint::LogicalPosition;
use slint::platform::WindowEvent;
use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor};

use crate::snapshot;

pub const FPS: u32 = 30;

/// One step of the animation.
pub enum Step {
    /// Holds still for a number of seconds.
    Wait(f32),
    /// Glides the mouse to a point in a number of seconds.
    Move(f32, f32, f32),
    /// Clicks where the mouse is.
    Click,
    /// Turns the mouse wheel where the mouse is, by a number of pixels over a
    /// number of seconds; negative scrolls down.
    Scroll(f32, f32),
}

/// Records `steps`, starting with the mouse at `start`, into `dir` as
/// numbered PNG files. The last `fade` seconds blend into the first frame,
/// so the animation loops without a jump.
pub fn record(
    window: &MinimalSoftwareWindow,
    dir: &Path,
    start: (f32, f32),
    steps: &[Step],
    fade: f32,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut frames = Frames {
        window,
        dir,
        mouse: start,
        count: 0,
        first: Vec::new(),
    };
    frames.move_mouse(start);
    for step in steps {
        match *step {
            Step::Wait(seconds) => {
                for _ in 0..frame_count(seconds) {
                    frames.frame(None)?;
                }
            }
            Step::Move(x, y, seconds) => {
                let from = frames.mouse;
                let n = frame_count(seconds);
                for i in 1..=n {
                    let t = ease(i as f32 / n as f32);
                    frames.move_mouse((from.0 + (x - from.0) * t, from.1 + (y - from.1) * t));
                    frames.frame(None)?;
                }
            }
            Step::Click => {
                let (x, y) = frames.mouse;
                snapshot::click(window, x, y);
                frames.frame(None)?;
            }
            Step::Scroll(pixels, seconds) => {
                let n = frame_count(seconds);
                let (x, y) = frames.mouse;
                let mut done = 0.0;
                for i in 1..=n {
                    let to = pixels * ease(i as f32 / n as f32);
                    window.dispatch_event(WindowEvent::PointerScrolled {
                        position: LogicalPosition::new(x, y),
                        delta_x: 0.0,
                        delta_y: to - done,
                    });
                    done = to;
                    frames.frame(None)?;
                }
            }
        }
    }
    let n = frame_count(fade);
    for i in 1..=n {
        frames.frame(Some(i as f32 / n as f32))?;
    }
    Ok(())
}

fn frame_count(seconds: f32) -> usize {
    (seconds * FPS as f32).round() as usize
}

/// Slow at both ends, like a hand moving a mouse.
fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

struct Frames<'a> {
    window: &'a MinimalSoftwareWindow,
    dir: &'a Path,
    mouse: (f32, f32),
    count: usize,
    /// The first frame, which the last ones blend into.
    first: Vec<[u8; 3]>,
}

impl Frames<'_> {
    fn move_mouse(&mut self, to: (f32, f32)) {
        self.mouse = to;
        self.window.dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(to.0, to.1),
        });
    }

    /// Moves time on by a frame and saves it, blended `fade` of the way into
    /// the first frame.
    fn frame(&mut self, fade: Option<f32>) -> std::io::Result<()> {
        if self.count > 0 {
            snapshot::settle(Duration::from_secs(1) / FPS);
        }
        let pixels = snapshot::render(self.window);
        let (width, height) = snapshot::size();
        let mut rgb: Vec<[u8; 3]> = pixels.iter().map(|p| rgb(*p)).collect();
        draw_pointer(&mut rgb, width as usize, height as usize, self.mouse);
        if self.first.is_empty() {
            self.first = rgb.clone();
        }
        if let Some(t) = fade {
            for (pixel, first) in rgb.iter_mut().zip(&self.first) {
                for c in 0..3 {
                    pixel[c] =
                        (pixel[c] as f32 + (first[c] as f32 - pixel[c] as f32) * t).round() as u8;
                }
            }
        }
        self.count += 1;
        let path = self.dir.join(format!("{:04}.png", self.count));
        let mut encoder = png::Encoder::new(
            std::io::BufWriter::new(std::fs::File::create(path)?),
            width,
            height,
        );
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        encoder
            .write_header()?
            .write_image_data(rgb.as_flattened())?;
        Ok(())
    }
}

fn rgb(p: PremultipliedRgbaColor) -> [u8; 3] {
    [p.red, p.green, p.blue]
}

/// The outline of an arrow pointer with its tip at (0, 0), in pixels.
const POINTER: [(f32, f32); 7] = [
    (0.0, 0.0),
    (0.0, 17.0),
    (4.2, 13.2),
    (7.2, 19.6),
    (9.8, 18.4),
    (6.9, 12.2),
    (12.4, 12.2),
];

/// Draws a white arrow pointer with a dark edge, its tip at `at`.
fn draw_pointer(rgb: &mut [[u8; 3]], width: usize, height: usize, at: (f32, f32)) {
    let edge = |x: f32, y: f32| -> (bool, f32) {
        let mut inside = false;
        let mut distance = f32::MAX;
        for i in 0..POINTER.len() {
            let a = POINTER[i];
            let b = POINTER[(i + 1) % POINTER.len()];
            if (a.1 > y) != (b.1 > y) && x < a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0) {
                inside = !inside;
            }
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let t = (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            distance = distance.min((x - a.0 - t * dx).hypot(y - a.1 - t * dy));
        }
        (inside, distance)
    };
    let (left, top) = (at.0.floor() as i32 - 2, at.1.floor() as i32 - 2);
    for py in top..top + 25 {
        for px in left..left + 17 {
            if px < 0 || py < 0 || px as usize >= width || py as usize >= height {
                continue;
            }
            // The pixel's center, relative to the tip.
            let (x, y) = (px as f32 + 0.5 - at.0, py as f32 + 0.5 - at.1);
            let (inside, distance) = edge(x, y);
            // The dark edge is about a pixel wide, inside and out.
            let dark = if inside {
                (1.6 - distance).clamp(0.0, 1.0)
            } else {
                (1.0 - distance).clamp(0.0, 1.0)
            };
            let white = if inside { 1.0 - dark } else { 0.0 };
            let pixel = &mut rgb[py as usize * width + px as usize];
            for channel in pixel.iter_mut() {
                let v = *channel as f32;
                let v = v + (255.0 - v) * white;
                *channel = (v * (1.0 - dark * 0.85)).round() as u8;
            }
        }
    }
}
