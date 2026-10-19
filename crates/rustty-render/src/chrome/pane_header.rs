//! Le bandeau de panneau : une rangée de titre au-dessus de chaque panneau
//! d'un onglet divisé.

use super::*;

/// Assombrissement du fond d'un bandeau dont le panneau n'a pas le focus.
pub(super) const UNFOCUSED_DIM: f32 = 0.55;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneHeaderSpec<'a> {
    /// Rectangle du bandeau, une rangée de cellules de base de haut.
    pub rect: PixelRect,
    pub title: &'a str,
    /// Couleur du panneau ; le texte s'en déduit pour rester lisible.
    pub color: Rgba,
    pub focused: bool,
    pub show_close: bool,
    pub close_hovered: bool,
}

/// Bouton ✕ du bandeau, collé au bord droit (même largeur que sur un onglet).
pub fn pane_header_close_rect(rect: PixelRect, metrics: CellMetrics) -> PixelRect {
    let width = (CLOSE_CELLS * metrics.width).min(rect.width);
    PixelRect::new(
        rect.x + rect.width - width,
        rect.y,
        width,
        metrics.height.min(rect.height),
    )
}

pub fn pane_header_chrome(
    headers: &[PaneHeaderSpec],
    style: &TabBarStyle,
    metrics: CellMetrics,
) -> Chrome {
    let cw = metrics.width;
    let mut chrome = Chrome::default();
    for header in headers {
        let bg = if header.focused {
            header.color
        } else {
            header.color.dim(UNFOCUSED_DIM)
        };
        let fg = readable_on(bg, style.light_text, style.dark_text);
        chrome.quads.push(ChromeQuad {
            rect: header.rect,
            color: bg,
        });
        // Une cellule de marge à gauche ; le ✕ éventuel est réservé à droite.
        let reserved = if header.show_close { CLOSE_CELLS } else { 1 };
        let max_title = (header.rect.width / cw).saturating_sub(1 + reserved);
        chrome.texts.push(ChromeText {
            x: header.rect.x + cw,
            y: header.rect.y,
            text: fit_title(header.title, max_title),
            color: fg,
        });
        if header.show_close {
            push_close_button(&mut chrome, header, style, metrics);
        }
    }
    chrome
}

/// Pastille ✕, dessinée comme celle des onglets.
fn push_close_button(
    chrome: &mut Chrome,
    header: &PaneHeaderSpec,
    style: &TabBarStyle,
    metrics: CellMetrics,
) {
    let cw = metrics.width;
    let close = pane_header_close_rect(header.rect, metrics);
    let (button_bg, button_fg) = if header.close_hovered {
        (style.close_hover_background, style.close_hover_foreground)
    } else {
        (style.close_background, style.close_foreground)
    };
    let x0 = close.x + cw;
    let cap = |x: u32, glyph: &str| ChromeText {
        x,
        y: close.y,
        text: glyph.into(),
        color: button_bg,
    };
    chrome.texts.push(cap(x0, "\u{E0B6}"));
    chrome.quads.push(ChromeQuad {
        rect: PixelRect::new(x0 + cw, close.y, cw, close.height),
        color: button_bg,
    });
    chrome.texts.push(ChromeText {
        x: x0 + cw,
        y: close.y,
        text: "✕".into(),
        color: button_fg,
    });
    chrome.texts.push(cap(x0 + 2 * cw, "\u{E0B4}"));
}

#[cfg(test)]
#[path = "pane_header_tests.rs"]
mod tests;
