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

`ui/gallery.slint` has a working example of every chart. The code button
(`</>`) at the top right shows the sample's code, taken from that file at build
time, with a Copy button. Clicking a chart shows the click callback and what
was clicked, such as `slice-clicked: Safari, 19`, at the bottom right.

On Linux, Slint needs the fontconfig and xkbcommon development packages first
(`libfontconfig-dev` and `libxkbcommon-dev` on Debian and Ubuntu).

Documentation: [slintplot.dev](https://slintplot.dev). The gallery also
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
