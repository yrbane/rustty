//! Infobulle générique : un message (et des rangées optionnelles pour qui
//! en veut, comme le correcteur) placé près d'une ancre, dessiné en chrome
//! dans le style du menu du clic droit. Pur.

use rustty_render::{CellMetrics, Chrome, ChromeQuad, ChromeText, Palette, PixelRect};

use crate::context_menu::truncate_to_cells;

/// Pixels entre le cadre et la première ou dernière rangée.
pub const PADDING: u32 = 4;
/// Pixels au-dessus et au-dessous du texte d'une rangée.
pub const ROW_PADDING: u32 = 3;
/// Largeur maximale du contenu, en cellules.
pub const MAX_CELLS: usize = 60;
/// Largeur minimale du contenu, en cellules.
pub const MIN_CELLS: usize = 12;

/// Un message ancré sur un rectangle de la fenêtre.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tooltip {
    pub anchor: PixelRect,
    pub message: String,
}

/// Rectangles de l'infobulle : cadre, rangée du message, rangées suivantes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupLayout {
    pub rect: PixelRect,
    pub message: PixelRect,
    pub rows: Vec<PixelRect>,
}

fn inside(r: PixelRect, x: f64, y: f64) -> bool {
    x >= f64::from(r.x)
        && x < f64::from(r.x + r.width)
        && y >= f64::from(r.y)
        && y < f64::from(r.y + r.height)
}

impl PopupLayout {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        inside(self.rect, x, y)
    }

    pub fn hit(&self, x: f64, y: f64) -> Option<usize> {
        self.rows.iter().position(|r| inside(*r, x, y))
    }
}

/// Place l'infobulle au-dessus de l'ancre, alignée sur son bord gauche, ou
/// en dessous quand la place manque ; décalée pour rester dans la fenêtre.
/// `rows` rangées suivent le message ; `widest_cells` est la largeur du
/// contenu le plus large (message compris), bornée à `MIN_CELLS..=MAX_CELLS`.
pub fn layout(
    tip: &Tooltip,
    rows: u32,
    widest_cells: usize,
    viewport: (u32, u32),
    metrics: CellMetrics,
) -> PopupLayout {
    let cw = metrics.width.max(1);
    let row_height = metrics.height + 2 * ROW_PADDING;
    let cells = widest_cells.clamp(MIN_CELLS, MAX_CELLS) as u32;
    let width = (cells + 2) * cw;
    let height = 2 * PADDING + row_height * (rows + 1);
    let (vw, vh) = viewport;
    let x = tip.anchor.x.min(vw.saturating_sub(width));
    let y = if tip.anchor.y >= height {
        tip.anchor.y - height
    } else {
        (tip.anchor.y + tip.anchor.height).min(vh.saturating_sub(height))
    };
    let rect = PixelRect::new(x, y, width, height);
    let row_at = |i: u32| PixelRect::new(x, y + PADDING + i * row_height, width, row_height);
    PopupLayout {
        rect,
        message: row_at(0),
        rows: (0..rows).map(|i| row_at(i + 1)).collect(),
    }
}

/// Cadre, fond et message grisé ; les rangées sont à la charge de l'appelant.
pub fn chrome(
    tip: &Tooltip,
    layout: &PopupLayout,
    palette: &Palette,
    metrics: CellMetrics,
) -> Chrome {
    let cw = metrics.width.max(1);
    let r = layout.rect;
    let avail = (r.width / cw).saturating_sub(2) as usize;
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
    chrome.texts.push(ChromeText {
        x: layout.message.x + cw,
        y: layout.message.y + ROW_PADDING,
        text: truncate_to_cells(&tip.message, avail),
        color: palette.ansi[8],
    });
    chrome
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustty_config::Colors;
    use rustty_render::Palette;
    use unicode_width::UnicodeWidthStr;

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

    #[test]
    fn a_tooltip_without_rows_is_one_message_row_placed_under_the_anchor_when_room_above_is_missing()
     {
        let tip = Tooltip {
            anchor: PixelRect::new(10, 0, 8, 16),
            message: "https://x".into(),
        };
        let l = layout(&tip, 0, tip.message.width(), (800, 600), metrics());
        assert_eq!(l.rows.len(), 0);
        assert_eq!(l.rect.y, 16, "sous l'ancre : pas de place au-dessus");
        assert_eq!(
            l.rect.height,
            2 * PADDING + metrics().height + 2 * ROW_PADDING
        );
        let c = chrome(
            &tip,
            &l,
            &Palette::from_config(&Colors::default(), false),
            metrics(),
        );
        assert_eq!(c.texts.len(), 1);
        assert_eq!(c.texts[0].text, "https://x");
    }
}
