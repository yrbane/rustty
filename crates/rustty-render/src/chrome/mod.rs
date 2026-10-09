//! La barre d'onglets : disposition en pixels, rectangles cliquables, et sa
//! traduction en quads et textes pour le renderer. Pur, sans GPU.

mod title_fit;

use rustty_config::{Rgb, Tabs};

use title_fit::{fit_title, title_cells};

use crate::color::{Palette, Rgba, readable_on};
use crate::font::CellMetrics;
use crate::frame::{Chrome, ChromeQuad, ChromeText, PixelRect};

/// Marge verticale par défaut de la barre d'onglets, en pixels.
pub const TAB_BAR_PADDING: u32 = 2;
/// Assombrissement d'un onglet inactif qui porte une couleur d'accent.
pub const INACTIVE_ACCENT_DIM: f32 = 0.6;
/// Cellules occupées par le bouton de fermeture : espace, ◖, ✕, ◗.
const CLOSE_CELLS: u32 = 4;
const NEW_TAB_CELLS: u32 = 3;
/// Cellules de titre minimales pour tronquer un onglet au lieu de l'omettre.
const MIN_TRUNCATED_TITLE: u32 = 3;

#[derive(Clone, Debug, PartialEq)]
pub struct TabBarStyle {
    pub background: Rgba,
    pub active_background: Rgba,
    pub inactive_background: Rgba,
    pub active_foreground: Rgba,
    pub inactive_foreground: Rgba,
    pub close_foreground: Rgba,
    pub close_background: Rgba,
    pub close_hover_foreground: Rgba,
    pub close_hover_background: Rgba,
    pub show_close_button: bool,
    /// Cellules vides de chaque côté du titre.
    pub padding_horizontal: u32,
    /// Pixels au-dessus et au-dessous du titre.
    pub padding_vertical: u32,
    /// Pixels entre deux onglets.
    pub spacing: u32,
    /// Texte sur un fond d'accent sombre.
    pub light_text: Rgba,
    /// Texte sur un fond d'accent clair.
    pub dark_text: Rgba,
}

impl TabBarStyle {
    pub fn from_config(palette: &Palette, tabs: &Tabs) -> Self {
        let pick =
            |configured: Option<Rgb>, derived: Rgba| configured.map_or(derived, Rgba::from_rgb);
        let colors = &tabs.colors;
        let close = &tabs.close_button_style;
        // Les deux couleurs de texte de la palette, rangées par luminance : un
        // thème clair a un premier plan sombre.
        let (light_text, dark_text) =
            if palette.foreground.luminance() >= palette.background.luminance() {
                (palette.foreground, palette.background)
            } else {
                (palette.background, palette.foreground)
            };
        // Texte d'un onglet : la couleur configurée, sinon celle de la palette,
        // sauf si le fond a été choisi sans texte — alors une couleur lisible dessus.
        let text = |fg: Option<Rgb>, bg: Option<Rgb>, derived: Rgba| match (fg, bg) {
            (Some(fg), _) => Rgba::from_rgb(fg),
            (None, Some(bg)) => readable_on(Rgba::from_rgb(bg), light_text, dark_text),
            (None, None) => derived,
        };
        Self {
            background: pick(colors.bar_background, palette.ansi[8].dim(0.5)),
            active_background: pick(colors.active_background, palette.background),
            inactive_background: pick(colors.inactive_background, palette.ansi[0]),
            active_foreground: text(
                colors.active_foreground,
                colors.active_background,
                palette.foreground,
            ),
            inactive_foreground: text(
                colors.inactive_foreground,
                colors.inactive_background,
                palette.ansi[7],
            ),
            close_foreground: Rgba::from_rgb(close.foreground),
            close_background: Rgba::from_rgb(close.background),
            close_hover_foreground: Rgba::from_rgb(close.hover_foreground),
            close_hover_background: Rgba::from_rgb(close.hover_background),
            show_close_button: tabs.close_button,
            padding_horizontal: tabs.padding_horizontal,
            padding_vertical: tabs.padding_vertical,
            spacing: tabs.spacing,
            light_text,
            dark_text,
        }
    }

