//! Ce que le binaire donne au renderer pour une image : les panneaux avec
//! leur instantané, et le chrome (barre d'onglets, bordures, bandeaux)
//! déjà réduit à des rectangles et des textes.

use rustty_vt::Snapshot;

use crate::color::Rgba;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub struct PaneFrame<'a> {
    pub rect: PixelRect,
    pub snapshot: &'a Snapshot,
    pub focused: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChromeQuad {
    pub rect: PixelRect,
    pub color: Rgba,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChromeText {
    pub x: u32,
    pub y: u32,
    pub text: String,
    pub color: Rgba,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chrome {
    pub quads: Vec<ChromeQuad>,
    pub texts: Vec<ChromeText>,
}

pub struct Frame<'a> {
    pub viewport: (u32, u32),
    /// Fond de la fenêtre ; son alpha est l'opacité configurée.
    pub background: Rgba,
    pub panes: Vec<PaneFrame<'a>>,
    pub chrome: Chrome,
}
