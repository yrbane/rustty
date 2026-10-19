//! Bandeaux de panneau : géométrie pure, survol, titres et dessin.

use rustty_layout::WindowId;
use rustty_render::{
    CellMetrics, Chrome, PaneHeaderSpec, PixelRect, pane_header_chrome, pane_header_close_rect,
};

use crate::appearance;
use crate::tab::{Tab, TermId};
use crate::window_state::OsWindow;

/// Largeur minimale (en cellules) pour afficher le ✕ : la pastille occupe
/// 4 cellules à partir d'une cellule de marge, plus une cellule de titre.
const CLOSE_MIN_CELLS: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneHeader {
    pub term: TermId,
    pub rect: PixelRect,
    pub close: PixelRect,
}

impl PaneHeader {
    pub fn has_close(&self) -> bool {
        self.close.width > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderHit {
    Title(TermId),
    Close(TermId),
}

/// Les bandeaux n'existent que si l'onglet montre au moins deux panneaux.
pub fn headers_visible(tab: &Tab) -> bool {
    tab.layout.zoomed().is_none() && tab.layout.windows().len() >= 2
}

/// Sépare un panneau en (bandeau, contenu) ; la hauteur est saturée.
pub fn split_header(pane: PixelRect, height: u32) -> (PixelRect, PixelRect) {
    let h = height.min(pane.height);
    (
        PixelRect::new(pane.x, pane.y, pane.width, h),
        PixelRect::new(pane.x, pane.y + h, pane.width, pane.height - h),
    )
}

/// Le bandeau de `term` ; sans ✕ (rectangle vide) quand il est trop étroit.
pub fn header_for(term: TermId, rect: PixelRect, metrics: CellMetrics) -> PaneHeader {
    let close = if rect.width >= CLOSE_MIN_CELLS * metrics.width {
        pane_header_close_rect(rect, metrics)
    } else {
        PixelRect::new(rect.x + rect.width, rect.y, 0, 0)
    };
    PaneHeader { term, rect, close }
}

fn contains(r: PixelRect, x: f64, y: f64) -> bool {
    x >= f64::from(r.x)
        && x < f64::from(r.x + r.width)
        && y >= f64::from(r.y)
        && y < f64::from(r.y + r.height)
}

/// Le bandeau sous le point ; le ✕ prime sur le titre.
pub fn header_at(headers: &[PaneHeader], x: f64, y: f64) -> Option<HeaderHit> {
    headers.iter().find_map(|h| {
        if contains(h.close, x, y) {
            Some(HeaderHit::Close(h.term))
        } else {
            contains(h.rect, x, y).then_some(HeaderHit::Title(h.term))
        }
    })
}

type Rects = Vec<(WindowId, PixelRect)>;

/// Découpe les rectangles complets des panneaux en bandeaux et contenus ;
/// sans bandeau visible, les contenus sont les rectangles complets.
pub fn layout_headers(
    tab: &Tab,
    full: &[(WindowId, PixelRect)],
    metrics: CellMetrics,
) -> (Vec<PaneHeader>, Rects) {
    if !headers_visible(tab) {
        return (Vec::new(), full.to_vec());
    }
    let mut headers = Vec::new();
    let mut contents = Vec::new();
    for (wid, rect) in full {
        let (header, content) = split_header(*rect, metrics.height);
        if let Some(term) = tab.term_at(*wid) {
            headers.push(header_for(term, header, metrics));
        }
        contents.push((*wid, content));
    }
    (headers, contents)
}

impl OsWindow {
    /// Nom du panneau, sinon titre du shell, sinon rien.
    pub fn pane_title(&self, term: TermId) -> &str {
        self.model
            .workspace
            .pane_name(term)
            .or_else(|| self.titles.get(&term).map(String::as_str))
            .unwrap_or("")
    }

    /// Bandeau et contenu réunis : la cible d'un dépôt.
    #[allow(dead_code)] // consommé par le glisser-déposer (tâche 9)
    pub fn full_pane_rect(&self, term: TermId) -> Option<PixelRect> {
        let wid = self.model.workspace.active_tab().window_of(term)?;
        let content = self.pane_rects.iter().find(|(w, _)| *w == wid)?.1;
        Some(match self.headers.iter().find(|h| h.term == term) {
            Some(h) => PixelRect::new(
                h.rect.x,
                h.rect.y,
                h.rect.width,
                content.y + content.height - h.rect.y,
            ),
            None => content,
        })
    }

    /// Recalcule le survol ; vrai s'il a changé (nouveau dessin à demander).
    pub fn update_header_hover(&mut self, x: f64, y: f64) -> bool {
        let hit = if self.cursor_inside {
            header_at(&self.headers, x, y)
        } else {
            None
        };
        let changed = hit != self.header_hover;
        self.header_hover = hit;
        changed
    }

    /// Dessin des bandeaux ; le ✕ n'apparaît que sur le bandeau survolé.
    pub fn header_chrome(&self) -> Chrome {
        if self.headers.is_empty() || self.model.workspace.is_empty() {
            return Chrome::default();
        }
        let tab = self.model.workspace.active_tab();
        let focused = tab.focused_term();
        let specs: Vec<_> = self
            .headers
            .iter()
            .map(|h| {
                let hovered = match self.header_hover {
                    Some(HeaderHit::Title(t) | HeaderHit::Close(t)) => t == h.term,
                    None => false,
                };
                PaneHeaderSpec {
                    rect: h.rect,
                    title: self.pane_title(h.term),
                    color: appearance::header_color(
                        &self.config.splits,
                        &self.palette,
                        tab,
                        h.term,
                    ),
                    focused: self.focused && focused == Some(h.term),
                    show_close: hovered && h.has_close(),
                    close_hovered: self.header_hover == Some(HeaderHit::Close(h.term)),
                }
            })
            .collect();
        pane_header_chrome(&specs, &self.tab_style, self.metrics())
    }
}

#[cfg(test)]
#[path = "pane_header_tests.rs"]
mod tests;
