//! Headless rendering of the gallery to PNG files, for visual checks and docs:
//! `cargo run -p slintplot-gallery -- --snapshot <dir>`.

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{Platform, WindowAdapter, WindowEvent};
use slint::{LogicalPosition, PhysicalSize};

pub const WIDTH: u32 = 1280;
pub const HEIGHT: u32 = 820;

thread_local! {
    /// The window's size in pixels, changed by `resize`.
    static SIZE: Cell<(u32, u32)> = const { Cell::new((WIDTH, HEIGHT)) };
}

/// Changes the window's size for the following renders.
pub fn resize(window: &MinimalSoftwareWindow, width: u32, height: u32) {
    SIZE.set((width, height));
    window.set_size(PhysicalSize::new(width, height));
}

struct Headless(Rc<MinimalSoftwareWindow>);

impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

/// Installs the headless platform. Must run before any component is created.
pub fn install() -> Rc<MinimalSoftwareWindow> {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    window.set_size(PhysicalSize::new(WIDTH, HEIGHT));
    slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
    window
}

/// Lets timers and animations run for `duration` of wall-clock time.
pub fn settle(duration: Duration) {
    std::thread::sleep(duration);
    slint::platform::update_timers_and_animations();
}

pub fn hover(window: &MinimalSoftwareWindow, x: f32, y: f32) {
    window.dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(x, y),
    });
    settle(Duration::from_millis(250));
}

/// Scrolls the mouse wheel at `(x, y)`; positive `delta` scrolls up.
pub fn click(window: &MinimalSoftwareWindow, x: f32, y: f32) {
    let position = LogicalPosition::new(x, y);
    window.dispatch_event(WindowEvent::PointerMoved { position });
    let button = slint::platform::PointerEventButton::Left;
    window.dispatch_event(WindowEvent::PointerPressed { position, button });
    window.dispatch_event(WindowEvent::PointerReleased { position, button });
    slint::platform::update_timers_and_animations();
}

/// Presses and releases `key`, such as `Key::RightArrow`.
pub fn key(window: &MinimalSoftwareWindow, key: slint::platform::Key) {
    let text: slint::SharedString = key.into();
    window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text });
    slint::platform::update_timers_and_animations();
}

/// The mouse leaves the window.
pub fn leave(window: &MinimalSoftwareWindow) {
    window.dispatch_event(WindowEvent::PointerExited);
    slint::platform::update_timers_and_animations();
}

pub fn scroll(window: &MinimalSoftwareWindow, x: f32, y: f32, delta: f32) {
    let position = LogicalPosition::new(x, y);
    window.dispatch_event(WindowEvent::PointerMoved { position });
    window.dispatch_event(WindowEvent::PointerScrolled {
        position,
        delta_x: 0.0,
        delta_y: delta,
    });
    settle(Duration::from_millis(250));
}

/// Renders a frame without saving it. Elements created by `if` and `for`
/// only exist after a frame, so this starts their entry animations.
pub fn render(window: &MinimalSoftwareWindow) -> Vec<PremultipliedRgbaColor> {
    let (width, height) = SIZE.get();
    window.request_redraw();
    let mut pixels = vec![PremultipliedRgbaColor::default(); (width * height) as usize];
    window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, width as usize);
    });
    pixels
}

pub fn save(window: &MinimalSoftwareWindow, path: &Path) -> std::io::Result<()> {
    let pixels = render(window);
    let (width, height) = SIZE.get();
    let bytes: Vec<u8> = pixels
        .iter()
        .flat_map(|p| [p.red, p.green, p.blue])
        .collect();
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(path)?),
        width,
        height,
    );
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&bytes)?;
    Ok(())
}
