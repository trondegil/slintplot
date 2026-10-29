# Changelog

## 0.2.1

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
