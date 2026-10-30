//! L'infobulle de correction : état, placement près du mot et dessin en
//! chrome, dans le style du menu du clic droit. Pur.

use rustty_render::{CellMetrics, Chrome, ChromeQuad, ChromeText, Palette, PixelRect};
use unicode_width::UnicodeWidthStr;

use super::run::UnderlinedRun;
use crate::context_menu::truncate_to_cells;
use crate::tab::TermId;

/// Pixels entre le cadre et la première ou dernière rangée.
const PADDING: u32 = 4;
/// Pixels au-dessus et au-dessous du texte d'une rangée.
const ROW_PADDING: u32 = 3;
/// Largeur maximale du contenu, en cellules.
const MAX_CELLS: usize = 60;
/// Largeur minimale du contenu, en cellules.
const MIN_CELLS: usize = 12;

/// L'infobulle ouverte sur un mot d'un panneau.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpellPopup {
    pub term: TermId,
    pub run: UnderlinedRun,
    pub message: String,
    pub replacements: Vec<String>,
    /// Remplacement sous la souris.
    pub hovered: Option<usize>,
    /// Rectangle du mot à l'écran, en pixels : l'infobulle s'y accroche.
    pub anchor: PixelRect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupLayout {
    pub rect: PixelRect,
    /// Rangée du message.
    pub message: PixelRect,
    /// Une rangée par remplacement, dans l'ordre.
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

    /// Le remplacement sous le point.
    pub fn hit(&self, x: f64, y: f64) -> Option<usize> {
        self.rows.iter().position(|r| inside(*r, x, y))
    }
}

/// Place l'infobulle au-dessus du mot, alignée sur sa première colonne, ou
/// en dessous quand la place manque ; décalée pour rester dans la fenêtre.
pub fn layout(popup: &SpellPopup, viewport: (u32, u32), metrics: CellMetrics) -> PopupLayout {
    let cw = metrics.width.max(1);
    let row_height = metrics.height + 2 * ROW_PADDING;
    let widest = popup
        .replacements
        .iter()
        .map(|r| r.width())
        .chain(std::iter::once(popup.message.width()))
        .max()
        .unwrap_or(0);
    let cells = widest.clamp(MIN_CELLS, MAX_CELLS) as u32;
    let width = (cells + 2) * cw;
    let rows = popup.replacements.len() as u32;
    let height = 2 * PADDING + row_height * (rows + 1);
    let (vw, vh) = viewport;
    let x = popup.anchor.x.min(vw.saturating_sub(width));
    let y = if popup.anchor.y >= height {
        popup.anchor.y - height
    } else {
        (popup.anchor.y + popup.anchor.height).min(vh.saturating_sub(height))
    };
    let rect = PixelRect::new(x, y, width, height);
    let row_at = |i: u32| PixelRect::new(x, y + PADDING + i * row_height, width, row_height);
    PopupLayout {
        rect,
        message: row_at(0),
        rows: (0..rows).map(|i| row_at(i + 1)).collect(),
    }
}

/// L'infobulle en quads et textes : cadre, fond, message grisé, remplacements
/// (le survolé sur fond de sélection), coupés à la largeur disponible.
pub fn chrome(
    popup: &SpellPopup,
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
        text: truncate_to_cells(&popup.message, avail),
        color: palette.ansi[8],
    });
    for (i, (row, replacement)) in layout.rows.iter().zip(&popup.replacements).enumerate() {
        if popup.hovered == Some(i) {
            chrome.quads.push(ChromeQuad {
                rect: *row,
                color: palette.selection,
            });
        }
        chrome.texts.push(ChromeText {
            x: row.x + cw,
            y: row.y + ROW_PADDING,
            text: truncate_to_cells(replacement, avail),
            color: palette.foreground,
        });
    }
    chrome
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn popup(anchor: PixelRect) -> SpellPopup {
        SpellPopup {
            term: TermId(1),
            run: UnderlinedRun {
                row: 5,
                start: 17,
                end: 23,
                word: "poulle".into(),
                context: "ma poulle".into(),
                offset: 3,
                length: 6,
            },
            message: "Faute de frappe possible trouvée.".into(),
            replacements: vec!["poule".into(), "poulie".into()],
            hovered: None,
            anchor,
        }
    }

    #[test]
    fn popup_sits_above_the_word_aligned_on_its_first_column() {
        let l = layout(
            &popup(PixelRect::new(170, 300, 60, 20)),
            (800, 600),
            metrics(),
        );
        // 33 cellules de message + 2 de marge ; 3 rangées de 26 px + 2 × 4 px.
        assert_eq!(l.rect, PixelRect::new(170, 300 - 86, 350, 86));
        assert_eq!(l.message, PixelRect::new(170, 218, 350, 26));
        assert_eq!(
            l.rows,
            vec![
                PixelRect::new(170, 244, 350, 26),
                PixelRect::new(170, 270, 350, 26)
            ]
        );
    }

    #[test]
    fn popup_goes_below_when_there_is_no_room_above_and_stays_in_the_window() {
        let l = layout(
            &popup(PixelRect::new(700, 10, 60, 20)),
            (800, 600),
            metrics(),
        );
        assert_eq!(l.rect.y, 30, "sous le mot");
        assert_eq!(l.rect.x, 800 - 350, "décalée à gauche");
    }

    #[test]
    fn hit_names_the_replacement_under_the_point() {
        let p = popup(PixelRect::new(170, 300, 60, 20));
        let l = layout(&p, (800, 600), metrics());
        assert_eq!(l.hit(200.0, 250.0), Some(0));
        assert_eq!(l.hit(200.0, 280.0), Some(1));
        assert_eq!(l.hit(200.0, 220.0), None, "le message n'est pas cliquable");
        assert!(l.contains(200.0, 220.0));
        assert!(!l.contains(200.0, 350.0));
    }

    #[test]
    fn chrome_draws_frame_message_and_highlighted_replacement() {
        let mut p = popup(PixelRect::new(170, 300, 60, 20));
        p.hovered = Some(1);
        let l = layout(&p, (800, 600), metrics());
        let palette = Palette::from_config(&rustty_config::Colors::default(), false);
        let c = chrome(&p, &l, &palette, metrics());
        assert_eq!(c.quads.len(), 3, "cadre, fond, survol");
        assert_eq!(c.quads[2].rect, l.rows[1]);
        let texts: Vec<&str> = c.texts.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["Faute de frappe possible trouvée.", "poule", "poulie"]
        );
    }
}
