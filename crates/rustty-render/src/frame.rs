//! Ce que le binaire donne au renderer pour une image : les panneaux avec
//! leur instantané, et le chrome (barre d'onglets, bordures, bandeaux)
//! déjà réduit à des rectangles et des textes.

use rustty_vt::Snapshot;

use crate::color::Rgba;
use crate::pipeline::quad::QuadStyle;

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

/// Un glyphe seul, centré sur un point par le centre de son encre (et non posé
/// sur une cellule) : les symboles d'une pastille tombent au milieu du cercle.
#[derive(Clone, Debug, PartialEq)]
pub struct ChromeIcon {
    /// Centre voulu, en pixels.
    pub x: u32,
    pub y: u32,
    pub ch: char,
    pub color: Rgba,
}

/// Rectangle stylé (coins arrondis, bordure, dégradé), dessiné après les quads.
#[derive(Clone, Debug, PartialEq)]
pub struct ChromeShape {
    pub rect: PixelRect,
    pub style: QuadStyle,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chrome {
    pub quads: Vec<ChromeQuad>,
    pub shapes: Vec<ChromeShape>,
    pub texts: Vec<ChromeText>,
    pub icons: Vec<ChromeIcon>,
}

pub struct Frame<'a> {
    pub viewport: (u32, u32),
    /// Fond de la fenêtre ; son alpha est l'opacité configurée.
    pub background: Rgba,
    pub panes: Vec<PaneFrame<'a>>,
    pub chrome: Chrome,
}
