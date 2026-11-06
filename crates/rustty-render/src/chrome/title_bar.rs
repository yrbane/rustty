//! La barre de titre maison : style en pixels physiques, disposition des
//! boutons et du titre, zones cliquables, et sa traduction en formes et
//! textes pour le renderer. Pur, sans GPU.

use rustty_config::{ButtonStyle, TitleBar, TitleButton};

use super::title_fit::{fit_title, title_cells};
use crate::color::{Rgba, readable_on};
use crate::font::CellMetrics;
use crate::frame::{Chrome, ChromeIcon, ChromeShape, ChromeText, PixelRect};
use crate::pipeline::quad::QuadStyle;

/// Couleur du titre sur un fond clair.
const DARK_TITLE: Rgba = Rgba::new(0.07, 0.07, 0.07, 1.0);

/// Glyphes des boutons de fenêtre en attendant les icônes Material.
const MINIMIZE_GLYPH: char = '–';
const MAXIMIZE_GLYPH: char = '□';
const RESTORE_GLYPH: char = '❐';
const CLOSE_GLYPH: char = '✕';

/// Aspect d'un bouton, en pixels physiques.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonLook {
    pub size: u32,
    pub background: Rgba,
    pub border: Rgba,
    pub border_width: u32,
    pub radius: u32,
    pub icon: Rgba,
    pub hover_background: Rgba,
    pub hover_border: Rgba,
    pub hover_icon: Rgba,
}

impl ButtonLook {
    fn from_config(style: &ButtonStyle, scale: f64) -> Self {
        Self {
            size: px(style.size, scale),
            background: Rgba::from_rgba8(style.background),
            border: Rgba::from_rgba8(style.border_color),
            border_width: px(style.border_width, scale),
            radius: px(style.radius, scale),
            icon: Rgba::from_rgba8(style.icon_color),
            hover_background: Rgba::from_rgba8(style.hover_background),
            hover_border: Rgba::from_rgba8(style.hover_border_color),
            hover_icon: Rgba::from_rgba8(style.hover_icon_color),
        }
    }

    fn quad_style(&self, hovered: bool) -> QuadStyle {
        let (fill, border) = if hovered {
            (self.hover_background, self.hover_border)
        } else {
            (self.background, self.border)
        };
        QuadStyle {
            fill,
            fill_bottom: fill,
            radius: self.radius as f32,
            border_width: self.border_width as f32,
            border,
        }
    }
}

/// Pixels logiques → physiques.
fn px(logical: u32, scale: f64) -> u32 {
    (f64::from(logical) * scale).round() as u32
}

#[derive(Clone, Debug, PartialEq)]
pub struct TitleBarStyle {
    pub height: u32,
    pub background: Rgba,
    pub gloss: f32,
    pub separator: Rgba,
    pub title_color: Rgba,
    pub padding: u32,
    pub spacing: u32,
    /// Boutons visibles, dans l'ordre d'affichage.
    pub buttons: Vec<(TitleButton, ButtonLook)>,
}

