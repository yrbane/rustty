#!/bin/bash
# Régénère toutes les icônes de rustty depuis assets/icon.svg (res/rustty.svg
# mis au carré). Outils : rsvg-convert, ImageMagick (magick), Python + Pillow.
set -euo pipefail
cd "$(dirname "$0")/.."
src=assets/icon.svg
out=assets/icons
mkdir -p "$out"
for size in 16 32 48 64 128 256 512; do
    rsvg-convert --width "$size" --height "$size" --keep-aspect-ratio "$src" -o "$out/rustty-$size.png"
done
magick "$out"/rustty-{16,32,48,64,128,256}.png "$out/rustty.ico"
python3 -c "
from PIL import Image
Image.open('$out/rustty-512.png').save('$out/rustty.icns')
"
ls -la "$out"
