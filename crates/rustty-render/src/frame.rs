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

    /// Le point `(x, y)` en pixels est dans le rectangle (bornes hautes exclues).
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= f64::from(self.x)
            && x < f64::from(self.x + self.width)
            && y >= f64::from(self.y)
            && y < f64::from(self.y + self.height)
    }

    /// Le pixel `(x, y)` est dans le rectangle (bornes hautes exclues).
    pub fn contains_px(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Plage de cellules `(ligne, colonne)` bornes incluses, en ordre de
/// lecture : de `start` à la fin de sa ligne, les lignes entières entre
/// les deux, puis le début de la ligne de `end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRange {
    pub start: (usize, usize),
    pub end: (usize, usize),
}

impl CellRange {
    pub fn contains(&self, row: usize, col: usize) -> bool {
        (row, col) >= self.start && (row, col) <= self.end
    }
}

pub struct PaneFrame<'a> {
    pub rect: PixelRect,
    pub snapshot: &'a Snapshot,
    pub focused: bool,
    /// Lien survolé : ses cellules sont soulignées.
    pub link_hover: Option<CellRange>,
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
    /// Corps de la police d'icônes, en pixels.
    pub size: u32,
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

#[cfg(test)]
mod tests {
    use super::PixelRect;

    #[test]
    fn contains_point_is_half_open() {
        let r = PixelRect::new(10, 20, 5, 5);
        assert!(r.contains_point(10.0, 20.0) && r.contains_point(14.9, 24.9));
        assert!(!r.contains_point(15.0, 20.0) && !r.contains_point(9.9, 20.0));
        assert!(r.contains_px(14, 24) && !r.contains_px(15, 24));
    }
}
