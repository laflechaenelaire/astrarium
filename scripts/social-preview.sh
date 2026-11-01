#!/usr/bin/env bash
# Optional artwork helper: ImageMagick, librsvg and fontconfig are needed.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
font=$(fc-match monospace -f '%{file}')
stage=$(mktemp -d "${TMPDIR:-/tmp}/astrarium-art.XXXXXX")
trap 'rm -rf -- "$stage"' EXIT
rsvg-convert -w 72 -h 72 assets/logo.svg -o "$stage/logo.png"
magick assets/preview.png -resize 760x418 "$stage/sky.png"
magick -size 1280x640 xc:'#0b1516' \
  -stroke '#334845' -fill none -draw 'rectangle 24,24 1255,615' \
  "$stage/logo.png" -geometry +52+53 -composite \
  -font "$font" -stroke none -fill '#cacebb' -pointsize 50 \
  -annotate +142+105 'Astrarium' \
  -fill '#b8a25e' -pointsize 25 \
  -annotate +56+210 'A small atlas' \
  -annotate +56+246 'of an immense sky.' \
  -fill '#cacebb' -pointsize 20 \
  -annotate +56+326 'Stars, planets,' \
  -annotate +56+358 'Sun & Moon.' \
  -fill '#6f8581' -pointsize 18 \
  -annotate +56+434 'RUST  /  OFFLINE' \
  -annotate +56+464 'ENGLISH / ESPAÑOL' \
  "$stage/sky.png" -geometry +464+135 -composite \
  -fill '#b8a25e' -pointsize 17 \
  -annotate +56+585 'github.com/laflechaenelaire/astrarium' \
  -strip assets/social-preview.png
