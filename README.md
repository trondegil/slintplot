<p align="center"><img src="https://slintplot.dev/assets/slintplot-logo.png" alt="SlintPlot" width="420"></p>

# SlintPlot

Native, interactive charts for [Slint](https://slint.dev).

Charts are built from Slint's own `Path`, `Rectangle` and `Text` elements, not
rendered into images. They stay sharp at any scale, follow the light/dark theme
and react to the mouse.

![The gallery app](https://slintplot.dev/screenshots/light.png)
*The gallery app: a bar chart with axis titles, currency formatting and data labels, in the Modern theme. The sidebar lists the samples by category.*

Full API documentation with examples: [slintplot.dev](https://slintplot.dev).

Try the charts with the gallery app:

```sh
cargo install slintplot-gallery
slintplot-gallery
```

## Features

- **Line charts**: straight or smooth (`tension`) lines, optional area fill
- **Bar charts**: grouped, stacked or 100% stacked (survey charts), vertical or horizontal
- **Mixed charts**: draw any dataset as a line or bars (`series-type`), such as an average line over bars
- **Stacking** for bars and areas, with positive and negative values stacked separately
- **Scatter and bubble charts** with numeric x and y axes
- **Pie, doughnut and polar area charts**; hovered pie slices move outwards
- **Radar charts** with a polygon grid and labels around the edge
- **Time series** with calendar-aligned date and time axes
- **Heatmaps**: matrix heatmaps (sequential or diverging) and GitHub-style calendar heatmaps
- **Treemaps**: squarified treemaps with optional groups, labels and values in the cells
- **Sunburst charts**: hierarchies as rings; click an item to zoom into it, and the center to zoom out
- **Funnel charts**: one curved funnel in a fade of one color, with each stage's share and conversion badges
- **Waterfall charts**: increases, decreases and totals, with connector lines
- **Dumbbell charts** comparing two (or more) values per category
- **Gauges**: a filled arc with rounded ends and a ring of bands, or a needle with ticks over colored bands
- **Axes**: titles, fixed ranges, logarithmic scales, and value formatting (prefix, suffix, decimals)
- **Data labels** on bars, points and pie slices, as values or percentages
- Tooltips and hover highlights: by category (`index` mode) for line, bar and
  radar charts, nearest point for scatter and bubble charts, slice under the
  mouse for pie and polar area charts
- Clickable legend that toggles datasets or slices; it wraps when space is short
- **Zoom and pan** for line, bar, scatter and time series charts: mouse wheel, drag, double-click to reset
- **Keyboard navigation** in every chart, with the arrow keys, Home, End and Enter, and accessibility labels
- **Large datasets**: min-max decimation, and zoomed views read only the visible values
- Entry animation, live updates when the data models change
- "Nice" axis ticks and automatic skipping of crowded category labels
- Five built-in color themes (modern, Material, corporate, mono, pastel), plus custom palettes
- Theming through the `ChartTheme` global, with dark mode following `Palette`

## Gallery

Screenshots from the gallery app (`cargo install slintplot-gallery`), in the default Modern theme: light mode, except for the two dark mode examples marked as such. Every sample in the app is here.

### Line and area charts

<table>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/line.png" alt="Line chart: smooth lines through the values, with point markers"><br><em>Line chart: smooth lines through the values, with point markers</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/line-interpolation.png" alt="Interpolation: the same values with straight segments and a smooth curve"><br><em>Interpolation: the same values with straight segments and a smooth curve</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/line-stepped.png" alt="Stepped line chart"><br><em>Stepped line chart</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/line-points.png" alt="Line chart without points: hovering still marks every line's value"><br><em>Line chart without points: hovering still marks every line's value</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/area-line.png" alt="Line chart with fill: the area under the first line is filled"><br><em>Line chart with fill: the area under the first line is filled</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/area-stacked.png" alt="Stacked area chart: each area fills down to the one below"><br><em>Stacked area chart: each area fills down to the one below</em></td>
  </tr>
</table>

### Bar charts

<table>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/bar-vertical.png" alt="Grouped bar chart with negative values"><br><em>Grouped bar chart with negative values</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/bar-horizontal.png" alt="Horizontal bar chart, with the legend on the right"><br><em>Horizontal bar chart, with the legend on the right</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/bar-stacked.png" alt="Stacked bars: positive and negative values stack separately"><br><em>Stacked bars: positive and negative values stack separately</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/survey.png" alt="100% stacked bars with percentage labels: survey answers in shades of blue"><br><em>100% stacked bars with percentage labels: survey answers in shades of blue</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/bar-border-radius.png" alt="Rounded bars with a solid outline"><br><em>Rounded bars with a solid outline</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/combo-bar-line.png" alt="Mixed chart: a line drawn over grouped bars"><br><em>Mixed chart: a line drawn over grouped bars</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/stacked-bar-line.png" alt="Stacked bars with a line on top: lines and bars stack separately"><br><em>Stacked bars with a line on top: lines and bars stack separately</em></td>
    <td width="50%"></td>
  </tr>
</table>

### Scatter, bubble and time series

<table>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/scatter.png" alt="Scatter chart on numeric axes"><br><em>Scatter chart on numeric axes</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/bubble.png" alt="Bubble chart: each bubble's radius is part of the data"><br><em>Bubble chart: each bubble's radius is part of the data</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/time-hourly.png" alt="Time series: two weeks of hourly values, with dates on the axis"><br><em>Time series: two weeks of hourly values, with dates on the axis</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/time-daily.png" alt="Time series: a year of daily values, with quarterly ticks"><br><em>Time series: a year of daily values, with quarterly ticks</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/time-week.png" alt="One week of electricity use every 15 minutes, with day ticks"><br><em>One week of electricity use every 15 minutes, with day ticks</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/time-week-1h.png" alt="The same week zoomed to one hour: 10-minute ticks, the date on the first"><br><em>The same week zoomed to one hour: 10-minute ticks, the date on the first</em></td>
  </tr>
</table>

### Pie, polar area and radar charts

<table>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/pie.png" alt="Pie chart: the hovered slice moves outwards"><br><em>Pie chart: the hovered slice moves outwards</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/pie-labels.png" alt="Pie chart with percentage labels"><br><em>Pie chart with percentage labels</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/doughnut.png" alt="Doughnut chart"><br><em>Doughnut chart</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/polar-area.png" alt="Polar area chart: equal angles, the radius shows the value"><br><em>Polar area chart: equal angles, the radius shows the value</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/radar.png" alt="Radar chart: one spoke per label, one polygon per dataset"><br><em>Radar chart: one spoke per label, one polygon per dataset</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/area-radar.png" alt="Radar chart with filled polygons"><br><em>Radar chart with filled polygons</em></td>
  </tr>
</table>

### Heatmaps and treemaps

<table>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/heatmap.png" alt="Heatmap: visits by weekday and hour"><br><em>Heatmap: visits by weekday and hour</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/heatmap-diverging.png" alt="Diverging heatmap: a correlation matrix, with values in the cells"><br><em>Diverging heatmap: a correlation matrix, with values in the cells</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/calendar.png" alt="Calendar heatmap: a year of daily values, in the theme's first color"><br><em>Calendar heatmap: a year of daily values, in the theme's first color</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/calendar-github.png" alt="The calendar heatmap in GitHub's green, chosen with its own color picker"><br><em>The calendar heatmap in GitHub's green, chosen with its own color picker</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/treemap.png" alt="Treemap: world population by country, grouped by continent"><br><em>Treemap: world population by country, grouped by continent</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/treemap-hidden.png" alt="The same treemap with Asia hidden from the legend"><br><em>The same treemap with Asia hidden from the legend</em></td>
  </tr>
</table>

### Sunbursts, funnels, waterfalls, dumbbells and gauges

<table>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/sunburst.png" alt="Sunburst: world population by continent, region and country"><br><em>Sunburst: world population by continent, region and country</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/sunburst-zoomed.png" alt="The same sunburst zoomed into Asia by clicking it, with a tooltip"><br><em>The same sunburst zoomed into Asia by clicking it, with a tooltip</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/funnel.png" alt="Funnel: website visitors through to a purchase, with each stage's share of the first"><br><em>Funnel: website visitors through to a purchase, with each stage's share of the first</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/waterfall.png" alt="Waterfall: a profit and loss statement with increases, decreases and totals"><br><em>Waterfall: a profit and loss statement with increases, decreases and totals</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/dumbbell.png" alt="Dumbbell: life expectancy in 2000 and 2023 by country"><br><em>Dumbbell: life expectancy in 2000 and 2023 by country</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/gauge.png" alt="Gauges: a half circle fill, a 240° fill with a ring of bands, and a needle with ticks"><br><em>Gauges: a half circle fill, a 240° fill with a ring of bands, and a needle with ticks</em></td>
  </tr>
</table>

### Axes, legend, zoom and large datasets

<table>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/axes-fixed.png" alt="A fixed value range from -150 to 150, with an axis title"><br><em>A fixed value range from -150 to 150, with an axis title</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/axes-log.png" alt="Logarithmic value axis"><br><em>Logarithmic value axis</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/legend-position.png" alt="The legend on the left: legend-position places it on any side"><br><em>The legend on the left: legend-position places it on any side</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/zoom.png" alt="Zoom and pan: scroll to zoom around the mouse, drag to pan"><br><em>Zoom and pan: scroll to zoom around the mouse, drag to pan</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/decimation.png" alt="Decimation: two lines of 100,000 points each, drawn as one span per pixel column"><br><em>Decimation: two lines of 100,000 points each, drawn as one span per pixel column</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/radar-dark.png" alt="Dark mode: every chart and theme has dark colors"><br><em>Dark mode: every chart and theme has dark colors</em></td>
  </tr>
</table>

## Usage

`Cargo.toml`:

```toml
[dependencies]
slint = "1.18"
slintplot = "0.1"

[build-dependencies]
slint-build = "1.18"
slintplot = "0.1"
```

`build.rs`:

```rust
fn main() {
    let config = slint_build::CompilerConfiguration::new().with_library_paths(
        std::collections::HashMap::from([("slintplot".into(), slintplot::library_path())]),
    );
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}
```

`ui/app.slint` (import only the charts you use, plus `ChartEngine`; Slint
generates code only for the components your UI uses, so the others cost
nothing):

```slint
import { BarChart, ChartEngine, ChartLegendPosition, ChartSeriesType, ChartTheme, LineChart } from "@slintplot";
export { ChartEngine } // required, so Rust can connect it

export component App inherits Window {
    VerticalLayout {
        // Every property is listed here, most at its default value. Properties
        // at their default value can be dropped, as in the BarChart below.
        LineChart {
            // Data
            title: "Revenue";                       // default ""
            labels: ["Jan", "Feb", "Mar", "Apr"];
            datasets: [
                {
                    label: "2025",
                    values: [12, 19, 8, 15],
                    points: [],                     // scatter, bubble and time series charts
                    color: transparent,             // transparent: the theme's next color
                    hidden: false,
                    fill: true,                     // default false
                    tension: 0.4,                   // default 0: straight lines
                    stepped: false,
                    series-type: ChartSeriesType.auto, // .line or .bar in mixed charts
                },
                { label: "2026", values: [15, 22, 13, 24], tension: 0.4 },
            ];

            // Look
            show-legend: true;
            legend-position: ChartLegendPosition.top;
            show-tooltip: true;
            show-points: true;
            show-values: false;
            fill-opacity: ChartTheme.fill-opacity;  // 0.5 by default
            animated: true;

            // Value axis
            begin-at-zero: false;                   // true for bar charts
            stacked: false;
            stacked-percent: false;
            fixed-range: false;
            min-value: 0;
            max-value: 100;
            logarithmic: false;
            x-title: "";
            y-title: "";
            value-prefix: "";
            value-suffix: "";
            value-decimals: -1;                     // -1: automatic

            // Large datasets, zoom and pan
            decimate: true;
            zoom-enabled: false;
            view-start: 0;
            view-end: 1;
            min-view: 0.0005;

            category-clicked(index) => { debug("clicked category", index); }
            legend-toggled(index) => { debug("toggled dataset", index); }
        }
        // Only the data: every other property keeps its default.
        BarChart {
            labels: ["Desktop", "Mobile", "Tablet"];
            datasets: [{ label: "Visitors", values: [1200, 2400, 400] }];
        }
    }
}
```

`src/main.rs`:

```rust
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = App::new()?;
    slintplot::register!(app);
    app.run()
}
```

Datasets are ordinary Slint models: change a row from Rust (for example with
`VecModel::set_row_data`) and the chart updates.

### Line, bar, scatter and bubble chart properties

`LineChart`, `BarChart`, `ScatterChart` and `BubbleChart` share these.
`HorizontalBarChart` and `RadarChart` support the subset that applies to them.

| Property | Type | Default | |
|---|---|---|---|
| `datasets` | `[ChartDataset]` | | `{ label, values, color, hidden, fill, tension, stepped, series-type, points }`. Leave `color` unset to use the theme's palette. |
| `labels` | `[string]` | | Category labels on the x axis |
| `title` | `string` | `""` | |
| `begin-at-zero` | `bool` | `true` for bars | Include 0 in the value axis |
| `stacked` | `bool` | `false` | Stack the datasets; lines and bars stack separately |
| `legend-position` | `ChartLegendPosition` | `top` | `top`, `bottom`, `left` or `right` |
| `bar-radius`, `bar-border-width` | `length` | `0`, `1px` | Rounded bars and their outline |
| `zoom-enabled` | `bool` | `false` | Mouse wheel zooms around the mouse, dragging pans, double-click resets |
| `view-start`, `view-end` | `float` (in-out) | `0`, `1` | The visible part of the x axis, as fractions; set them to zoom from code |
| `min-view` | `float` | `0.0005` | The smallest visible part when zooming |
| `zoomed` | `bool` (out) | | Whether the chart is zoomed in |
| `show-legend`, `show-tooltip`, `animated` | `bool` | `true` | |
| `show-points` | `bool` | `true` | Line charts: a marker on every point. Turn off for large datasets. |
| `decimate` | `bool` | `true` | Line charts and time series: min-max decimation when a line has more than twice as many points as the plot has pixels |
| `hovered-index` | `int` (out) | | Category under the mouse, or -1 |
| `hovered-point` | `ChartHit` (out) | | Scatter charts: point under the mouse; `dataset` is -1 when none |
| `category-clicked(int)`, `point-clicked(int, int)`, `legend-toggled(int)` | callbacks | | |

`reset-zoom()` is a public function on these charts.

A dataset can set `series-type: ChartSeriesType.line` or `.bar` to be drawn
differently from its chart:

```slint
BarChart {
    labels: ["Q1", "Q2", "Q3", "Q4"];
    datasets: [
        { label: "Sales", values: [12, 19, 8, 15] },
        { label: "Average", values: [10, 14, 12, 13], series-type: ChartSeriesType.line },
    ];
}
```

`ScatterChart` and `BubbleChart` datasets use `points: [{ x, y, r }]` instead
of `values`. `r` is the bubble radius in pixels, and scatter charts ignore it:

```slint
BubbleChart {
    datasets: [{ label: "Markets", points: [{ x: 20, y: 30, r: 15 }, { x: 40, y: 10, r: 8 }] }];
}
```

`RadarChart` takes `labels` and `datasets` like a line chart, with one spoke
per label:

```slint
RadarChart {
    labels: ["Speed", "Power", "Range", "Comfort", "Price"];
    datasets: [{ label: "Model A", values: [80, 65, 70, 50, 60], fill: true }];
}
```

### Pie, doughnut and polar area charts

```slint
DoughnutChart {
    title: "Traffic sources";
    slices: [
        { label: "Search", value: 48 },
        { label: "Direct", value: 22 },
        { label: "Social", value: 17 },
    ];
}
```

| Property | Type | Default | |
|---|---|---|---|
| `slices` | `[ChartSlice]` | | `{ label, value, color, hidden }`. Leave `color` unset to use the palette. |
| `cutout` | `float` | `0` (pie), `0.5` (doughnut) | Size of the hole, as a fraction of the radius |

`PolarAreaChart` takes the same `slices`: every slice gets the same angle and
its radius shows the value.
| `title`, `show-legend`, `show-tooltip`, `animated` | | | As above |
| `hovered-index` | `int` (out) | | Slice under the mouse, or -1 |
| `slice-clicked(int)`, `legend-toggled(int)` | callbacks | | |

### Sunburst, funnel, waterfall, dumbbell and gauge charts

These set only their data and a few options; the [API documentation](https://slintplot.dev/)
lists every property.

```slint
SunburstChart {
    // A parent's value is the sum of its children; parents come first.
    items: [
        { label: "Asia", parent: -1 },
        { label: "India", value: 1464, parent: 0 },
        { label: "China", value: 1416, parent: 0 },
        { label: "Europe", parent: -1 },
        { label: "Germany", value: 84, parent: 3 },
    ];
}
FunnelChart {
    stages: [
        { label: "Visited", value: 12400 },
        { label: "Added to cart", value: 3120 },
        { label: "Purchased", value: 1150 },
    ];
}
WaterfallChart {
    steps: [
        { label: "Revenue", value: 420 },
        { label: "Costs", value: -185 },
        { label: "Profit", total: true },
    ];
    value-prefix: "$";
}
DumbbellChart {
    labels: ["Japan", "India"];
    datasets: [
        { label: "2000", values: [81.1, 62.7] },
        { label: "2023", values: [84.7, 72.0] },
    ];
}
GaugeChart {
    value: 77;
    label: "CPU";
    value-suffix: "%";
    bands: [{ to: 60, color: green }, { to: 85, color: orange }, { to: 100, color: red }];
    show-needle: true;
}
```

### Themes

Datasets and slices that don't set a `color` take the next color of the
active palette. Each built-in theme has eight colors with light and dark mode
variants, ordered so that neighboring colors stay distinct, also with
red-green color blindness.

| `ChartTheme.palette` | Style |
|---|---|
| `modern` (default) | Clean, saturated colors in the style of modern UI frameworks (Tailwind) |
| `material` | Material Design colors |
| `corporate` | Restrained business colors, based on Tableau 10 |
| `mono` | Shades of one blue; best for up to five series |
| `pastel` | Soft, light colors; strongest on dark backgrounds or as fills, and best for up to three series |

```slint
import { ChartPalette, ChartTheme } from "@slintplot";

export component App inherits Window {
    init => {
        ChartTheme.palette = ChartPalette.corporate;
        // Or a custom palette:
        // ChartTheme.colors = [#0b6e4f, #f4a259, #5b8e7d, #bc4b51];
    }
}
```

The gallery's **Theme** selector switches between them live.

Filled marks (bars, areas, points, bubbles, polar slices) are
drawn with a translucent fill and a solid outline. `ChartTheme.fill-opacity`
(default `0.5`) sets the fill's opacity.

### Large datasets

Values are read from the data models on demand. A chart zoomed into part of
a large dataset reads only the visible values, and changes outside the view
cause no work. With `decimate: true` (the default), a line with more than twice as many points as pixels is
drawn as one span per pixel column; set `decimate: false` to draw every
segment. Measured with the software renderer and
3 datasets of 1,000,000 points each:

| | Full view | Zoomed to 0.1% |
|---|---|---|
| Layout | 194 ms | 0.8 ms |
| Frame | 248 ms | 47 ms |
| Hover | 4.3 ms | |

Frame times by size, 3 datasets, 1280×800 (`slintplot-stress --bench`, from `cargo install slintplot-stress`):

| Points per dataset | Line | Line, decimated | Bars | Time series, decimated | Scatter |
|---:|---:|---:|---:|---:|---:|
| 100 | 5.5 ms | 5.3 ms | 4.6 ms | 4.2 ms | 3.7 ms |
| 1,000 | 8.5 ms | 8.2 ms | 8.0 ms | 7.2 ms | 5.9 ms |
| 10,000 | 69 ms | 6.6 ms | 37 ms | 6.6 ms | 28 ms |
| 100,000 | 1.4 s | 27 ms | 317 ms | 31 ms | |
| 1,000,000 | 24 s | 248 ms | | 269 ms | |

Waterfall, dumbbell, funnel and sunburst charts draw one element per step,
row, stage or item, so their frame time grows with the count. With 3 datasets
(3 sunburst groups):

| Steps, rows, stages or items | Waterfall | Dumbbell | Funnel | Sunburst |
|---:|---:|---:|---:|---:|
| 100 | 1.9 ms | 1.4 ms | 1.7 ms | 4.6 ms |
| 1,000 | 9.1 ms | 11 ms | 4.0 ms | 27 ms |
| 10,000 | 95 ms | 106 ms | 34 ms | 323 ms |

A gauge frame takes 3 ms. Funnels leave out the stage names, values and
badges when the rows are too short for them, which also keeps them fast.

Measured on an Apple M4 with Slint 1.18. Without decimation, the software
renderer's time grows with the number of line segments and bars; with it, a
frame stays near the cost of one segment per pixel column. Hovering redraws
only the tooltip and markers, about 4 ms at every size with decimation on.

![Three datasets of 1,000,000 points, zoomed in](https://slintplot.dev/screenshots/zoom.png)
*The stress test: three datasets of 1,000,000 points each, zoomed in to 0.1% of the data. Only the visible values are read.*

### Software renderer

To use the software renderer, enable Slint's `software-renderer-path` feature,
or lines and areas won't be drawn.

## How it works

Slint can't build a path from an array inside a `.slint` file (`for` isn't
allowed in `Path`), so the geometry is computed in Rust. The library declares a
`ChartEngine` global with `pure callback`s, and `register!` connects them to the
layout code in this crate. Because the callbacks are pure and read the models
through their trackers, Slint re-evaluates them whenever the size or the data
changes, and the components just render the result.

![Dark mode](https://slintplot.dev/screenshots/dark.png)
*Dark mode: a time series in the gallery app. Charts follow Slint's color scheme automatically.*

## Example apps

```sh
cargo install slintplot-gallery   # sample browser: every chart type, themes and dark mode
cargo install slintplot-stress    # stress test: up to 8 × 1,000,000 points, 60 Hz streaming
```

Run them as `slintplot-gallery` and `slintplot-stress`. `slintplot-gallery --snapshot <dir>` renders every
sample to PNG, and `slintplot-stress --bench` prints headless timings. On Linux, Slint needs the fontconfig and
xkbcommon development packages (`libfontconfig-dev` and `libxkbcommon-dev` on Debian and Ubuntu).

The source of the apps is in [`examples/gallery`](https://github.com/trondegil/slintplot/tree/main/examples/gallery) and
[`examples/stress`](https://github.com/trondegil/slintplot/tree/main/examples/stress).

## Development

```sh
cargo test                                                     # layout, ticks and path tests
cargo run -p slintplot-gallery                                 # sample browser
cargo run -p slintplot-gallery -- --snapshot target/snapshots  # headless PNG renders
cargo run --release -p slintplot-stress                        # stress test, streaming
cargo run --release -p slintplot-stress -- --bench             # headless timings
```

## License

SlintPlot is source-available under the
[PolyForm Noncommercial License 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0):
free for personal, hobby, research and other noncommercial use. **Commercial
use requires a commercial license**: email
[license@slintplot.dev](mailto:license@slintplot.dev). `COMMERCIAL.md`, in
the crate, has the details.
