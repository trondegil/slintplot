#!/bin/sh
# Writes a WebP copy next to every PNG the website shows: lossless for the
# screenshots, high-quality lossy for the logos. The website workflow runs this;
# run it yourself to preview the docs folder locally. Needs cwebp.
set -eu
cd "$(dirname "$0")"
for png in screenshots/*.png screenshots/charts/*.png; do
  cwebp -quiet -lossless -z 9 "$png" -o "${png%.png}.webp"
done
for png in assets/slintplot-logo.png assets/slintplot-logo-dark.png assets/MadeWithSlint-logo-whitebg.png; do
  cwebp -quiet -q 90 -alpha_q 100 "$png" -o "${png%.png}.webp"
done
