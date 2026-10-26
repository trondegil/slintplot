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

SlintPlot is free for open-source GPLv3 applications and for any
noncommercial use (PolyForm Noncommercial 1.0.0). For commercial use in
other applications, email license@slintplot.dev. See LICENSE.md.
Documentation: [slintplot.dev](https://slintplot.dev).