    /// Fond et texte d'un onglet, avec son accent éventuel.
    fn tab_colors(&self, spec: &TabSpec) -> (Rgba, Rgba) {
        match (spec.accent, spec.active) {
            (Some(accent), active) => {
                let bg = if active {
                    accent
                } else {
                    accent.dim(INACTIVE_ACCENT_DIM)
                };
                (bg, readable_on(bg, self.light_text, self.dark_text))
            }
            (None, true) => (self.active_background, self.active_foreground),
            (None, false) => (self.inactive_background, self.inactive_foreground),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabSpec<'a> {
    pub title: &'a str,
    pub active: bool,
    /// Couleur propre à l'onglet (aléatoire ou choisie) ; `None` = style.
    pub accent: Option<Rgba>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverTarget {
    None,
    Tab(usize),
    CloseButton(usize),
    NewTabButton,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabRect {
    pub rect: PixelRect,
    pub close: Option<PixelRect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabBarLayout {
    pub bar: PixelRect,
    pub tabs: Vec<TabRect>,
    pub new_tab: PixelRect,
}

impl TabBarLayout {
    pub fn hit_test(&self, x: u32, y: u32) -> HoverTarget {
        let inside = |r: PixelRect| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height;
        if !inside(self.bar) {
            return HoverTarget::None;
        }
        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.close.is_some_and(inside) {
                return HoverTarget::CloseButton(i);
            }
            if inside(tab.rect) {
                return HoverTarget::Tab(i);
            }
        }
        if self.new_tab.width > 0 && inside(self.new_tab) {
            return HoverTarget::NewTabButton;
        }
        HoverTarget::None
    }
}

pub fn tab_bar_height(metrics: CellMetrics, style: &TabBarStyle) -> u32 {
    metrics.height + 2 * style.padding_vertical
}

pub fn layout_tab_bar(
    viewport_width: u32,
    y: u32,
    tabs: &[TabSpec],
    style: &TabBarStyle,
    metrics: CellMetrics,
) -> TabBarLayout {
    let (cw, bar_h) = (metrics.width.max(1), tab_bar_height(metrics, style));
    let close_cells = if style.show_close_button {
        CLOSE_CELLS
    } else {
        0
    };
    let pad = style.padding_horizontal;
    let mut x = 0u32;
    let mut out = Vec::new();
    for tab in tabs {
        let max_title = (viewport_width / cw).saturating_sub(2 * pad + close_cells);
        let mut title = title_cells(tab.title).min(max_title);
        let mut width = (2 * pad + title + close_cells) * cw;
        if width == 0 {
            break;
        }
        if x.saturating_add(width) > viewport_width {
            // Dernier onglet visible : tronqué à la place restante s'il
            // garde au moins MIN_TRUNCATED_TITLE cellules de titre.
            let free_cells = viewport_width.saturating_sub(x) / cw;
            let room = free_cells.saturating_sub(2 * pad + close_cells);
            if room < MIN_TRUNCATED_TITLE {
                break;
            }
            title = room;
            width = (2 * pad + title + close_cells) * cw;
        }
        let close = style.show_close_button.then(|| {
            PixelRect::new(
                x + (pad + title) * cw,
                y + style.padding_vertical,
                close_cells * cw,
                metrics.height,
            )
        });
        out.push(TabRect {
            rect: PixelRect::new(x, y, width, bar_h),
            close,
        });
        x += width + style.spacing;
    }
    let new_w = NEW_TAB_CELLS * cw;
    let new_tab = if x.saturating_add(new_w) <= viewport_width {
        PixelRect::new(x, y, new_w, bar_h)
    } else {
        PixelRect::new(x.min(viewport_width), y, 0, bar_h)
    };
    TabBarLayout {
        bar: PixelRect::new(0, y, viewport_width, bar_h),
        tabs: out,
        new_tab,
    }
}

pub fn tab_bar_chrome(
    layout: &TabBarLayout,
    tabs: &[TabSpec],
    style: &TabBarStyle,
    metrics: CellMetrics,
    hover: HoverTarget,
) -> Chrome {
    let cw = metrics.width;
    let mut chrome = Chrome {
        quads: vec![ChromeQuad {
            rect: layout.bar,
            color: style.background,
        }],
        texts: Vec::new(),
    };
    let text_y = layout.bar.y + style.padding_vertical;
    let pad = style.padding_horizontal;
    for (i, (tab_rect, spec)) in layout.tabs.iter().zip(tabs).enumerate() {
        let (bg, fg) = style.tab_colors(spec);
        chrome.quads.push(ChromeQuad {
            rect: tab_rect.rect,
            color: bg,
        });
        let close_cells = if tab_rect.close.is_some() {
            CLOSE_CELLS
        } else {
            0
        };
        let max_title = (tab_rect.rect.width / cw).saturating_sub(2 * pad + close_cells);
        chrome.texts.push(ChromeText {
            x: tab_rect.rect.x + pad * cw,
            y: text_y,
            text: fit_title(spec.title, max_title),
            color: fg,
        });
        if let Some(close) = tab_rect.close {
            let hovered = hover == HoverTarget::CloseButton(i);
            let (button_bg, button_fg) = if hovered {
                (style.close_hover_background, style.close_hover_foreground)
            } else {
                (style.close_background, style.close_foreground)
            };
            let x0 = close.x + cw;
            chrome.texts.push(ChromeText {
                x: x0,
                y: close.y,
                text: "\u{E0B6}".into(),
                color: button_bg,
            });
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
            chrome.texts.push(ChromeText {
                x: x0 + 2 * cw,
                y: close.y,
                text: "\u{E0B4}".into(),
                color: button_bg,
            });
        }
    }
    if layout.new_tab.width > 0 {
        chrome.texts.push(ChromeText {
            x: layout.new_tab.x + cw,
            y: text_y,
            text: "+".into(),
            color: style.inactive_foreground,
        });
    }
    chrome
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