impl TitleBarStyle {
    /// Tailles converties en pixels physiques. Ce jalon ne dessine que les
    /// boutons de fenêtre ; les autres viendront avec leurs icônes.
    pub fn from_config(cfg: &TitleBar, scale_factor: f64) -> Self {
        let buttons = TitleButton::ALL
            .into_iter()
            .filter(|b| b.is_window_control() && cfg.button_visible(*b))
            .map(|b| {
                (
                    b,
                    ButtonLook::from_config(&cfg.button_style(b), scale_factor),
                )
            })
            .collect();
        Self {
            height: px(cfg.height, scale_factor),
            background: Rgba::from_rgba8(cfg.background),
            gloss: cfg.gloss,
            separator: Rgba::from_rgba8(cfg.separator_color),
            title_color: Rgba::from_rgba8(cfg.title_color),
            padding: px(cfg.padding, scale_factor),
            spacing: px(cfg.spacing, scale_factor),
            buttons,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleTarget {
    None,
    Button(TitleButton),
    Title,
    Background,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleBarLayout {
    pub bar: PixelRect,
    pub separator: PixelRect,
    pub buttons: Vec<(TitleButton, PixelRect)>,
    pub title: PixelRect,
}

impl TitleBarLayout {
    pub fn hit_test(&self, x: u32, y: u32) -> TitleTarget {
        if !inside(self.bar, x, y) {
            return TitleTarget::None;
        }
        if let Some((button, _)) = self.buttons.iter().find(|(_, r)| inside(*r, x, y)) {
            return TitleTarget::Button(*button);
        }
        if inside(self.title, x, y) {
            return TitleTarget::Title;
        }
        TitleTarget::Background
    }
}

fn inside(r: PixelRect, x: u32, y: u32) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

/// Hauteur totale réservée : la barre et son séparateur.
pub fn title_bar_height(style: &TitleBarStyle) -> u32 {
    style.height + 1
}

pub fn layout_title_bar(viewport_width: u32, style: &TitleBarStyle) -> TitleBarLayout {
    let bar = PixelRect::new(0, 0, viewport_width, style.height);
    let separator = PixelRect::new(0, style.height, viewport_width, 1);
    let centered = |size: u32| (style.height.saturating_sub(size)) / 2;
    let mut buttons = Vec::with_capacity(style.buttons.len());
    let mut left_edge = style.padding;
    for (button, look) in style.buttons.iter().filter(|(b, _)| !b.is_window_control()) {
        buttons.push((
            *button,
            PixelRect::new(left_edge, centered(look.size), look.size, look.size),
        ));
        left_edge += look.size + style.spacing;
    }
    let mut right_edge = viewport_width.saturating_sub(style.padding);
    let mut controls = Vec::new();
    for (button, look) in style
        .buttons
        .iter()
        .rev()
        .filter(|(b, _)| b.is_window_control())
    {
        let x = right_edge.saturating_sub(look.size);
        controls.push((
            *button,
            PixelRect::new(x, centered(look.size), look.size, look.size),
        ));
        right_edge = x.saturating_sub(style.spacing);
    }
    let left_edge = if buttons.is_empty() {
        style.padding
    } else {
        left_edge - style.spacing
    };
    let right_edge = controls
        .last()
        .map_or(viewport_width.saturating_sub(style.padding), |(_, r)| r.x);
    controls.reverse();
    buttons.extend(controls);
    let title_x = left_edge + style.spacing;
    let title_width = right_edge
        .saturating_sub(style.spacing)
        .saturating_sub(title_x);
    TitleBarLayout {
        bar,
        separator,
        buttons,
        title: PixelRect::new(title_x, 0, title_width, style.height),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitleBarState<'a> {
    pub title: &'a str,
    pub hover: Option<TitleButton>,
    pub maximized: bool,
}

pub fn title_bar_chrome(
    layout: &TitleBarLayout,
    style: &TitleBarStyle,
    state: TitleBarState,
    metrics: CellMetrics,
) -> Chrome {
    let mut chrome = Chrome::default();
    chrome.shapes.push(ChromeShape {
        rect: layout.bar,
        style: QuadStyle {
            fill: style.background.lighten(style.gloss),
            fill_bottom: style.background,
            ..QuadStyle::flat(style.background)
        },
    });
    if style.gloss > 0.0 {
        chrome.shapes.push(ChromeShape {
            rect: PixelRect::new(layout.bar.x, layout.bar.y, layout.bar.width, 1),
            style: QuadStyle::flat(Rgba::new(1.0, 1.0, 1.0, style.gloss)),
        });
    }
    chrome.shapes.push(ChromeShape {
        rect: layout.separator,
        style: QuadStyle::flat(style.separator),
    });
    for (button, rect) in &layout.buttons {
        let Some((_, look)) = style.buttons.iter().find(|(b, _)| b == button) else {
            continue;
        };
        let hovered = state.hover == Some(*button);
        chrome.shapes.push(ChromeShape {
            rect: *rect,
            style: look.quad_style(hovered),
        });
        let glyph = match button {
            TitleButton::Minimize => MINIMIZE_GLYPH,
            TitleButton::Maximize if state.maximized => RESTORE_GLYPH,
            TitleButton::Maximize => MAXIMIZE_GLYPH,
            TitleButton::Close => CLOSE_GLYPH,
            _ => continue,
        };
        chrome.icons.push(ChromeIcon {
            x: rect.x + rect.width / 2,
            y: rect.y + rect.height / 2,
            ch: glyph,
            color: if hovered { look.hover_icon } else { look.icon },
        });
    }
    let max_cells = layout.title.width / metrics.width.max(1);
    let text = fit_title(state.title, max_cells);
    if !text.is_empty() {
        let cells = title_cells(&text);
        chrome.texts.push(ChromeText {
            x: layout.title.x + layout.title.width.saturating_sub(cells * metrics.width) / 2,
            y: center(layout.bar.y, layout.bar.height, metrics.height),
            text,
            color: readable_on(style.background, style.title_color, DARK_TITLE),
        });
    }
    chrome
}

/// Origine d'un élément de `inner` pixels centré dans `[start, start + outer)` ;
/// un élément plus grand que la zone déborde des deux côtés, jamais sous 0.
fn center(start: u32, outer: u32, inner: u32) -> u32 {
    let offset = (i64::from(outer) - i64::from(inner)) / 2;
    (i64::from(start) + offset).max(0) as u32
}

#[cfg(test)]
#[path = "title_bar_tests.rs"]
mod tests;
