# Changelog

## Unreleased

### Example apps

- Gallery: the segmented ring and tile grid samples with four charts show
  one at a time, picked above the chart, on phones and small windows, where
  four were too small to read.

## 0.5.0

### Added

- `legend-attached` on pie, doughnut, polar area, radar, radial bar,
  sunburst, segmented ring and tile grid charts, on by default: the legend
  stays next to the chart instead of at the edge of its area, which was far
  away when the area was much taller or wider than the round or square
  chart in it. `false` keeps it at the edge, as before.

### Changed

- The focus ring round a chart shows only when the keyboard brought the
  focus (Tab) or has been used since, not after a click, as `:focus-visible`
  on the web. Clicking still gives the chart the arrow keys.

### Example apps

- **Gallery:** the line chart with and without points, the pie chart with
  and without data labels, and the radar chart with lines and filled are one
  sidebar entry each, with a tab per variant; the filled radar chart moves
  from Area charts to the radar chart.
- **Stress test:** `--bench-gpu [dir]` times the same charts as `--bench`
  with a GPU renderer, Skia on macOS, drawing each frame with
  `Window::take_snapshot` so that the GPU's work is included; with a folder,
  it saves every frame as a PNG.
- **Both apps:** a `skia` feature builds them with Slint's Skia renderer, to
  run with `SLINT_BACKEND=winit-skia`.

### Tested

- Skia: every gallery sample draws correctly with Slint's Skia renderer on
  macOS, Linux and Windows, as with FemtoVG and the software renderer.

### Documentation

- A compatibility box next to Getting started: Rust only, the Slint and
  Rust versions, the platforms and renderers tested, and no live preview.
- Performance: how many points or items each chart can show at 60 fps,
  30 fps and within a second, with the software renderer and with Skia, and
  why the numbers look as they do. The full results moved to a new
  benchmarks page, measured again with SlintPlot 0.5.0 and Slint 1.18.1, with
  Skia's numbers and how to repeat them.
- How to choose a renderer for the example apps.
- "Request a commercial license" until checkout opens.

### Website

- A library release (`v*`) no longer cancels the deploy of the apps
  release that follows it.

## 0.4.3

### Fixed

- `FunnelChart`: on Linux, Windows and in browsers, where Slint draws with
  femtovg, every stage was drawn a pixel larger all round, so the gap
  between stages was about half of `gap`, with a stray line at its ends.
  The stages are now drawn counterclockwise and back to their start, as the
  other charts' shapes since 0.4.2, and a test checks that Slint takes them
  as solid.

## 0.4.2

### Fixed

- `SegmentedRingChart`, `TileGridChart`, and the slices and rings of pie,
  doughnut, polar area, radial bar, gauge and sunburst charts: on Linux,
  Windows and in browsers, where Slint draws with femtovg, some cells merged
  into solid bands and gaps closed, on one half of a segmented ring chart and
  in every tile grid. Slint's femtovg renderer works out from a shape's
  points whether it is solid or a hole, leaving out the closing edge, so the
  answer depended on where the shape was; it draws a hole's smoothed edge a
  pixel outside it. Every shape is now drawn counterclockwise and back to its
  start, which Slint takes as solid wherever it is.

## 0.4.1

### Fixed

- `SegmentedRingChart`: the gaps between segments are as wide in every ring,
  with parallel sides, and at least 2px. They were a share of each segment's
  angle, so they widened outwards and, on screens of 1 pixel per point such
  as most Linux desktops and browsers, looked uneven where they were under 2
  pixels. `segment-gap` is now a share of a segment of the inner ring.

## 0.4.0

### Added

- `SegmentedRingChart`: a heatmap bent into concentric rings. `values` is a
  matrix (`[[float]]`), one row per ring, the first outermost, and one
  segment per column, going round from `start-angle`, either way. The matrix
  sets the size: as many rings as rows and as many segments as the longest
  row, so data of any shape fits without settings; shorter rows and missing
  values (NaN) are left as a faint track. Cells are colored on a scale, as in
  `HeatmapChart` (`color`, `diverging`, `fixed-range`), with a color scale
  legend, or by category: with `classes` set, each value is a category index,
  and the categories make a legend that hides them. `x-labels` go round the
  outside, thinned where they don't fit, and `y-labels` name the rings in the
  tooltip. `center-text` and `center-label` are written in the middle, and
  hovering a cell shows it there instead. A dot can mark the start, and
  `glow` adds a soft light in the middle. The cells sweep in when the chart
  appears; the arrow keys go round and between the rings, and
  `cell-clicked(row, column)` reports clicks.

