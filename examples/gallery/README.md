# slintplot-gallery

The sample browser for [SlintPlot](https://slintplot.dev), native interactive
charts for [Slint](https://slint.dev): every chart type, with buttons to
randomize, add and remove data, a theme selector and dark mode.

```sh
cargo install slintplot-gallery
slintplot-gallery                    # open the app
slintplot-gallery --snapshot <dir>   # render every sample to PNG, without a window
```

![The gallery app](https://slintplot.dev/screenshots/light.png)

`ui/gallery.slint` has a working example of every chart.

On Linux, Slint needs the fontconfig and xkbcommon development packages first
(`libfontconfig-dev` and `libxkbcommon-dev` on Debian and Ubuntu).

SlintPlot is available under your choice of the GPLv3, the PolyForm
Noncommercial License 1.0.0 (free for any noncommercial use) or a commercial
license, for commercial use without the GPLv3's conditions
(license@slintplot.dev). See LICENSE.md.
Documentation: [slintplot.dev](https://slintplot.dev).

SlintPlot is an independent project. It is not affiliated with, endorsed by or sponsored by SixtyFPS GmbH or the Slint project. Slint and the Slint logo are trademarks of SixtyFPS GmbH.
