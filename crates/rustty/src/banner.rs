//! Bandeau d'une ligne en bas de fenêtre : erreur de configuration,
//! confirmation de fermeture, shell terminé. Pur : produit du Chrome.

use rustty_render::{
    CellMetrics, Chrome, ChromeQuad, ChromeText, Palette, PixelRect, Rgba, TAB_BAR_PADDING,
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerKind {
    Error,
    Info,
    Confirm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banner {
    pub kind: BannerKind,
    pub text: String,
}

impl Banner {
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            kind: BannerKind::Error,
            text: text.into(),
        }
    }

    pub fn info(text: impl Into<String>) -> Self {
        Self {
            kind: BannerKind::Info,
            text: text.into(),
        }
    }

    pub fn confirm(text: impl Into<String>) -> Self {
        Self {
            kind: BannerKind::Confirm,
            text: text.into(),
        }
    }
}

pub fn banner_height(metrics: CellMetrics) -> u32 {
    metrics.height + 2 * TAB_BAR_PADDING
}

fn band_color(kind: BannerKind, palette: &Palette) -> Rgba {
    match kind {
        BannerKind::Error => palette.ansi[1],
        BannerKind::Confirm => palette.ansi[3],
        BannerKind::Info => palette.ansi[4],
    }
}

/// Tronque `text` à `max_cells` colonnes avec `…` ; vide si rien ne tient.
fn fit(text: &str, max_cells: usize) -> String {
    if text.width() <= max_cells {
        return text.to_string();
    }
    if max_cells == 0 {
        return String::new();
    }
    let mut out = String::new();
    for ch in text.chars() {
        if out.width() + 2 > max_cells {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

pub fn banner_chrome(
    banner: &Banner,
    viewport_width: u32,
    y: u32,
    metrics: CellMetrics,
    palette: &Palette,
) -> Chrome {
    let cw = metrics.width.max(1);
    let band = PixelRect::new(0, y, viewport_width, banner_height(metrics));
    let mut chrome = Chrome {
        quads: vec![ChromeQuad {
            rect: band,
            color: band_color(banner.kind, palette),
        }],
        texts: Vec::new(),
    };
    let max_cells = (viewport_width / cw).saturating_sub(2) as usize;
    let text = fit(&banner.text, max_cells);
    if !text.is_empty() {
        chrome.texts.push(ChromeText {
            x: cw,
            y: y + TAB_BAR_PADDING,
            text,
            color: palette.background,
        });
    }
    chrome
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn palette() -> Palette {
        Palette::from_config(&Colors::default(), false)
    }

    #[test]
    fn height_matches_the_tab_bar() {
        assert_eq!(
            banner_height(metrics()),
            rustty_render::tab_bar_height(metrics())
        );
    }

    #[test]
    fn chrome_has_a_band_and_the_text() {
        let b = Banner::error("config : ligne 3");
        let c = banner_chrome(&b, 400, 576, metrics(), &palette());
        assert_eq!(c.quads.len(), 1);
        assert_eq!(c.quads[0].rect, PixelRect::new(0, 576, 400, 24));
        assert_eq!(c.quads[0].color, palette().ansi[1]);
        assert_eq!(c.texts.len(), 1);
        assert_eq!((c.texts[0].x, c.texts[0].y), (10, 578));
        assert_eq!(c.texts[0].text, "config : ligne 3");
        assert_eq!(c.texts[0].color, palette().background);
    }

    #[test]
    fn kinds_pick_their_color() {
        let p = palette();
        assert_eq!(
            banner_chrome(&Banner::info("i"), 100, 0, metrics(), &p).quads[0].color,
            p.ansi[4]
        );
        assert_eq!(
            banner_chrome(&Banner::confirm("c"), 100, 0, metrics(), &p).quads[0].color,
            p.ansi[3]
        );
    }

    #[test]
    fn long_text_is_truncated_with_an_ellipsis() {
        let b = Banner::info("un message beaucoup trop long pour tenir");
        let c = banner_chrome(&b, 100, 0, metrics(), &palette());
        assert_eq!(
            c.texts[0].text, "un mess…",
            "8 cellules disponibles sur 10, dont une pour …"
        );
        let tiny = banner_chrome(&b, 15, 0, metrics(), &palette());
        assert!(tiny.texts.is_empty(), "pas la place d'un seul caractère");
    }
}
