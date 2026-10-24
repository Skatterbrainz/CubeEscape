#!/bin/sh
# Rebuild the square, transparent icon set in assets/icons/ from the original
# artwork, assets/cube_escape_icon.png. Needs ImageMagick (`convert`).
#
#     sh assets/make_icons.sh
#
# The original has a flat near-white background. Flood-filling inward from the
# four corners makes it transparent; the cube's dark outline stops the fill, so
# the glass faces inside stay opaque. The result is trimmed and centred on a
# square canvas, then scaled to each size.
set -eu
cd "$(dirname "$0")"
SRC=cube_escape_icon.png
OUT=icons
BG='srgb(250,250,250)'
W=$(identify -format %w "$SRC")
H=$(identify -format %h "$SRC")
mkdir -p "$OUT"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

convert "$SRC" -alpha set -fuzz 38% -fill none \
    -floodfill +0+0 "$BG" \
    -floodfill +$((W - 1))+0 "$BG" \
    -floodfill +0+$((H - 1)) "$BG" \
    -floodfill +$((W - 1))+$((H - 1)) "$BG" \
    -trim +repage -background none -gravity center -extent 640x640 "$TMP/square.png"

for size in 256 64 32 16; do
    case $size in
        16|32) sharpen="-unsharp 0x0.6" ;;
        *)     sharpen="" ;;
    esac
    # shellcheck disable=SC2086
    convert "$TMP/square.png" -filter Lanczos -resize ${size}x${size} $sharpen \
        -define png:exclude-chunk=all "$OUT/cube_escape_$size.png"
done
echo "wrote $OUT/cube_escape_{256,64,32,16}.png"
