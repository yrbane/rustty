//! Dessin du menu en chrome : cadre, survol, séparateurs, libellés et
//! raccourcis, coupés à la largeur disponible.

use rustty_render::{CellMetrics, Chrome, ChromeQuad, ChromeText, Palette, PixelRect};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::layout::{ITEM_PADDING, MenuLayout};
use super::{ContextMenu, ENTRIES, MenuEntry};

/// Coupe `text` à `cells` cellules, avec `…` quand il faut couper.
pub fn truncate_to_cells(text: &str, cells: usize) -> String {
    if text.width() <= cells {
        return text.to_string();
    }
    if cells == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > cells - 1 {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push('…');
    out
}

/// Le menu en quads et textes : cadre, fond, survol, séparateurs, libellés
/// (grisés quand l'entrée est inactive) et raccourcis alignés à droite.
pub fn menu_chrome(
    menu: &ContextMenu,
    layout: &MenuLayout,
    palette: &Palette,
    metrics: CellMetrics,
) -> Chrome {
    let cw = metrics.width.max(1);
    let r = layout.rect;
    let mut chrome = Chrome::default();
    chrome.quads.push(ChromeQuad {
        rect: r,
        color: palette.ansi[8],
    });
    chrome.quads.push(ChromeQuad {
        rect: PixelRect::new(
            r.x + 1,
            r.y + 1,
            r.width.saturating_sub(2),
            r.height.saturating_sub(2),
        ),
        color: palette.background,
    });
    for (i, row) in &layout.rows {
        match ENTRIES[*i] {
            MenuEntry::Separator => chrome.quads.push(ChromeQuad {
                rect: PixelRect::new(
                    row.x + cw,
                    row.y + row.height / 2,
                    row.width.saturating_sub(2 * cw),
                    1,
                ),
                color: palette.ansi[8],
            }),
            MenuEntry::Item(item) => {
                let enabled = menu.enabled(item);
                if enabled && menu.hovered == Some(*i) {
                    chrome.quads.push(ChromeQuad {
                        rect: *row,
                        color: palette.selection,
                    });
                }
                let text_y = row.y + ITEM_PADDING;
                // Cellules utilisables entre les marges d'une cellule.
                let avail = (row.width / cw).saturating_sub(2) as usize;
                let label = truncate_to_cells(item.label(menu.zoomed), avail);
                let label_cells = label.width();
                chrome.texts.push(ChromeText {
                    x: row.x + cw,
                    y: text_y,
                    text: label,
                    color: if enabled {
                        palette.foreground
                    } else {
                        palette.ansi[8]
                    },
                });
                // Le raccourci cède la place au libellé : il prend ce qui reste.
                let room = avail.saturating_sub(label_cells + 1);
                if let Some(Some(hint)) = menu.shortcuts.get(*i)
                    && room > 0
                {
                    let hint = truncate_to_cells(hint, room);
                    let hint_cells = hint.width() as u32;
                    chrome.texts.push(ChromeText {
                        x: (row.x + row.width).saturating_sub((1 + hint_cells) * cw),
                        y: text_y,
                        text: hint,
                        color: palette.ansi[7],
                    });
                }
            }
        }
    }
    chrome
}