- `TileGridChart`: a matrix as rows of rounded squares, one per value, like
  an activity grid. It takes its data as `SegmentedRingChart` does: `values`
  is a matrix (`[[float]]`), one row of tiles per row, and the matrix sets the
  size, the tiles staying square, as large as fit, and centered. Cells are
  colored on a scale or by category (`classes`), with the same properties; missing
  values are empty tiles, a faint track. `y-labels` go at the left and
  `x-labels` below, thinned where they don't fit; `tile-gap` and
  `tile-radius` shape the tiles. The tiles sweep in from the left; hovering
  outlines a tile with a tooltip, the arrow keys move from tile to tile, and
  `cell-clicked(row, column)` reports clicks.

### Example apps

- **Gallery:** builds without Slint's binding loop warnings. The window
  shrinks to phone width, so a browser window made narrower switches to the
  phone layout instead of cutting off the page; the layout switches at 900px
  instead of 720px, where the sample list and header no longer fit beside the
  chart.
  Samples of the two new charts under Heatmaps, each one sidebar entry with
  two tabs, by category and on a scale, of four charts of different sizes and
  uses. They follow the theme, with their own colors only where the colors
  carry meaning, such as green, amber and red for a service's status; on the
  segmented ring category page and both tile grid pages, a second dropdown
  under the theme picker switches between the theme's colors and these. The heatmap sample shows
  the visits in each cell, in two-hour blocks so that the numbers fit.
- **Stress test:** segmented ring and tile grid charts, on a scale and by
  category, from 100 to 1,000,000 cells.
- **Fuzz test:** the two new charts, for 25 chart types.

## 0.3.0

### Added

- `GanttChart`: one row per task, with a bar from its start to its end on a
  time axis, tasks grouped under headings, progress drawn as the solid part
  of a bar (and written after it with `show-values`), milestones as
  diamonds, filled once reached, and an optional line at `now`. It zooms and
  pans along time like the time series, scrolls rows that don't fit and lays
  out only the rows in view. Tasks are `ChartGanttTask`s.
- `BulletChart`: rows of a bar for a value and a mark for its target, over
  shaded bands such as poor, OK and good. A compact alternative to gauges
  for dashboards. Each row has its own axis, or all share one with
  `shared-axis`; `vertical: true` turns the rows into columns. Bands without
  a color are three steps of gray that suit light and dark mode. Each value
  is written at the end of its row (`show-values`), the tooltip shows the
  distance to the target, `below-target-color` colors the rows that fall
  short, and a value beyond a fixed axis ends in an arrow.
- `CandlestickChart`: open, high, low and close per period on a time axis,
  as candles or, with `ohlc: true`, OHLC bars. Rising and falling periods
  have their own colors (`increase-color`, `decrease-color`). By default the
  candles sit side by side, skipping nights, weekends and holidays
  (`skip-gaps`); `skip-gaps: false` gives a linear time axis. It zooms and
  pans like `TimeSeriesChart`, and merges candles closer than a few pixels:
  into hours, days, weeks, months, quarters or years in local time, or with
  `skip-gaps` into a few candles each, so large datasets stay fast.
  `ChartCandle.volume` and `show-volume` draw volume bars below the candles,
  and `show-last-price` a line at the last close. Hovering shows a crosshair
  and a tooltip with the change from the close before and the lines' values.
  `datasets` draws lines over the candles, and `ChartEngine.moving-average`
  (or `slintplot::candle::moving_average`) computes a moving average for
  one. `candle-clicked(first, last)` and `hovered-last` give the candles a
  merged candle holds. The tooltip texts are translated into the 14
  languages (`ChartLocale.open`, `high`, `low`, `close`, `volume`).
- `RadialBarChart`: one ring per bar, the first outermost, each filled along
  its arc in proportion to its value over a faint gray track, like activity
  rings. A full ring is `max-value`, or the largest value. The rings cover
  `sweep` degrees (270 by default) from `start-angle`, with rounded ends,
  and each label sits in the opening before its ring. Values are written
  past the end of each bar, or inside it when it is nearly full. Bars take
  the theme's colors, or one `color`; they grow in and move to new values.
  Hovering a ring or its label fades the others and shows a tooltip; the
  arrow keys, Home, End and Enter work, and `bar-clicked` reports clicks.
  Bars are `ChartSlice`s, as in pie charts.
