//! La barre d'onglets : disposition en pixels, rectangles cliquables, et sa
//! traduction en quads et textes pour le renderer. Pur, sans GPU.

use rustty_config::CloseButtonStyle;
use unicode_width::UnicodeWidthStr;

use crate::color::{Palette, Rgba};
use crate::font::CellMetrics;
use crate::frame::{Chrome, ChromeQuad, ChromeText, PixelRect};

pub const TAB_BAR_PADDING: u32 = 2;
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
}

impl TabBarStyle {
    pub fn from_config(
        palette: &Palette,
        close: &CloseButtonStyle,
        show_close_button: bool,
    ) -> Self {
        Self {
            background: palette.ansi[8].dim(0.5),
            active_background: palette.background,
            inactive_background: palette.ansi[0],
            active_foreground: palette.foreground,
            inactive_foreground: palette.ansi[7],
            close_foreground: Rgba::from_rgb(close.foreground),
            close_background: Rgba::from_rgb(close.background),
            close_hover_foreground: Rgba::from_rgb(close.hover_foreground),
            close_hover_background: Rgba::from_rgb(close.hover_background),
            show_close_button,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabSpec<'a> {
    pub title: &'a str,
    pub active: bool,
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

pub fn tab_bar_height(metrics: CellMetrics) -> u32 {
    metrics.height + 2 * TAB_BAR_PADDING
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
    let (cw, bar_h) = (metrics.width, tab_bar_height(metrics));
    let close_cells = if style.show_close_button {
        CLOSE_CELLS
    } else {
        0
    };
    let mut x = 0u32;
    let mut out = Vec::new();
    for tab in tabs {
        let max_title = (viewport_width / cw).saturating_sub(2 + close_cells);
        let title = title_cells(tab.title).min(max_title);
        let cells = 1 + title + close_cells + 1;
        let width = cells * cw;
        if x + width > viewport_width {
            break;
        }
        let close = style.show_close_button.then(|| {
            PixelRect::new(
                x + (1 + title) * cw,
                y + TAB_BAR_PADDING,
                close_cells * cw,
                metrics.height,
            )
        });
        out.push(TabRect {
            rect: PixelRect::new(x, y, width, bar_h),
            close,
        });
        x += width;
    }
    let new_w = NEW_TAB_CELLS * cw;
    let new_tab = if x + new_w <= viewport_width {
        PixelRect::new(x, y, new_w, bar_h)
    } else {
        PixelRect::new(x, y, 0, bar_h)
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
    let text_y = layout.bar.y + TAB_BAR_PADDING;
    for (i, (tab_rect, spec)) in layout.tabs.iter().zip(tabs).enumerate() {
        let (bg, fg) = if spec.active {
            (style.active_background, style.active_foreground)
        } else {
            (style.inactive_background, style.inactive_foreground)
        };
        chrome.quads.push(ChromeQuad {
            rect: tab_rect.rect,
            color: bg,
        });
        let close_cells = if tab_rect.close.is_some() {
            CLOSE_CELLS
        } else {
            0
        };
        let max_title = (tab_rect.rect.width / cw).saturating_sub(2 + close_cells);
        chrome.texts.push(ChromeText {
            x: tab_rect.rect.x + cw,
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
    use rustty_config::{CloseButtonStyle, Colors};

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

    fn style(show_close: bool) -> TabBarStyle {
        TabBarStyle::from_config(
            &Palette::from_config(&Colors::default(), false),
            &CloseButtonStyle::default(),
            show_close,
        )
    }

    fn tabs() -> Vec<TabSpec<'static>> {
        vec![
            TabSpec {
                title: "1: sh",
                active: true,
            },
            TabSpec {
                title: "2: vim",
                active: false,
            },
        ]
    }

    #[test]
    fn height_is_a_cell_plus_padding() {
        assert_eq!(tab_bar_height(metrics()), 24);
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
