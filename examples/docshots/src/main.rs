//! Renders the website's chart screenshots from the code examples next to
//! them: `cargo run -p slintplot-docshots -- docs/screenshots/charts`.
//! Each `<pre data-shot="name">` example in docs/index.html becomes
//! `name.png` and `name-dark.png`, 1012 by 660 pixels.

use std::time::Duration;

use slint::{ComponentHandle, ModelRc, VecModel};

#[allow(dead_code)]
#[path = "../../gallery/src/snapshot.rs"]
mod snapshot;

slint::include_modules!();
include!(concat!(env!("OUT_DIR"), "/shot_names.rs"));

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: slintplot-docshots <dir>")?,
    );
    std::fs::create_dir_all(&dir)?;
    let window = snapshot::install();
    snapshot::resize(&window, 1012, 660);
    let ui = Shots::new()?;
    slintplot::register!(ui);
    ui.set_daily_counts(ModelRc::new(VecModel::from(daily_counts())));
    ui.show()?;
    for (i, name) in SHOTS.iter().enumerate() {
        for (dark, suffix) in [(false, ""), (true, "-dark")] {
            ui.set_dark(dark);
            // Hide the chart first, so it animates in again.
            ui.set_shot(-1);
            snapshot::render(&window);
            ui.set_shot(i as i32);
            snapshot::render(&window);
            snapshot::settle(Duration::from_millis(1100));
            snapshot::render(&window);
            snapshot::settle(Duration::from_millis(400));
            snapshot::save(&window, &dir.join(format!("{name}{suffix}.png")))?;
        }
    }
    Ok(())
}

/// A year of contributions for the calendar heatmap example: busier on
/// weekdays, with some days off. The same every run.
fn daily_counts() -> Vec<f32> {
    let mut seed: u32 = 0x5eed;
    (0..365)
        .map(|day| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let random = seed % 100;
            // The example starts on a Sunday.
            let weekend = day % 7 == 0 || day % 7 == 6;
            if random < if weekend { 70 } else { 20 } {
                0.0
            } else {
                (random % 12) as f32
            }
        })
        .collect()
}
