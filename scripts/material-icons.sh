#!/bin/bash
# Régénère la police d'icônes de la barre de titre : Material Symbols Rounded
# (Apache 2.0) instanciée en poids 400, sans remplissage, puis réduite aux
# sept glyphes utilisés. Outils : python3 (venv jetable avec fonttools).
set -euo pipefail
cd "$(dirname "$0")/.."

base='https://github.com/google/material-design-icons/raw/master'
font_url="$base/variablefont/MaterialSymbolsRounded%5BFILL%2CGRAD%2Copsz%2Cwght%5D.ttf"
codepoints_url="$base/variablefont/MaterialSymbolsRounded%5BFILL%2CGRAD%2Copsz%2Cwght%5D.codepoints"
license_url="$base/LICENSE"
names="settings palette folder_open remove crop_square filter_none close"
out=crates/rustty-render/assets/MaterialSymbolsRounded-subset.ttf
license=crates/rustty-render/assets/LICENSE-MaterialSymbols

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

python3 -m venv "$work/venv"
"$work/venv/bin/pip" install --quiet fonttools

"$work/venv/bin/python" - "$work" "$font_url" "$codepoints_url" "$license_url" <<'EOF'
import sys, urllib.request
work, font, codepoints, license = sys.argv[1:]
for url, name in ((font, "full.ttf"), (codepoints, "codepoints"), (license, "LICENSE")):
    urllib.request.urlretrieve(url, f"{work}/{name}")
EOF

# Codepoints des sept icônes, depuis le fichier « nom hexa » du dépôt.
unicodes=""
for name in $names; do
    hex=$(awk -v n="$name" '$1 == n { print $2 }' "$work/codepoints")
    [ -n "$hex" ] || { echo "icône inconnue : $name" >&2; exit 1; }
    echo "$name U+$hex"
    unicodes="$unicodes,U+$hex"
done
unicodes=${unicodes#,}

"$work/venv/bin/fonttools" varLib.instancer --quiet \
    -o "$work/static.ttf" "$work/full.ttf" wght=400 FILL=0 GRAD=0 opsz=20
"$work/venv/bin/pyftsubset" "$work/static.ttf" \
    --unicodes="$unicodes" --no-layout-closure --output-file="$out"
cp "$work/LICENSE" "$license"
ls -la "$out" "$license"
