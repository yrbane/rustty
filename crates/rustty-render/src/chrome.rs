//! La barre d'onglets : disposition en pixels, rectangles cliquables, et sa
//! traduction en quads et textes pour le renderer. Pur, sans GPU.

use rustty_config::{Rgb, Tabs};
use unicode_width::UnicodeWidthStr;

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

fn title_cells(title: &str) -> u32 {
    title.width() as u32
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
        let title = title_cells(tab.title).min(max_title);
        let width = (2 * pad + title + close_cells) * cw;
        if width == 0 || x.saturating_add(width) > viewport_width {
            break;
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

/// Tronque `title` à `max_cells` colonnes, avec `…` si nécessaire.
fn fit_title(title: &str, max_cells: u32) -> String {
    if title_cells(title) <= max_cells {
        return title.to_string();
    }
    let mut out = String::new();
    for ch in title.chars() {
        if title_cells(&out) + 2 > max_cells {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
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
mod tests {
    use super::*;

    #[test]
    fn a_configured_background_without_text_color_gets_readable_text() {
        let colors = TabColors {
            active_background: Some(Rgb::new(0xf0, 0xf0, 0xf0)),
            inactive_background: Some(Rgb::new(0x10, 0x10, 0x10)),
            ..TabColors::default()
        };
        let st = style_with(Tabs {
            colors,
            ..Tabs::default()
        });
        assert_eq!(
            st.active_foreground, st.dark_text,
            "texte sombre sur fond clair"
        );
        assert_eq!(
            st.inactive_foreground, st.light_text,
            "texte clair sur fond sombre"
        );
    }

    #[test]
    fn light_and_dark_text_follow_luminance_even_on_a_light_theme() {
        let light_theme = Colors {
            foreground: Rgb::new(0x20, 0x20, 0x20),
            background: Rgb::new(0xfa, 0xfa, 0xfa),
            ..Colors::default()
        };
        let st =
            TabBarStyle::from_config(&Palette::from_config(&light_theme, false), &Tabs::default());
        assert!(st.light_text.luminance() > st.dark_text.luminance());
    }
    use rustty_config::{Colors, Rgb, TabColors, Tabs};

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

    fn style_with(tabs: Tabs) -> TabBarStyle {
        TabBarStyle::from_config(&palette(), &tabs)
    }

    fn style(show_close: bool) -> TabBarStyle {
        style_with(Tabs {
            close_button: show_close,
            ..Tabs::default()
        })
    }

    fn tabs() -> Vec<TabSpec<'static>> {
        vec![
            TabSpec {
                title: "1: sh",
                active: true,
                accent: None,
            },
            TabSpec {
                title: "2: vim",
                active: false,
                accent: None,
            },
        ]
    }

    #[test]
    fn default_style_keeps_the_previous_geometry() {
        let l = layout_tab_bar(400, 0, &tabs(), &style(true), metrics());
        assert_eq!(l.tabs[0].rect, PixelRect::new(0, 0, 110, 24));
        assert_eq!(l.tabs[0].close.unwrap().x, 60);
        assert_eq!(l.new_tab.x, 230);
    }

    #[test]
    fn horizontal_padding_widens_tabs() {
        let st = style_with(Tabs {
            padding_horizontal: 3,
            ..Tabs::default()
        });
        let l = layout_tab_bar(400, 0, &tabs(), &st, metrics());
        assert_eq!(l.tabs[0].rect.width, 150, "3 + 5 + 4 + 3 cellules");
        assert_eq!(l.tabs[0].close.unwrap().x, 80);
        let c = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::None);
        let title = c.texts.iter().find(|t| t.text == "1: sh").unwrap();
        assert_eq!(title.x, 30);
    }

    #[test]
    fn spacing_separates_tabs() {
        let st = style_with(Tabs {
            spacing: 4,
            ..Tabs::default()
        });
        let l = layout_tab_bar(400, 0, &tabs(), &st, metrics());
        assert_eq!(l.tabs[1].rect.x, 114);
        assert_eq!(l.new_tab.x, 114 + 120 + 4);
        assert_eq!(
            l.hit_test(112, 10),
            HoverTarget::None,
            "l'espace entre onglets n'est pas cliquable"
        );
    }

    #[test]
    fn vertical_padding_sets_the_bar_height() {
        let st = style_with(Tabs {
            padding_vertical: 6,
            ..Tabs::default()
        });
        assert_eq!(tab_bar_height(metrics(), &st), 32);
        let l = layout_tab_bar(400, 0, &tabs(), &st, metrics());
        assert_eq!(l.bar.height, 32);
        assert_eq!(l.tabs[0].close.unwrap().y, 6);
        let c = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::None);
        assert_eq!(c.texts.iter().find(|t| t.text == "1: sh").unwrap().y, 6);
    }

    #[test]
    fn configured_colors_override_the_palette() {
        let colors = TabColors {
            bar_background: Some(Rgb::new(1, 2, 3)),
            active_background: Some(Rgb::new(4, 5, 6)),
            active_foreground: Some(Rgb::new(7, 8, 9)),
            inactive_background: Some(Rgb::new(10, 11, 12)),
            inactive_foreground: Some(Rgb::new(13, 14, 15)),
            random: false,
        };
        let st = style_with(Tabs {
            colors,
            ..Tabs::default()
        });
        assert_eq!(st.background, Rgba::from_rgb(Rgb::new(1, 2, 3)));
        assert_eq!(st.active_background, Rgba::from_rgb(Rgb::new(4, 5, 6)));
        assert_eq!(st.active_foreground, Rgba::from_rgb(Rgb::new(7, 8, 9)));
        assert_eq!(st.inactive_background, Rgba::from_rgb(Rgb::new(10, 11, 12)));
        assert_eq!(st.inactive_foreground, Rgba::from_rgb(Rgb::new(13, 14, 15)));
        let default = style(true);
        assert_eq!(default.active_background, palette().background);
    }

    #[test]
    fn accent_colors_the_tab_and_dims_inactive_ones() {
        let st = style(true);
        let accent = Rgba::from_rgb(Rgb::new(0x89, 0xb4, 0xfa));
        let specs = [
            TabSpec {
                title: "a",
                active: true,
                accent: Some(accent),
            },
            TabSpec {
                title: "b",
                active: false,
                accent: Some(accent),
            },
        ];
        let l = layout_tab_bar(400, 0, &specs, &st, metrics());
        let c = tab_bar_chrome(&l, &specs, &st, metrics(), HoverTarget::None);
        let bg = |i: usize| {
            c.quads
                .iter()
                .find(|q| q.rect == l.tabs[i].rect)
                .unwrap()
                .color
        };
        assert_eq!(bg(0), accent);
        assert_eq!(bg(1), accent.dim(INACTIVE_ACCENT_DIM));
    }

    #[test]
    fn text_contrasts_with_the_tab_color() {
        let st = style(true);
        let light = Rgba::from_rgb(Rgb::new(0xf9, 0xe2, 0xaf));
        let dark = Rgba::from_rgb(Rgb::new(0x31, 0x32, 0x44));
        let specs = [
            TabSpec {
                title: "clair",
                active: true,
                accent: Some(light),
            },
            TabSpec {
                title: "sombre",
                active: true,
                accent: Some(dark),
            },
        ];
        let l = layout_tab_bar(400, 0, &specs, &st, metrics());
        let c = tab_bar_chrome(&l, &specs, &st, metrics(), HoverTarget::None);
        let fg = |t: &str| c.texts.iter().find(|x| x.text == t).unwrap().color;
        assert_eq!(fg("clair"), st.dark_text);
        assert_eq!(fg("sombre"), st.light_text);
    }

    #[test]
    fn huge_paddings_never_overflow() {
        let st = style_with(Tabs {
            padding_horizontal: 8,
            padding_vertical: 32,
            spacing: 64,
            ..Tabs::default()
        });
        let l = layout_tab_bar(100, 0, &tabs(), &st, metrics());
        assert!(l.tabs.is_empty());
        let c = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::None);
        assert_eq!(c.quads.len(), 1, "le fond de barre seulement");
        let tiny = layout_tab_bar(0, 0, &tabs(), &st, metrics());
        assert!(tiny.tabs.is_empty());
    }

    #[test]
    fn height_is_a_cell_plus_padding() {
        assert_eq!(tab_bar_height(metrics(), &style(true)), 24);
    }

    #[test]
    fn tabs_are_laid_out_left_to_right_with_close_buttons() {
        let l = layout_tab_bar(400, 0, &tabs(), &style(true), metrics());
        assert_eq!(l.bar, PixelRect::new(0, 0, 400, 24));
        assert_eq!(l.tabs.len(), 2);
        // « 1: sh » = 5 colonnes + 1 + 1 + 4 (bouton) = 11 cellules = 110 px.
        assert_eq!(l.tabs[0].rect, PixelRect::new(0, 0, 110, 24));
        assert_eq!(
            l.tabs[0].close,
            Some(PixelRect::new(60, 2, 40, 20)),
            "espace, demi-disque, ✕, demi-disque"
        );
        assert_eq!(l.tabs[1].rect.x, 110);
        assert_eq!(l.tabs[1].rect.width, 120);
        assert_eq!(l.new_tab, PixelRect::new(230, 0, 30, 24));
    }

    #[test]
    fn without_close_button_tabs_are_narrower() {
        let l = layout_tab_bar(400, 0, &tabs(), &style(false), metrics());
        assert_eq!(l.tabs[0].rect.width, 70);
        assert!(l.tabs[0].close.is_none());
    }

    #[test]
    fn tabs_that_do_not_fit_are_dropped() {
        let l = layout_tab_bar(150, 0, &tabs(), &style(true), metrics());
        assert_eq!(
            l.tabs.len(),
            1,
            "le second onglet (120 px) ne tient pas après 110 px"
        );
        assert_eq!(l.new_tab.width, 30);
        let tiny = layout_tab_bar(20, 0, &tabs(), &style(true), metrics());
        assert!(tiny.tabs.is_empty());
        assert_eq!(tiny.new_tab.width, 0, "pas de place pour « + »");
    }

    #[test]
    fn hit_test_prefers_close_button_then_tab_then_new_tab() {
        let l = layout_tab_bar(400, 0, &tabs(), &style(true), metrics());
        assert_eq!(l.hit_test(5, 5), HoverTarget::Tab(0));
        assert_eq!(l.hit_test(75, 10), HoverTarget::CloseButton(0));
        assert_eq!(l.hit_test(115, 10), HoverTarget::Tab(1));
        assert_eq!(l.hit_test(240, 10), HoverTarget::NewTabButton);
        assert_eq!(l.hit_test(300, 10), HoverTarget::None);
        assert_eq!(l.hit_test(5, 30), HoverTarget::None, "sous la barre");
    }

    #[test]
    fn chrome_colors_follow_active_state_and_hover() {
        let st = style(true);
        let l = layout_tab_bar(400, 0, &tabs(), &st, metrics());
        let normal = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::None);
        assert_eq!(
            normal.quads[0].color, st.background,
            "fond de barre d'abord"
        );
        let active_bg = normal
            .quads
            .iter()
            .find(|q| q.rect == l.tabs[0].rect)
            .unwrap();
        assert_eq!(active_bg.color, st.active_background);
        let inactive_bg = normal
            .quads
            .iter()
            .find(|q| q.rect == l.tabs[1].rect)
            .unwrap();
        assert_eq!(inactive_bg.color, st.inactive_background);
        let close0 = normal
            .quads
            .iter()
            .find(|q| q.rect.x == 80 && q.rect.width == 10)
            .unwrap();
        assert_eq!(close0.color, st.close_background);
        let x_text = normal
            .texts
            .iter()
            .find(|t| t.text == "✕" && t.x == 80)
            .unwrap();
        assert_eq!(x_text.color, st.close_foreground);
        let discs: Vec<_> = normal
            .texts
            .iter()
            .filter(|t| t.text == "\u{E0B6}" || t.text == "\u{E0B4}")
            .collect();
        assert_eq!(discs.len(), 4, "deux demi-disques par onglet");
        assert!(discs.iter().all(|t| t.color == st.close_background));
        let title = normal.texts.iter().find(|t| t.text == "1: sh").unwrap();
        assert_eq!(
            (title.x, title.y, title.color),
            (10, 2, st.active_foreground)
        );
        let plus = normal.texts.iter().find(|t| t.text == "+").unwrap();
        assert_eq!(plus.x, 240);

        let hovered = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::CloseButton(0));
        let close0 = hovered
            .quads
            .iter()
            .find(|q| q.rect.x == 80 && q.rect.width == 10)
            .unwrap();
        assert_eq!(close0.color, st.close_hover_background);
        let x_text = hovered
            .texts
            .iter()
            .find(|t| t.text == "✕" && t.x == 80)
            .unwrap();
        assert_eq!(x_text.color, st.close_hover_foreground);
        let close1 = hovered
            .quads
            .iter()
            .find(|q| q.rect.x == 200 && q.rect.width == 10)
            .unwrap();
        assert_eq!(
            close1.color, st.close_background,
            "l'autre bouton garde ses couleurs"
        );
    }

    #[test]
    fn long_titles_are_truncated_with_an_ellipsis() {
        let long = [TabSpec {
            title: "un titre vraiment beaucoup trop long pour la barre",
            active: true,
            accent: None,
        }];
        let l = layout_tab_bar(200, 0, &long, &style(false), metrics());
        assert_eq!(l.tabs.len(), 1);
        assert!(l.tabs[0].rect.width <= 200);
        let chrome = tab_bar_chrome(&l, &long, &style(false), metrics(), HoverTarget::None);
        let title = chrome
            .texts
            .iter()
            .find(|t| t.text.ends_with('…'))
            .expect("titre tronqué");
        assert!(title.text.chars().count() * 10 <= 180, "{}", title.text);
    }
}
