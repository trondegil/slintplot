<p align="center">
  <a href="https://slintplot.dev">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://slintplot.dev/assets/slintplot-logo-dark.png">
      <img src="https://slintplot.dev/assets/slintplot-logo.png" alt="SlintPlot" width="420">
    </picture>
  </a>
</p>

<p align="center">
  <b><a href="https://slintplot.dev">Documentation</a></b> ·
  <b><a href="https://slintplot.dev/demo/">Try the gallery in your browser</a></b> ·
  <b><a href="https://slintplot.dev/#pricing">Buy a commercial license</a></b>
</p>

<p align="center">
  <a href="https://crates.io/crates/slintplot"><img src="https://img.shields.io/crates/v/slintplot.svg" alt="crates.io"></a>
  <img src="https://img.shields.io/badge/Slint-1.18%2B-2379f4.svg" alt="Slint 1.18 or later">
  <img src="https://img.shields.io/badge/Rust-1.88%2B-dea584.svg" alt="Rust 1.88 or later">
</p>

<br>

SlintPlot is a chart library for [Slint](https://slint.dev) in Rust. It draws
charts with Slint's own `Path`, `Rectangle` and `Text` elements, not images,
so charts stay sharp at any scale, follow the theme and dark mode, and react
to the mouse.

- **Cartesian charts.** Line, bar, horizontal bar, scatter and bubble charts, with stacked, mixed, stepped and filled variants.
- **Radial charts.** Pie, doughnut, polar area and radar charts.
- **Time and heatmaps.** Time series with date axes, matrix heatmaps and GitHub-style calendar heatmaps.
- **Hierarchies and business.** Treemaps, sunbursts, funnels, waterfalls, dumbbells and gauges.
- **Interactive.** Tooltips, hover highlights, clickable legends, click callbacks, zoom and pan, keyboard navigation.
- **Large datasets.** Min-max decimation and on-demand data reading for millions of points.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://slintplot.dev/screenshots/light-dark.png">
  <img src="https://slintplot.dev/screenshots/light.png" alt="The gallery app: a bar chart with axis titles, data labels and currency formatting">
</picture>

*The gallery app shows every chart type with live data, themes and dark mode.
[Try it in your browser](https://slintplot.dev/demo/), install it with
`cargo install slintplot-gallery`, or download it ready-built from the
[releases page](https://github.com/trondegil/slintplot/releases/tag/apps-v0.2.2).*

## Getting started

**1. Add the dependencies.** SlintPlot is both a regular dependency (the
`register!` macro) and a build dependency (to find its `.slint` files).

```toml
[dependencies]
slint = "1.18"
slintplot = "0.2"

[build-dependencies]
slint-build = "1.18"
slintplot = "0.2"
```

**2. Make the library importable** in `build.rs`:

```rust
fn main() {
    let config = slint_build::CompilerConfiguration::new().with_library_paths(
        std::collections::HashMap::from([("slintplot".into(), slintplot::library_path())]),
    );
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}
```

**3. Use a chart** in `ui/app.slint`. Import the charts you use, plus
`ChartEngine`. Slint generates code only for the components your UI uses, so
the other charts cost nothing.

```slint
import { BarChart, ChartEngine, LineChart } from "@slintplot";
export { ChartEngine } // required, so Rust can connect it

export component App inherits Window {
    preferred-width: 800px;
    preferred-height: 500px;

    VerticalLayout {
        LineChart {
            title: "Revenue";
            labels: ["Jan", "Feb", "Mar", "Apr"];
            datasets: [
                { label: "2025", values: [12, 19, 8, 15], fill: true, tension: 0.4 },
                { label: "2026", values: [15, 22, 13, 24], tension: 0.4 },
            ];
            value-prefix: "$";
            category-clicked(index) => { debug("clicked", index); }
        }
        BarChart {
            labels: ["Desktop", "Mobile", "Tablet"];
            datasets: [{ label: "Visitors", values: [1200, 2400, 400] }];
        }
    }
}
```

**4. Connect it** in `src/main.rs`:

```rust
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = App::new()?;
    slintplot::register!(app);
    app.run()
}
```

Datasets are ordinary Slint models. Change a row from Rust, for example with
`VecModel::set_row_data`, and the chart updates. The
[Getting started](https://slintplot.dev/#getting-started) section lists every
property of a `LineChart` with its default value.

## Charts

Each name links to the chart's documentation, with a screenshot, a code
example and its properties.

| Chart | What it draws |
|---|---|
| [`LineChart`](https://slintplot.dev/#line-chart) | Lines through each dataset's values, straight, smooth or stepped, with optional area fill |
| [`BarChart`](https://slintplot.dev/#bar-chart) | Vertical bars, grouped, stacked or [100% stacked](https://slintplot.dev/#percent-bars) |
| [`HorizontalBarChart`](https://slintplot.dev/#horizontal-bar-chart) | Categories down the left side, bars growing to the right |
| [`ScatterChart`](https://slintplot.dev/#scatter-chart) | Points on numeric x and y axes |
| [`BubbleChart`](https://slintplot.dev/#bubble-chart) | A scatter chart whose points have a radius |
| [`PieChart`](https://slintplot.dev/#pie-chart), [`DoughnutChart`](https://slintplot.dev/#doughnut-chart) | Slices proportional to their values; the hovered slice moves outwards |
| [`PolarAreaChart`](https://slintplot.dev/#polar-area-chart) | Slices of equal angle whose radius shows the value |
| [`RadarChart`](https://slintplot.dev/#radar-chart) | One spoke per label and one closed polygon per dataset |
| [`TimeSeriesChart`](https://slintplot.dev/#time-series) | Lines over a time axis with calendar-aligned ticks |
| [`HeatmapChart`](https://slintplot.dev/#heatmap) | A matrix of colored cells, sequential or diverging |
| [`CalendarHeatmap`](https://slintplot.dev/#calendar-heatmap) | Daily values over a year, like GitHub's contribution graph |
| [`TreemapChart`](https://slintplot.dev/#treemap) | Squarified rectangles sized by value, optionally in groups |
| [`SunburstChart`](https://slintplot.dev/#sunburst) | A hierarchy as rings; click an item to zoom into it |
| [`FunnelChart`](https://slintplot.dev/#funnel) | Stages as slices of one curved funnel, with conversion rates |
| [`WaterfallChart`](https://slintplot.dev/#waterfall) | Increases, decreases and totals, with connector lines |
| [`DumbbellChart`](https://slintplot.dev/#dumbbell) | A dot per value and a bar between them, such as before and after |
| [`GaugeChart`](https://slintplot.dev/#gauge) | A filled arc or a needle over colored bands |

Line and bar charts can mix the two: any dataset can be drawn as a line or as
bars with `series-type`, such as an average line over bars. See
[Mixed charts](https://slintplot.dev/#mixed-charts).

## Features

- **[Axes](https://slintplot.dev/#axes).** Titles, fixed ranges, logarithmic scales, prefixes, suffixes and decimals, and a second value axis on the right with ticks lined up with the first.
- **[Data labels](https://slintplot.dev/#data-labels).** On bars, points and pie slices, as values or percentages.
- **[Interaction](https://slintplot.dev/#interaction).** Tooltips and hover highlights, a clickable legend that hides datasets or slices, click callbacks, and keyboard navigation with accessibility labels.
- **Zoom and pan.** In line, bar, scatter and time series charts, with the mouse wheel, by dragging, or from code.
- **[Themes](https://slintplot.dev/#theming).** Five built-in palettes (Modern, Material, Corporate, Mono and Pastel) with light and dark variants, or your own colors. Dark mode follows Slint's `Palette`.
- **[Languages](https://slintplot.dev/#languages).** Numbers and dates in the conventions of 14 languages, or your own.
- **[Streaming](https://slintplot.dev/#streaming) and [large datasets](https://slintplot.dev/#large-datasets).** Charts read values from the models on demand, and decimation draws a million-point line as one span per pixel column.

## Performance

Every chart is drawn with Slint's own elements, so the cost grows with the
data. Frame times with Slint's software renderer on one core of an Apple M4,
1280×800, 3 datasets (a 60 fps frame has 16.7 ms):

| Points per dataset | Line | Line, decimated | Time series, decimated | Bars | Scatter |
|---:|---:|---:|---:|---:|---:|
| 1,000 | 6.6 ms | 6.4 ms | 5.7 ms | 5.6 ms | 4.6 ms |
| 10,000 | 57 ms | 5.3 ms | 5.4 ms | 30 ms | 27 ms |
| 100,000 | 1.1 s | 24 ms | 28 ms | 275 ms | not run |
| 1,000,000 | 16.0 s | 237 ms | 277 ms | not run | not run |

A decimated line streaming the latest 1,000 of 10,000,000 values still updates
at 60 fps. [Performance](https://slintplot.dev/#performance) has the full
results, including sunburst, waterfall, dumbbell and funnel charts, and the
stress test app (`cargo install slintplot-stress`) measures your own machine.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://slintplot.dev/screenshots/zoom-dark.png">
  <img src="https://slintplot.dev/screenshots/zoom.png" alt="Three datasets of 1,000,000 points, zoomed in">
</picture>

*The stress test: three datasets of 1,000,000 points each, zoomed in to 0.1% of the data. Only the visible values are read.*

## How it works

Slint can't build a path from an array inside a `.slint` file (`for` isn't
allowed in `Path`), so SlintPlot computes the geometry in Rust. The library
declares a `ChartEngine` global with pure callbacks, and `register!` connects
them to the layout code in this crate. Because the callbacks are pure and read
the models through their trackers, Slint calls them again whenever the size or
the data changes, and the components draw the result.

## Example apps

```sh
cargo install slintplot-gallery   # every chart type, themes and dark mode
cargo install slintplot-stress    # up to 8 × 1,000,000 points, 60 Hz streaming
```

`slintplot-gallery --snapshot <dir>` renders every sample to PNG, and
`slintplot-stress --bench` prints headless timings. On Linux, Slint needs the
fontconfig and xkbcommon development packages (`libfontconfig-dev` and
`libxkbcommon-dev` on Debian and Ubuntu). Ready-built apps for macOS, Windows
and Linux are on the
[releases page](https://github.com/trondegil/slintplot/releases/tag/apps-v0.2.2),
and the source is in [`examples/gallery`](https://github.com/trondegil/slintplot/tree/main/examples/gallery) and
[`examples/stress`](https://github.com/trondegil/slintplot/tree/main/examples/stress). The gallery's `ui/gallery.slint` has a
working example of every chart.

## Limitations

What SlintPlot 0.2 doesn't do yet, or does with restrictions:

- **Rust only.** The chart geometry is computed in Rust, so the library can't be used from Slint's C++, JavaScript or Python APIs, and charts aren't drawn in Slint's live preview or SlintPad.
- **Platforms.** Tested on desktop (macOS, Windows and Linux) and in the browser with WebAssembly. Embedded (MCU) builds are untested. With Slint's software renderer, enable its `software-renderer-path` feature, or charts aren't drawn.
- **Languages.** Grouping is always in thousands, digits are Western, and right-to-left layout isn't supported.
- **Time zones.** Time axes are in UTC, with an optional fixed offset. Named time zones and daylight saving time aren't supported.
- **Two value axes at most**, in line, bar, scatter, bubble and time series charts.
- **Zoom, pan and decimation** work in line, bar, scatter and time series charts only.
- **Eight colors per theme.** With more datasets the colors repeat; set your own palette for more.
- **Charts of separate shapes are slower.** Sunburst, waterfall, dumbbell and funnel charts draw every item as its own shape: under 30 ms a frame up to 1,000 items, about 270 ms for a sunburst with 10,000.
- **No image or PDF export.** To save a chart as PNG, render the window with Slint's software renderer, as the gallery's `--snapshot` option does.
- **Requirements.** Slint 1.18 or later and Rust 1.88 or later.

The full list is under [Limitations](https://slintplot.dev/#limitations).

## Development

```sh
cargo test                                                     # layout, ticks and path tests
cargo run -p slintplot-gallery                                 # sample browser
cargo run -p slintplot-gallery -- --snapshot target/snapshots  # headless PNG renders
cargo run --release -p slintplot-stress                        # stress test, streaming
cargo run --release -p slintplot-stress -- --bench             # headless timings
cargo run --release -p slintplot-stress -- --snapshot docs/screenshots  # the stress test screenshots
cargo run --release -p slintplot-gallery -- --docs docs/screenshots     # the website's gallery pictures
cargo run --release -p slintplot-docshots -- docs/screenshots/charts   # the chart pictures, from the page's code
```

Each code example on the website with a screenshot is marked `<pre data-shot="name">`
in `docs/index.html`. `slintplot-docshots` compiles those examples and renders them, so
a picture always shows its code.

## Pricing and license

Download SlintPlot, experiment and build prototypes for free, at home or at
work. You need a commercial license when you sell, ship or deliver a
closed-source product that uses it.

SlintPlot is available under your choice of three licenses:

- **[GNU GPL, version 3](https://github.com/trondegil/slintplot/blob/main/LICENSE-GPL-3.0.txt)**:
  free for anyone, for any purpose, including companies. Software that
  includes SlintPlot and is distributed to others must be distributed under
  the GPLv3, with its source code. Using SlintPlot only within your own
  organization, without distributing it, has no such condition.
- **[PolyForm Noncommercial 1.0.0](https://github.com/trondegil/slintplot/blob/main/LICENSE-POLYFORM-NC.md)**:
  free for any noncommercial purpose, such as personal and hobby projects,
  research, education, and use by charitable and public organizations, with
  or without sharing your source code.
- **[A commercial license](https://github.com/trondegil/slintplot/blob/main/COMMERCIAL.md)**:
  for commercial use without the GPLv3's conditions, such as in
  closed-source software you sell, ship or deliver to clients.

Commercial licenses are per developer, per year, and cover any number of
your products. Prices exclude VAT.

| Plan | For | Price | Early bird |
|---|---|---|---|
| Indie | Solo developers and companies with under €250,000 in annual revenue | €99 per developer per year | €49 |
| Business | Every other company | €299 per developer per year | €149 |
| Enterprise | Every developer in the company, custom terms | from €2,500 per year | from €1,250 |

**Early bird:** buy by 31 December 2027 and pay half price for your first
two years: the first year and the first renewal. **Perpetual fallback:**
products you release while your license is active may keep being sold and
shipped after it ends, with the SlintPlot version they were built with.

See [Pricing](https://slintplot.dev/#pricing) to buy a license, or email
<license@slintplot.dev>. The full terms are in
[LICENSE.md](https://github.com/trondegil/slintplot/blob/main/LICENSE.md) and
[COMMERCIAL.md](https://github.com/trondegil/slintplot/blob/main/COMMERCIAL.md).
Your application also needs a suitable [Slint license](https://slint.dev/pricing).

SPDX: `GPL-3.0-only OR PolyForm-Noncommercial-1.0.0 OR LicenseRef-SlintPlot-Commercial`

<a href="https://slint.dev"><img src="https://slintplot.dev/assets/MadeWithSlint-logo-whitebg.png" alt="Made with Slint" width="212"></a>

SlintPlot is an independent project. It is not affiliated with, endorsed by or sponsored by SixtyFPS GmbH or the Slint project. Slint and the Slint logo are trademarks of SixtyFPS GmbH.
