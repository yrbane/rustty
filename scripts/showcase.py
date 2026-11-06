#!/usr/bin/env python3
"""Compose la carte vitrine de rustty (1200×630, WebP) à partir du logo et de
la capture de fenêtre produite par le renderer (`tests/golden/window.png`).

    python3 -I scripts/showcase.py ~/Dev/yrbane.github.io/shots/rustty.webp

Dépendance : Pillow (Python 3.9 ou plus). La police est DejaVu Sans Mono
(système, sinon celle embarquée dans rustty-render) ; les emojis de la
tagline demandent Noto Color Emoji (paquet noto-fonts-emoji sous Linux) et
sont simplement omis sans elle.
"""

from __future__ import annotations

import sys
from functools import lru_cache
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

SIZE = (1200, 630)
MARGIN = 40
COLUMN_X = 430
BACKGROUND = (30, 30, 46)
TEXT = (216, 216, 216)
ACCENT = (255, 140, 60)
DIM = (150, 150, 165)
TITLE_PX = 56
TAGLINE_PX = 19
CAPTION_PX = 13
LOGO_WIDTH = 330
CAPTURE_SCALE = 1.2

FONT_CANDIDATES = [
    Path("/usr/share/fonts/TTF/DejaVuSansMono.ttf"),
    Path("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"),
]
BOLD_CANDIDATES = [
    Path("/usr/share/fonts/TTF/DejaVuSansMono-Bold.ttf"),
    Path("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"),
]
EMOJI_CANDIDATES = [
    Path("/usr/share/fonts/noto/NotoColorEmoji.ttf"),
    Path("/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf"),
    Path("/usr/share/fonts/google-noto-emoji/NotoColorEmoji.ttf"),
    Path.home() / ".local/share/fonts/NotoColorEmoji.ttf",
    Path.home() / "Library/Fonts/NotoColorEmoji.ttf",
]
# Noto Color Emoji n'existe qu'en 109 px (bitmaps CBDT) : rendu puis réduit.
EMOJI_NATIVE_PX = 109

TAGLINE = [
    "Terminal GPU en Rust, inspiré de kitty — wgpu · onglets à",
    "bouton ✕ · splits · barre de titre maison 🪟 · nuancier 🎨",
    "images 🖼️ · glisser-déposer à la Terminator 🖱️",
]
CAPTION = "Fenêtre composée hors écran par le renderer et vérifiée en CI sur Linux, macOS et Windows."


