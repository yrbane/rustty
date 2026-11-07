//! L'infobulle de correction : état, placement près du mot et dessin en
//! chrome, dans le style du menu du clic droit. Pur.

use rustty_render::{CellMetrics, Chrome, ChromeQuad, ChromeText, Palette, PixelRect};
use unicode_width::UnicodeWidthStr;

use super::run::UnderlinedRun;
use crate::context_menu::truncate_to_cells;
use crate::tab::TermId;
pub use crate::tooltip::PopupLayout;
use crate::tooltip::{ROW_PADDING, Tooltip};

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

impl SpellPopup {
    fn tooltip(&self) -> Tooltip {
        Tooltip {
            anchor: self.anchor,
            message: self.message.clone(),
        }
    }
}

/// Place l'infobulle au-dessus du mot, alignée sur sa première colonne, ou
/// en dessous quand la place manque ; décalée pour rester dans la fenêtre.
pub fn layout(popup: &SpellPopup, viewport: (u32, u32), metrics: CellMetrics) -> PopupLayout {
    let widest = popup
        .replacements
        .iter()
        .map(|r| r.width())
        .chain(std::iter::once(popup.message.width()))
        .max()
        .unwrap_or(0);
    crate::tooltip::layout(
        &popup.tooltip(),
        popup.replacements.len() as u32,
        widest,
        viewport,
        metrics,
    )
}

/// L'infobulle en quads et textes : cadre, fond et message par `tooltip`,
/// puis les remplacements (le survolé sur fond de sélection), coupés à la
/// largeur disponible.
pub fn chrome(
    popup: &SpellPopup,
    layout: &PopupLayout,
    palette: &Palette,
    metrics: CellMetrics,
) -> Chrome {
    let cw = metrics.width.max(1);
    let avail = (layout.rect.width / cw).saturating_sub(2) as usize;
    let mut chrome = crate::tooltip::chrome(&popup.tooltip(), layout, palette, metrics);
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
