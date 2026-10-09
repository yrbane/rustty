//! De l'état de la fenêtre à la `Frame` du renderer : panneaux, barre
//! d'onglets, sélection, bandeau, fond avec opacité. Pur.

use rustty_render::{
    CellMetrics, Chrome, ChromeQuad, Frame, PaneFrame, PixelRect, Rgba, TabSpec, grid_geometry,
};
use rustty_vt::Snapshot;

use crate::mouse::Selection;

pub struct PaneView {
    pub rect: PixelRect,
    pub snapshot: Snapshot,
    pub focused: bool,
}

/// Le fond de fenêtre avec l'opacité ; prémultiplié si le compositeur le veut.
pub fn background_color(background: Rgba, opacity: f32, premultiplied: bool) -> Rgba {
    let a = opacity.clamp(0.0, 1.0);
    if premultiplied {
        Rgba::new(background.r * a, background.g * a, background.b * a, a)
    } else {
        background.with_alpha(a)
    }
}

pub fn tab_specs(titles: &[String], active: usize) -> Vec<TabSpec<'_>> {
    titles
        .iter()
        .enumerate()
        .map(|(i, t)| TabSpec {
            title: t.as_str(),
            active: i == active,
        })
        .collect()
}

pub fn selection_quads(
    selection: &Selection,
    rect: PixelRect,
    metrics: CellMetrics,
    padding: u32,
    color: Rgba,
) -> Vec<ChromeQuad> {
    let g = grid_geometry(rect, metrics, padding);
    if g.cols == 0 || g.rows == 0 {
        return Vec::new();
    }
    let (start, end) = selection.bounds();
    let mut quads = Vec::new();
    for row in start.row..=end.row.min(g.rows - 1) {
        let first = if row == start.row { start.col } else { 0 };
        let last = if row == end.row { end.col } else { g.cols - 1 };
        let (first, last) = (first.min(g.cols - 1), last.min(g.cols - 1));
        if row == start.row && start.col >= g.cols {
            continue;
        }
        let x = g.origin_x + first as u32 * metrics.width;
        let y = g.origin_y + row as u32 * metrics.height;
        quads.push(ChromeQuad {
            rect: PixelRect::new(
                x,
                y,
                (last - first + 1) as u32 * metrics.width,
                metrics.height,
            ),
            color,
        });
    }
    quads
}

pub fn build_frame(
    viewport: (u32, u32),
    background: Rgba,
    panes: &[PaneView],
    chrome: Chrome,
) -> Frame<'_> {
    Frame {
        viewport,
        background,
        panes: panes
            .iter()
            .map(|p| PaneFrame {
                rect: p.rect,
                snapshot: &p.snapshot,
                focused: p.focused,
            })
            .collect(),
        chrome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mouse::CellPos;
    use rustty_vt::Term;

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

    #[test]
    fn background_is_premultiplied_when_the_surface_wants_it() {
        let bg = Rgba::new(1.0, 0.5, 0.0, 1.0);
        let straight = background_color(bg, 0.5, false);
        assert_eq!((straight.r, straight.g, straight.a), (1.0, 0.5, 0.5));
        let pre = background_color(bg, 0.5, true);
        assert_eq!((pre.r, pre.g, pre.a), (0.5, 0.25, 0.5));
    }

    #[test]
    fn tab_specs_mark_the_active_tab() {
        let titles = vec!["1: sh".to_string(), "2: vim".to_string()];
        let specs = tab_specs(&titles, 1);
        assert_eq!(specs.len(), 2);
        assert!(!specs[0].active && specs[1].active);
        assert_eq!(specs[1].title, "2: vim");
    }

    #[test]
    fn selection_quads_cover_each_row_segment() {
        let mut sel = Selection::start(CellPos { col: 2, row: 0 });
        sel.extend(CellPos { col: 1, row: 2 });
        let color = Rgba::new(0.0, 0.0, 1.0, 0.4);
        let quads = selection_quads(&sel, PixelRect::new(100, 50, 108, 68), metrics(), 4, color);
        assert_eq!(quads.len(), 3);
        assert_eq!(
            quads[0].rect,
            PixelRect::new(124, 54, 80, 20),
            "du col 2 à la fin (10 colonnes)"
        );
        assert_eq!(
            quads[1].rect,
            PixelRect::new(104, 74, 100, 20),
            "ligne entière"
        );
        assert_eq!(quads[2].rect, PixelRect::new(104, 94, 20, 20), "col 0 à 1");
        assert!(quads.iter().all(|q| q.color == color));
    }

    #[test]
    fn selection_quads_are_clipped_to_the_visible_grid() {
        let mut sel = Selection::start(CellPos { col: 50, row: 7 });
        sel.extend(CellPos { col: 60, row: 9 });
        let quads = selection_quads(
            &sel,
            PixelRect::new(0, 0, 100, 60),
            metrics(),
            0,
            Rgba::new(0.0, 0.0, 0.0, 0.5),
        );
        assert!(quads.is_empty(), "entièrement hors de la grille 10×3");
    }

    #[test]
    fn build_frame_assembles_panes_and_chrome() {
        let mut t = Term::new(10, 2, 0);
        t.input(b"x");
        let panes = vec![PaneView {
            rect: PixelRect::new(0, 24, 200, 100),
            snapshot: t.snapshot(),
            focused: true,
        }];
        let chrome = Chrome {
            quads: vec![ChromeQuad {
                rect: PixelRect::new(0, 0, 200, 24),
                color: Rgba::new(0.0, 0.0, 0.0, 1.0),
            }],
            texts: Vec::new(),
        };
        let frame = build_frame((200, 124), Rgba::new(0.1, 0.1, 0.1, 0.9), &panes, chrome);
        assert_eq!(frame.viewport, (200, 124));
        assert_eq!(frame.panes.len(), 1);
        assert!(frame.panes[0].focused);
        assert_eq!(frame.panes[0].rect, PixelRect::new(0, 24, 200, 100));
        assert_eq!(frame.chrome.quads.len(), 1);
        assert_eq!(frame.background.a, 0.9);
    }
}