def font(root: Path, px: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    candidates = (BOLD_CANDIDATES if bold else []) + FONT_CANDIDATES
    candidates.append(root / "crates/rustty-render/assets/DejaVuSansMono.ttf")
    for path in candidates:
        if path.exists():
            return ImageFont.truetype(str(path), px)
    raise FileNotFoundError("DejaVu Sans Mono introuvable")


@lru_cache(maxsize=1)
def emoji_font() -> ImageFont.FreeTypeFont | None:
    for path in EMOJI_CANDIDATES:
        if path.exists():
            return ImageFont.truetype(str(path), EMOJI_NATIVE_PX)
    return None


def is_emoji(ch: str) -> bool:
    return ord(ch) >= 0x1F000 or ch == "\ufe0f"


def draw_line(card: Image.Image, xy: tuple[int, int], text: str, body: ImageFont.FreeTypeFont, fill) -> None:
    """Texte en DejaVu, emojis en Noto Color Emoji (réduits à la hauteur de
    ligne) ; sans police emoji, ils sont simplement omis."""
    draw = ImageDraw.Draw(card)
    emoji = emoji_font()
    x, y = xy
    px = body.size
    i = 0
    while i < len(text):
        if is_emoji(text[i]):
            j = i + 1
            while j < len(text) and text[j] == "\ufe0f":
                j += 1
            if emoji is not None:
                glyph = Image.new("RGBA", (EMOJI_NATIVE_PX + 30, EMOJI_NATIVE_PX + 30), (0, 0, 0, 0))
                ImageDraw.Draw(glyph).text((0, 0), text[i:j], font=emoji, embedded_color=True)
                box = glyph.getbbox()
                if box:
                    glyph = glyph.crop(box)
                    side = int(px * 1.15)
                    glyph = glyph.resize((side, int(glyph.height * side / glyph.width)), Image.LANCZOS)
                    card.paste(glyph, (x, y + (px - glyph.height) // 2 + 2), glyph)
                    x += side
            i = j
        else:
            j = i
            while j < len(text) and not is_emoji(text[j]):
                j += 1
            draw.text((x, y), text[i:j], font=body, fill=fill)
            x += int(draw.textlength(text[i:j], font=body))
            i = j


def capture_path(root: Path) -> Path:
    return root / "crates/rustty-render/tests/golden/window.png"


def column_layout(root: Path) -> tuple[int, int, int, int, int]:
    """Haut de la colonne droite (bloc centré verticalement) et boîte de la
    capture : (top, x, y, w, h)."""
    with Image.open(capture_path(root)) as im:
        w, h = im.size
    max_w = SIZE[0] - MARGIN - COLUMN_X
    scale = min(CAPTURE_SCALE, max_w / w)
    sw, sh = int(w * scale), int(h * scale)
    tagline_h = len(TAGLINE) * (TAGLINE_PX + 9)
    block = TITLE_PX + 18 + tagline_h + 18 + sh + 14 + CAPTION_PX
    top = max(MARGIN, (SIZE[1] - block) // 2)
    y = top + TITLE_PX + 18 + tagline_h + 18
    return top, COLUMN_X, y, sw, sh


def capture_box(root: Path) -> tuple[int, int, int, int]:
    """Position et taille de la capture dans la carte : colonne droite, sous
    la tagline, agrandie sans dépasser la marge."""
    return column_layout(root)[1:]


def compose(root: Path, out: Path) -> None:
    card = Image.new("RGB", SIZE, BACKGROUND)
    draw = ImageDraw.Draw(card)
    logo = Image.open(root / "assets/logo.png").convert("RGBA")
    logo = logo.resize((LOGO_WIDTH, int(logo.height * LOGO_WIDTH / logo.width)), Image.LANCZOS)
    card.paste(logo, (MARGIN + 10, (SIZE[1] - logo.height) // 2), logo)

    top, x, y, w, h = column_layout(root)
    draw.text((COLUMN_X, top), "rustty", font=font(root, TITLE_PX, bold=True), fill=ACCENT)
    line_y = top + TITLE_PX + 18
    body = font(root, TAGLINE_PX)
    for line in TAGLINE:
        draw_line(card, (COLUMN_X, line_y), line, body, TEXT)
        line_y += TAGLINE_PX + 9

    with Image.open(capture_path(root)) as im:
        shot = im.convert("RGB").resize((w, h), Image.LANCZOS)
    draw.rectangle((x - 1, y - 1, x + w, y + h), outline=(70, 70, 90))
    card.paste(shot, (x, y))
    draw.text((COLUMN_X, y + h + 14), CAPTION, font=font(root, CAPTION_PX), fill=DIM)
    out.parent.mkdir(parents=True, exist_ok=True)
    card.save(out, "WEBP", quality=90)


def separator_row(image: Image.Image) -> int | None:
    """Première rangée dont le pixel de gauche est noir (le séparateur sous
    la barre de titre), `None` s'il n'y en a pas."""
    return next(
        (yy for yy in range(image.height) if image.getpixel((0, yy))[:3] == (0, 0, 0)),
        None,
    )


def contains_capture(root: Path, card: Image.Image) -> bool:
    """La rangée du séparateur noir de la capture se retrouve, mise à
    l'échelle, à la position où `compose` l'a posée."""
    x, y, w, h = capture_box(root)
    with Image.open(capture_path(root)) as im:
        ratio = h / im.height
        separator = separator_row(im)
    if separator is None:
        return False
    row = y + int(separator * ratio)
    pixels = [card.getpixel((x + i, row))[:3] for i in range(4, w - 4, max(1, w // 32))]
    return all(sum(p) < 60 for p in pixels)


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    root = Path(__file__).resolve().parent.parent
    try:
        compose(root, Path(argv[1]))
    except FileNotFoundError as err:
        print(f"fichier introuvable : {err.filename} (lancer cargo test -p rustty-render d'abord ?)", file=sys.stderr)
        return 1
    print(argv[1])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
