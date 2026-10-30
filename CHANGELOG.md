# Changelog

## Unreleased

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

### Fixed

- A time series with `fill: true` lost its fill when it had more than two
  points per pixel, for example a week of 15-minute values on a phone. The
  decimated line now keeps its area, like the line chart.

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
