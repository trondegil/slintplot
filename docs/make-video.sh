#!/bin/sh
# The animation of the gallery at the top of the website and the README, in
# light and dark mode: renders the frames with `slintplot-gallery --video`,
# then writes an MP4 for the website with its first frame as the poster, and
# an animated WebP for the README, as GitHub doesn't play videos from other
# sites. Needs ffmpeg (with libx264) and img2webp.
set -eu
cd "$(dirname "$0")/.."
frames=target/video
out=docs/screenshots
rm -rf "$frames"
cargo run --release -p slintplot-gallery -- --video "$frames"
for mode in light dark; do
  suffix=""; [ "$mode" = dark ] && suffix="-dark"
  ffmpeg -loglevel error -y -framerate 30 -i "$frames/$mode/%04d.png" \
    -c:v libx264 -preset veryslow -tune animation -crf 28 -pix_fmt yuv420p \
    -movflags +faststart -an "$out/tour$suffix.mp4"
  ffmpeg -loglevel error -y -i "$frames/$mode/0001.png" -compression_level 100 "$out/tour$suffix.png"
  # Every other frame, 15 per second, which is smooth enough for the README.
  # shellcheck disable=SC2046
  img2webp -loop 0 -lossy -q 60 -m 6 -mixed -d 67 \
    $(ls "$frames/$mode"/*.png | awk 'NR % 2 == 1') -o "$out/tour-anim$suffix.webp" >/dev/null 2>&1
done
ls -l "$out"/tour*
