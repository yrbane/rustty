//! Placement du menu dans la fenêtre et test de clic.

use rustty_render::{CellMetrics, PixelRect};
use unicode_width::UnicodeWidthStr;

use super::{ContextMenu, ENTRIES, MenuEntry};

/// Pixels entre le bord du menu et sa première ou dernière entrée.
pub const MENU_PADDING: u32 = 4;
/// Pixels au-dessus et au-dessous du libellé d'une entrée.
pub(super) const ITEM_PADDING: u32 = 3;
const SEPARATOR_HEIGHT: u32 = 9;
/// Cellules entre le libellé le plus long et les raccourcis.
pub(super) const SHORTCUT_GAP: u32 = 3;

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
    /// Le point est dans le cadre du menu.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let r = self.rect;
        x >= f64::from(r.x)
            && x < f64::from(r.x + r.width)
            && y >= f64::from(r.y)
            && y < f64::from(r.y + r.height)
    }

    pub fn click(&self, menu: &ContextMenu, left: bool, x: f64, y: f64) -> MenuClick {
        let inside = self.contains(x, y);
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
pub fn layout(menu: &ContextMenu, viewport: (u32, u32), metrics: CellMetrics) -> MenuLayout {
    let cw = metrics.width.max(1);
    let label_cells = ENTRIES
        .iter()
        .filter_map(|e| match e {
            MenuEntry::Item(item) => Some(item.label(menu.zoomed).width() as u32),
            MenuEntry::Separator => None,
        })
        .max()
        .unwrap_or(0);
    let shortcut_cells = menu
        .shortcuts
        .iter()
        .flatten()
        .map(|s| s.width() as u32)
        .max();
    let cells = 1 + label_cells + shortcut_cells.map_or(0, |c| SHORTCUT_GAP + c) + 1;
    // Jamais plus large que la fenêtre ; `menu_chrome` coupe les textes.
    let width = (cells * cw).min(viewport.0);
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

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
