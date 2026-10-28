<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://slintplot.dev/assets/slintplot-logo-dark.png">
    <img src="https://slintplot.dev/assets/slintplot-logo.png" alt="SlintPlot" width="420">
  </picture>
</p>

# SlintPlot

Native, interactive charts for [Slint](https://slint.dev).

Charts are built from Slint's own `Path`, `Rectangle` and `Text` elements, not
rendered into images. They stay sharp at any scale, follow the light/dark theme
and react to the mouse.

![The gallery app](https://slintplot.dev/screenshots/light.png)
*The gallery app: a bar chart with axis titles, currency formatting and data labels, in the Modern theme. The sidebar lists the samples by category.*

Full API documentation with examples: [slintplot.dev](https://slintplot.dev).

Try the charts in your browser: **[slintplot.dev/demo](https://slintplot.dev/demo/)**, or install the gallery app:

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
- **Axes**: titles, fixed ranges, logarithmic scales, value formatting (prefix, suffix, decimals), and a second value axis on the right with ticks lined up with the first
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
- **Languages**: numbers and dates in the conventions of 14 languages, or your own

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
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/axes-two.png" alt="A second value axis: rainfall on the right, its ticks on the left axis' grid"><br><em>A second value axis: rainfall on the right, its ticks on the left axis' grid</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/legend-position.png" alt="The legend on the left: legend-position places it on any side"><br><em>The legend on the left: legend-position places it on any side</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/zoom.png" alt="Zoom and pan: scroll to zoom around the mouse, drag to pan"><br><em>Zoom and pan: scroll to zoom around the mouse, drag to pan</em></td>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/decimation.png" alt="Decimation: two lines of 100,000 points each, drawn as one span per pixel column"><br><em>Decimation: two lines of 100,000 points each, drawn as one span per pixel column</em></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="https://slintplot.dev/screenshots/charts/radar-dark.png" alt="Dark mode: every chart and theme has dark colors"><br><em>Dark mode: every chart and theme has dark colors</em></td>
    <td width="50%" valign="top"></td>
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
                    right-axis: false,              // true: the second value axis
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

            // Second value axis, for datasets with right-axis: true
            y2-title: "";
            y2-begin-at-zero: false;                // like begin-at-zero
            y2-fixed-range: false;
            y2-min-value: 0;
            y2-max-value: 100;
            y2-logarithmic: false;
            y2-value-prefix: "";
            y2-value-suffix: "";
            y2-value-decimals: -1;

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
| `datasets` | `[ChartDataset]` | | `{ label, values, color, hidden, fill, tension, stepped, series-type, right-axis, points }`. Leave `color` unset to use the theme's palette. |
| `labels` | `[string]` | | Category labels on the x axis |
| `title` | `string` | `""` | |
| `begin-at-zero` | `bool` | `true` for bars | Include 0 in the value axis |
| `stacked` | `bool` | `false` | Stack the datasets; lines and bars stack separately, and so do the two value axes |
| `y2-title`, `y2-value-prefix`, `y2-value-suffix`, `y2-value-decimals`, `y2-fixed-range`, `y2-min-value`, `y2-max-value`, `y2-logarithmic`, `y2-begin-at-zero` | | | The second value axis, for datasets with `right-axis: true`, like the left axis' properties |
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

A dataset with `right-axis: true` is plotted against a second value axis on
the right, with its own title, format and range. Its ticks line up with the
left axis' grid lines, unless either axis is fixed or logarithmic:

```slint
BarChart {
    labels: ["Jan", "Feb", "Mar", "Apr"];
    datasets: [
        { label: "Temperature", values: [-1.5, -1.2, 1.2, 5.5], series-type: ChartSeriesType.line },
        { label: "Rainfall", values: [86, 136, 88, 103], right-axis: true },
    ];
    y-title: "Temperature";
    value-suffix: " °C";
    y2-title: "Rainfall";
    y2-value-suffix: " mm";
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

### Languages

`ChartTheme.language` sets how numbers and dates are written: separators, the
percent sign, and month and weekday names and date order on time axes,
tooltips and calendar heatmaps. It also translates the charts' few built-in
texts, such as "Less" and "More" under a calendar heatmap and a waterfall's
legend. Prefixes and suffixes stay as you set them.

```slint
import { ChartLanguage, ChartTheme } from "@slintplot";

export component App inherits Window {
    init => {
        ChartTheme.language = ChartLanguage.german; // 12.345,6 and 25. Okt. 2026
    }
}
```

The built-in languages are English (the default), Chinese, Danish, Dutch,
Finnish, French, German, Italian, Japanese, Norwegian, Polish, Portuguese,
Spanish and Swedish. For other conventions, assign a `ChartLocale` to
`ChartTheme.locale`; fields you leave out are English:

```slint
ChartTheme.locale = { decimal: ".", group: "'" }; // Swiss numbers, English dates
```

`ChartTexts` has the built-in texts in the current language, and
`ChartTexts.percent` the percent sign, for a `value-suffix` such as a gauge's.

Japanese and Chinese need a font with their characters, which desktop systems
have; in the browser, Slint only has its built-in Inter font. See
[Languages](https://slintplot.dev/#languages) for following the system
language from Rust.

### Streaming

For live data, give a dataset a model that appends new values and drops the
oldest, and add the samples that arrived since the last frame in one batch per
frame. The example apps use a ring buffer, `StreamModel`
([stream_model.rs](https://github.com/trondegil/slintplot/blob/main/examples/gallery/src/stream_model.rs),
about 70 lines), because `VecModel::remove(0)` moves every remaining value.
`TimeSeriesChart` points must be sorted by `x`, which appending newer points
keeps. See [Streaming](https://slintplot.dev/#streaming).

### Large datasets

Values are read from the data models on demand. A chart zoomed into part of
a large dataset reads only the visible values, and changes outside the view
cause no work. With `decimate: true` (the default), a line with more than twice as many points as pixels is
drawn as one span per pixel column; set `decimate: false` to draw every
segment.

### Performance

Every chart is drawn with Slint's own elements, so the drawing cost grows with
the data. The stress test app (`cargo install slintplot-stress`) was built to
show where that starts to matter: it pushes each chart type from 100 to
1,000,000 points, streams data at 60 Hz and shows live timings.
`slintplot-stress --bench` measures your own machine. The full results and what
they mean are at [slintplot.dev/#performance](https://slintplot.dev/#performance).

Frame times with Slint's software renderer on one core of an Apple M4,
1280×800, 3 datasets, median of three runs (they differed by at most ±8%).
A 60 fps frame has 16.7 ms.

| Points per dataset | Line | Line, decimated | Time series, decimated | Bars | Scatter |
|---:|---:|---:|---:|---:|---:|
| 100 | 5.0 ms | 4.7 ms | 3.6 ms | 3.8 ms | 3.3 ms |
| 1,000 | 7.9 ms | 6.6 ms | 6.9 ms | 6.5 ms | 5.2 ms |
| 10,000 | 60 ms | 6.1 ms | 5.9 ms | 34 ms | 26 ms |
| 100,000 | 1.3 s | 26 ms | 28 ms | 303 ms | not run |
| 1,000,000 | 23.8 s | 244 ms | 259 ms | not run | not run |

| Steps, rows, stages or items | Waterfall | Dumbbell | Funnel | Sunburst |
|---:|---:|---:|---:|---:|
| 100 | 1.6 ms | 1.3 ms | 0.8 ms | 4.2 ms |
| 1,000 | 8.8 ms | 10 ms | 3.6 ms | 26 ms |
| 10,000 | 83 ms | 104 ms | 31 ms | 307 ms |

Streaming limits: the largest window (values per dataset) that still
updates at 60 fps when each update appends a value to 3 datasets and renders a
frame (`slintplot-stress --bench-stream`, same machine):

| Chart | Everything shown | Latest 1,000 shown |
|---|---:|---:|
| Line or time series, decimated | 50,000 | 10,000,000+ |
| Line, time series or bars, not decimated | 2,000 | 10,000,000+ |
| Scatter | 2,000 | 20,000 |

Hovering a decimated line takes about 4 ms at every size, and zooming in to
0.1% of a million points brings a frame to 46 ms. Not measured: Slint's GPU
renderers (the default in desktop apps), the browser, other machines, memory
use.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://slintplot.dev/screenshots/zoom-dark.png">
  <img src="https://slintplot.dev/screenshots/zoom.png" alt="Three datasets of 1,000,000 points, zoomed in">
</picture>

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
cargo install slintplot-stress    # stress test: streaming up to 8 datasets, up to 100,000 samples/s
```

Run them as `slintplot-gallery` and `slintplot-stress`. `slintplot-gallery --snapshot <dir>` renders every
sample to PNG, and `slintplot-stress --bench` prints headless timings. On Linux, Slint needs the fontconfig and
xkbcommon development packages (`libfontconfig-dev` and `libxkbcommon-dev` on Debian and Ubuntu).

The source of the apps is in [`examples/gallery`](https://github.com/trondegil/slintplot/tree/main/examples/gallery) and
[`examples/stress`](https://github.com/trondegil/slintplot/tree/main/examples/stress).

Ready-built apps for macOS, Windows and Linux are on the
[releases page](https://github.com/trondegil/slintplot/releases/tag/apps-v0.1.2).

<a href="https://slint.dev"><img src="https://slintplot.dev/assets/MadeWithSlint-logo-whitebg.png" alt="Made with Slint" width="212"></a>

## Development

```sh
cargo test                                                     # layout, ticks and path tests
cargo run -p slintplot-gallery                                 # sample browser
cargo run -p slintplot-gallery -- --snapshot target/snapshots  # headless PNG renders
cargo run --release -p slintplot-stress                        # stress test, streaming
cargo run --release -p slintplot-stress -- --bench             # headless timings
cargo run --release -p slintplot-stress -- --snapshot docs/screenshots  # the stress test screenshots
```

## Limitations

What SlintPlot 0.2 doesn't do yet, or does with restrictions:

- **Rust only.** The chart geometry is computed in Rust and connected with `register!`, so the library can't be used from Slint's C++, JavaScript or Python APIs. For the same reason, charts aren't drawn in Slint's live preview or SlintPad.
- **Platforms.** Tested on desktop (macOS, Windows and Linux) and in the browser with WebAssembly, from version 0.1.2: the gallery runs [in the browser](https://slintplot.dev/demo/). Embedded (MCU) builds are untested. With Slint's software renderer, enable its `software-renderer-path` feature, or charts aren't drawn.
- **14 built-in languages.** `ChartTheme.language` covers 14 languages; for others, set the `ChartLocale` fields yourself. Grouping is always in thousands (no Indian lakh grouping, and four-digit numbers are grouped in every language), digits are Western, and right-to-left layout isn't supported.
- **Time zones.** Time axes are in UTC, with an optional fixed offset. Named time zones and daylight saving time changes aren't supported.
- **Two value axes at most**, and only in line, bar, scatter, bubble and time series charts: horizontal bar charts have one value axis.
- **Zoom, pan and decimation** work in the cartesian charts only (line, bar, scatter and time series). Zoom is off by default.
- **Eight colors per theme.** With more than eight datasets the colors repeat. Set your own palette for more.
- **Large charts of separate shapes are slower.** Sunburst, waterfall, dumbbell and funnel charts draw every item as its own shape: about 320 ms for a sunburst with 10,000 items, and about 100 ms for waterfall and dumbbell charts. With up to 1,000 items, a frame takes under 30 ms. Decimation applies to lines only.
- **Label fitting is estimated.** Sunburst and treemap labels are kept or hidden from an estimate of the text width, so very wide or narrow fonts can be clipped or hidden too early.
- **No image or PDF export.** To save a chart as PNG, render the window with Slint's software renderer, as the gallery's `--snapshot` option does.
- **Sunburst data order.** A parent item must come before its children in `items`.
- **Requirements.** Slint 1.18 or later and Rust 1.88 or later.

## License

Download SlintPlot, experiment and build prototypes for free, at home or at
work. You need a commercial license when you sell, ship or deliver a
closed-source product that uses it.

SlintPlot is available under your choice of three licenses:

- **GPLv3**: free for anyone, for any purpose. Software that includes
  SlintPlot and is distributed to others must be distributed under the
  GPLv3, with its source code.
- **[PolyForm Noncommercial 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0)**:
  free for personal, hobby, research, education and other noncommercial use,
  with or without sharing your source code.
- **A commercial license**: for commercial use without the GPLv3's
  conditions, such as in closed-source software you sell or ship. Per
  developer, per year, from €199; bought by 31 December 2026, half price
  through 2027: see [Pricing](https://slintplot.dev/#pricing).

See [LICENSE.md](LICENSE.md) and [COMMERCIAL.md](COMMERCIAL.md) for the
details. Your application also needs a suitable
[Slint license](https://slint.dev/pricing).

SlintPlot is an independent project. It is not affiliated with, endorsed by or sponsored by SixtyFPS GmbH or the Slint project. Slint and the Slint logo are trademarks of SixtyFPS GmbH.