- `MatrixChart`: rows by columns of circles whose area is proportional to
  the value, like `HeatmapChart`'s `x-labels`, `y-labels` and `values`. Zero
  draws nothing, and a missing value nothing too, or a short dash with
  `show-empty`, to tell the two apart; negative values are hollow, or filled in
  `negative-color` with `color-mode: sign`. The circles take one theme
  color, a theme color per row or column, or a color scale over a second
  measure (`color-values`), diverging at zero. A size legend shows round
  values at the chart's scale; `show-values` writes the values in or under
  the circles, `show-totals` adds row and column totals, and `x-labels-top`
  puts the column labels above. Hovering lights up the cell's row and
  column; the arrow keys move from cell to cell, and `cell-clicked(row,
  column)` reports clicks. The circles grow in and resize when the values
  change. Cells smaller than a few pixels merge into blocks, so a 500 by
  500 matrix stays fast.
- `ChartTheme.increase-color` and `decrease-color`: a green and a red for
  rises and falls that suit the palette (a light and a dark blue in the mono
  palette). Candlestick and waterfall charts use them.
- `ChartLocale` texts for the new charts: `open`, `high`, `low`, `close` and
  `volume` for candlesticks, `target`, `of-target` and `off-scale` for bullet
  charts, and `up-to` for bands without a label, in the 14 languages.

### Changed

- Waterfall charts take their rise and fall colors from the theme, so they
  follow the palette. The default palette's colors are the same as before.
- The calendar heatmap uses Slint's `content-x` and `content-width`, so
  building with SlintPlot no longer warns about the deprecated `viewport-x`
  and `viewport-width`.

### Breaking changes

- Slint: `ChartLocale` has new fields. Rust code that builds one needs
  `..Default::default()`.

### Fixed

- A time series with `fill: true` lost its fill when it had more than two
  points per pixel, for example a week of 15-minute values on a phone. The
  decimated line now keeps its area, like the line chart.

### Example apps

- **Gallery:** samples of the five new charts. Clicking a chart writes what
  was clicked, with its value, at the bottom right. The code button opens the
  sample as a small app that runs as it is: tabs with `ui/app.slint`, with
  the data the chart shows, `src/main.rs`, `build.rs`, `Cargo.toml` and how
  to run it, each with a Copy button. Clicking a task in the Gantt sample
  opens a dialog that changes its name, group, dates and progress. The pie,
  doughnut and polar area samples' slices are product categories instead of
  color names that didn't match their colors.
- **Stress test:** built with this release.

## 0.2.2

### Fixed

Charts no longer panic, hang or run out of memory on unusual data or
settings. A new fuzz app (`examples/fuzz`) fills all 18 chart types with
random data, including NaN, infinities, huge and tiny values, empty and flat
datasets and extreme settings, and found these:

- A heatmap with a huge `levels` allocated gigabytes. Levels are now 1 to 20.
- Time series with times far outside the calendar, logarithmic axes reaching
  infinity, and aligned second axes near the end of the f64 range looped
  forever. Times more than about 300,000 years from 1970 get no ticks, and tick
  loops are bounded.
- NaN or huge values gave bars, steps and highlights sizes the renderer
  cannot draw, which panicked. Bars are cut at the edge of the view, and
  border radius and width fit the bar.
- Lines, areas and bands far outside the view took minutes to draw. They are
  cut where they leave the plot.
- A flat range of huge values, such as every value 1e30, had no width.
- NaN in a bubble radius, line tension, pie, sunburst or gauge cutout, gauge
  sweep or radar value no longer gives NaN shapes.

### Changed

- With `fixed-range` or `y2-fixed-range`, values outside the range are
  clipped at the plot's edge instead of drawn over the axes.
- The hover highlight, markers and tooltip show only for points inside the
  view.

### Documentation

- Data from other threads: a channel and a `slint::Timer` feeding a
  `StreamModel`, and `upgrade_in_event_loop`.
- How decimation works, and what each chart does with missing and invalid
  values.
