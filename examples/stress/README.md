# slintplot-stress

The stress test for [SlintPlot](https://slintplot.dev), native interactive
charts for [Slint](https://slint.dev): line, bar, time series and scatter
charts with up to 8 × 1,000,000 points; waterfall, dumbbell, funnel and
sunburst charts with up to 10,000 items, and a gauge; 60 Hz streaming, zoom,
decimation and live performance numbers.

```sh
cargo install slintplot-stress
slintplot-stress           # open the app
slintplot-stress --bench   # print headless frame timings
```

On Linux, Slint needs the fontconfig and xkbcommon development packages first
(`libfontconfig-dev` and `libxkbcommon-dev` on Debian and Ubuntu).

SlintPlot is free for open-source GPLv3 applications and for any
noncommercial use (PolyForm Noncommercial 1.0.0). For commercial use in
other applications, email license@slintplot.dev. See LICENSE.md.
Documentation: [slintplot.dev](https://slintplot.dev).

SlintPlot is an independent project. It is not affiliated with, endorsed by or sponsored by SixtyFPS GmbH or the Slint project. Slint and the Slint logo are trademarks of SixtyFPS GmbH.
