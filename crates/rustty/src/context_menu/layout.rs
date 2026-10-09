//! Placement du menu dans la fenêtre, test de clic et dessin en chrome.

use rustty_render::{CellMetrics, Chrome, ChromeQuad, ChromeText, Palette, PixelRect};
use unicode_width::UnicodeWidthStr;

use super::{ContextMenu, ENTRIES, MenuEntry};

/// Pixels entre le bord du menu et sa première ou dernière entrée.
pub const MENU_PADDING: u32 = 4;
/// Pixels au-dessus et au-dessous du libellé d'une entrée.
const ITEM_PADDING: u32 = 3;
const SEPARATOR_HEIGHT: u32 = 9;
/// Cellules entre le libellé le plus long et les raccourcis.
const SHORTCUT_GAP: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuLayout {
    pub rect: PixelRect,
    /// Index dans `ENTRIES` et rectangle de chaque rangée.
    pub rows: Vec<(usize, PixelRect)>,
}

/// Ce que fait un clic quand le menu est ouvert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuClick {
    /// Exécuter l'entrée d'index donné dans `ENTRIES`.
    Run(usize),
    /// Clic dans le menu sans entrée active : le menu reste ouvert.
    Keep,
    /// Clic hors du menu : il se ferme.
    Close,
}

impl MenuLayout {
    /// La décision pour un clic (`left` : bouton gauche) au point `(x, y)`.
    pub fn click(&self, menu: &ContextMenu, left: bool, x: f64, y: f64) -> MenuClick {
        let r = self.rect;
        let inside = x >= f64::from(r.x)
            && x < f64::from(r.x + r.width)
            && y >= f64::from(r.y)
            && y < f64::from(r.y + r.height);
        match (inside, left.then(|| self.hit(menu, x, y)).flatten()) {
            (_, Some(i)) => MenuClick::Run(i),
            (true, None) => MenuClick::Keep,
            (false, None) => MenuClick::Close,
        }
    }

    /// L'entrée cliquable sous le point (ni séparateur, ni entrée grisée).
    pub fn hit(&self, menu: &ContextMenu, x: f64, y: f64) -> Option<usize> {
        self.rows
            .iter()
            .find(|(_, r)| {
                x >= f64::from(r.x)
                    && x < f64::from(r.x + r.width)
                    && y >= f64::from(r.y)
                    && y < f64::from(r.y + r.height)
            })
            .and_then(|(i, _)| match ENTRIES[*i] {
                MenuEntry::Item(item) if menu.enabled(item) => Some(*i),
                _ => None,
            })
    }
}

/// Place le menu au point du clic, décalé pour rester dans la fenêtre.
pub fn layout(
    menu: &ContextMenu,
    viewport: (u32, u32),
    metrics: CellMetrics,
    shortcuts: &[Option<String>],
) -> MenuLayout {
    let cw = metrics.width.max(1);
    let label_cells = ENTRIES
        .iter()
        .filter_map(|e| match e {
            MenuEntry::Item(item) => Some(item.label(menu.zoomed).width() as u32),
            MenuEntry::Separator => None,
        })
        .max()
        .unwrap_or(0);
    let shortcut_cells = shortcuts.iter().flatten().map(|s| s.width() as u32).max();
    let cells = 1 + label_cells + shortcut_cells.map_or(0, |c| SHORTCUT_GAP + c) + 1;
    let width = cells * cw;
    let item_height = metrics.height + 2 * ITEM_PADDING;
    let heights: Vec<u32> = ENTRIES
        .iter()
        .map(|e| match e {
            MenuEntry::Item(_) => item_height,
            MenuEntry::Separator => SEPARATOR_HEIGHT,
        })
        .collect();
    let height = 2 * MENU_PADDING + heights.iter().sum::<u32>();
    let (vw, vh) = viewport;
    let x = menu.x.min(vw.saturating_sub(width));
    let y = if menu.y + height > vh {
        menu.y.saturating_sub(height)
    } else {
        menu.y
    };
    let mut rows = Vec::with_capacity(ENTRIES.len());
    let mut row_y = y + MENU_PADDING;
    for (i, h) in heights.into_iter().enumerate() {
        rows.push((i, PixelRect::new(x, row_y, width, h)));
        row_y += h;
    }
    MenuLayout {
        rect: PixelRect::new(x, y, width, height),
        rows,
    }
}

/// Le menu en quads et textes : cadre, fond, survol, séparateurs, libellés
/// (grisés quand l'entrée est inactive) et raccourcis alignés à droite.
pub fn menu_chrome(
    menu: &ContextMenu,
    layout: &MenuLayout,
    shortcuts: &[Option<String>],
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
                chrome.texts.push(ChromeText {
                    x: row.x + cw,
                    y: text_y,
                    text: item.label(menu.zoomed).to_string(),
                    color: if enabled {
                        palette.foreground
                    } else {
                        palette.ansi[8]
                    },
                });
                if let Some(Some(hint)) = shortcuts.get(*i) {
                    let hint_cells = hint.width() as u32;
                    chrome.texts.push(ChromeText {
                        x: (row.x + row.width).saturating_sub((1 + hint_cells) * cw),
                        y: text_y,
                        text: hint.clone(),
                        color: palette.ansi[7],
                    });
                }
            }
        }
    }
    chrome
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