- Memory over time: a 10-minute soak test of all charts kept the heap level
  (870 KB to 874 KB over 7,760 rounds).

## 0.2.1

- Bubble charts leave room for the bubbles at the ends of both axes. A
  bubble at the smallest or largest value used to overlap the axis.
- Heatmaps size the row labels for the widest one. Labels of the same
  length, such as weekday names, were measured from the narrowest, so "Mon"
  and "Wed" were cut off.
- The website's chart screenshots are rendered from the code examples next to
  them (`slintplot-docshots`), and every example lists all the chart's
  properties, with the ones at their default value commented out.

- The README follows the website: getting started in four steps, a table of
  every chart type with links to its documentation, and links to the
  documentation, the web gallery and the pricing at the top.
- The license sections of the README and the example apps' READMEs list the
  three licenses with links to their texts, and the commercial plans, early
  bird and perpetual fallback, as in COMMERCIAL.md.

## 0.2.0

### Added

- **A second value axis.** Datasets with `right-axis: true` are plotted
  against a value axis on the right, in line, bar, scatter, bubble and time
  series charts, with its own `y2-title`, `y2-value-prefix`, `y2-value-suffix`,
  `y2-value-decimals`, `y2-fixed-range`, `y2-min-value`, `y2-max-value`,
  `y2-logarithmic` and `y2-begin-at-zero`. Its ticks line up with the left
  axis' grid. Each axis shows only while a visible dataset uses it, like
  Chart.js' `display: 'auto'`.
- **Languages.** `ChartTheme.language` writes numbers, percentages, dates,
  month and weekday names in 14 languages: English, Chinese, Danish, Dutch,
  Finnish, French, German, Italian, Japanese, Norwegian, Polish, Portuguese,
  Spanish and Swedish. The charts' few built-in texts, such as "No data",
  "Total" and "Less" and "More" under a calendar heatmap, are translated too.
  `ChartLocale` sets your own conventions and texts; `ChartTexts` has the
  texts and the percent sign for your own labels.
- Rust: `slintplot::locale::Locale`, and locale-aware `time::time_ticks_in`,
  `time::format_datetime_in`, `heatmap::calendar_in` and
  `heatmap::weekday_labels`.
- Scatter and bubble tooltips separate coordinates with "; " in languages
  with a decimal comma.

### Changed

- **Streaming is fast at any data size.** Time series read only the visible
  points, found with a binary search, so a live view of the latest values
  updates in about 7 ms whether 1,000 or 10,000,000 values are kept (it took
  265 ms at 1,000,000).
- Charts fit small screens better: value axes stay close to the data, stacked
  bar labels fit their bars, and horizontal bar charts and heatmaps lay out
  for phones.

### Breaking changes

- Rust: `layout::Dataset` has new fields `right_axis` and `points`, which is
  now a `Points` (owned values, or read on demand from a model) instead of a
  `Vec`. `ticks::Format` has a `locale`, and the pie and radar layout
  requests take one. `layout::AxisRequest` and `layout::LayoutRequest` have
  fields for the second axis.
- Slint: `ChartLocale` and the `ChartEngine` request structs have new fields.
  Rust code that builds them needs `..Default::default()`.
- Time series points must be sorted by `x`. Appending newer points keeps them
  sorted; scatter and bubble charts still take points in any order.

### Example apps

- **Gallery:** a new design, with the logo, a theme picker with each theme's
  colors, documentation and GitHub links and a dark mode button; a window
  frame of its own on Windows and Linux and a see-through title bar on macOS.
  New samples: two value axes, live data, and languages with flags.
- **Stress test:** streams up to 8 datasets at 10 to 100,000 samples a
  second, into a window from 1,000 to 10,000,000 values or everything
  received, from random walk, sine, square or noise signals with optional
  spikes, and stops by itself if the chart falls more than 3 seconds behind.
  Drawing options, a reset button and the gallery's design.
  `--bench-stream` measures the largest window that updates at 60 and 30
  frames per second, and prints the machine it ran on.
- The gallery's `--docs <dir>` renders the website's screenshots.

## 0.1.2

- The gallery runs in the browser at [slintplot.dev/demo](https://slintplot.dev/demo/).

## 0.1.1

- Licensed under the GPLv3, the PolyForm Noncommercial License or a
  commercial license.

## 0.1.0

- First release.
