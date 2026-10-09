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

impl MenuLayout {
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
mod tests {
    use super::*;
    use crate::context_menu::MenuItem;
    use crate::tab::TermId;
    use rustty_config::Colors;

    fn metrics() -> CellMetrics {
        CellMetrics {
            width: 10,
            height: 20,
            baseline: 16,
            underline_y: 18,
            underline_thickness: 1,
            strike_y: 10,
        }
    }

    fn menu(x: u32, y: u32) -> ContextMenu {
        ContextMenu {
            term: TermId(1),
            x,
            y,
            can_copy: true,
            zoomed: false,
            hovered: None,
        }
    }

    fn no_shortcuts() -> Vec<Option<String>> {
        vec![None; ENTRIES.len()]
    }

    fn item_index(item: MenuItem) -> usize {
        ENTRIES
            .iter()
            .position(|e| *e == MenuEntry::Item(item))
            .unwrap()
    }

    #[test]
    fn menu_opens_at_the_click() {
        let l = layout(&menu(100, 50), (800, 600), metrics(), &no_shortcuts());
        assert_eq!((l.rect.x, l.rect.y), (100, 50));
        assert_eq!(l.rows.len(), ENTRIES.len());
        assert!(
            l.rows
                .windows(2)
                .all(|w| w[0].1.y + w[0].1.height == w[1].1.y),
            "rangées jointives"
        );
        assert_eq!(
            l.rows.last().unwrap().1.y + l.rows.last().unwrap().1.height + MENU_PADDING,
            l.rect.y + l.rect.height
        );
    }

    #[test]
    fn menu_stays_inside_the_window() {
        let l = layout(&menu(790, 590), (800, 600), metrics(), &no_shortcuts());
        assert!(
            l.rect.x + l.rect.width <= 800 && l.rect.y + l.rect.height <= 600,
            "{:?}",
            l.rect
        );
        let tiny = layout(&menu(5, 5), (40, 30), metrics(), &no_shortcuts());
        assert_eq!(
            (tiny.rect.x, tiny.rect.y),
            (0, 0),
            "plus grand que la fenêtre : collé en haut à gauche"
        );
    }

    #[test]
    fn hit_finds_items_by_row() {
        let m = menu(100, 50);
        let l = layout(&m, (800, 600), metrics(), &no_shortcuts());
        let close = item_index(MenuItem::ClosePane);
        let row = l.rows[close].1;
        assert_eq!(
            l.hit(&m, f64::from(row.x + 5), f64::from(row.y + 2)),
            Some(close)
        );
        assert_eq!(l.hit(&m, 10.0, 10.0), None, "hors du menu");
    }

    #[test]
    fn separators_and_disabled_items_are_not_clickable() {
        let mut m = menu(100, 50);
        m.can_copy = false;
        let l = layout(&m, (800, 600), metrics(), &no_shortcuts());
        let sep = ENTRIES
            .iter()
            .position(|e| *e == MenuEntry::Separator)
            .unwrap();
        let r = l.rows[sep].1;
        assert_eq!(
            l.hit(&m, f64::from(r.x + 5), f64::from(r.y + r.height / 2)),
            None
        );
        let copy = l.rows[item_index(MenuItem::Copy)].1;
        assert_eq!(
            l.hit(&m, f64::from(copy.x + 5), f64::from(copy.y + 2)),
            None,
            "copier grisé sans sélection"
        );
    }

    #[test]
    fn chrome_highlights_the_hovered_item_and_greys_disabled_ones() {
        let palette = Palette::from_config(&Colors::default(), false);
        let mut m = menu(100, 50);
        m.can_copy = false;
        let close = item_index(MenuItem::ClosePane);
        m.hovered = Some(close);
        let mut shortcuts = no_shortcuts();
        shortcuts[close] = Some("ctrl+shift+w".into());
        let l = layout(&m, (800, 600), metrics(), &shortcuts);
        let c = menu_chrome(&m, &l, &shortcuts, &palette, metrics());
        assert!(
            c.quads
                .iter()
                .any(|q| q.rect == l.rows[close].1 && q.color == palette.selection),
            "survol"
        );
        let copy_text = c.texts.iter().find(|t| t.text == "Copier").unwrap();
        assert_eq!(copy_text.color, palette.ansi[8], "grisé");
        let close_text = c
            .texts
            .iter()
            .find(|t| t.text == "Fermer le panneau")
            .unwrap();
        assert_eq!(close_text.color, palette.foreground);
        let hint = c.texts.iter().find(|t| t.text == "ctrl+shift+w").unwrap();
        assert!(hint.x > close_text.x, "raccourci à droite du libellé");
        assert!(
            hint.x + 12 * 10 <= l.rect.x + l.rect.width,
            "raccourci dans le menu"
        );
    }
}
