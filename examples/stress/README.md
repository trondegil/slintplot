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

Documentation: [slintplot.dev](https://slintplot.dev). The gallery app
runs [in your browser](https://slintplot.dev/demo/).

## License

SlintPlot is available under your choice of three licenses:
[the GPLv3](https://github.com/trondegil/slintplot/blob/main/LICENSE-GPL-3.0.txt),
[the PolyForm Noncommercial License 1.0.0](https://github.com/trondegil/slintplot/blob/main/LICENSE-POLYFORM-NC.md)
(free for any noncommercial use) or
[a commercial license](https://github.com/trondegil/slintplot/blob/main/COMMERCIAL.md),
for commercial use without the GPLv3's conditions, such as in closed-source
software you sell or ship. Commercial licenses are per developer, per year,
from €99, and half price for the first two years when bought by 31 December
2027: see [Pricing](https://slintplot.dev/#pricing). The full terms are in
[LICENSE.md](https://github.com/trondegil/slintplot/blob/main/LICENSE.md).

SlintPlot is an independent project. It is not affiliated with, endorsed by or sponsored by SixtyFPS GmbH or the Slint project. Slint and the Slint logo are trademarks of SixtyFPS GmbH.
