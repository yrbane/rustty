//! Découpage de la fenêtre en pixels : barre d'onglets, zone de contenu,
//! rectangles des panneaux, cellule sous la souris. Pur.

use rustty_config::TabBarPosition;
use rustty_layout::{Rect, TabLayout, WindowId};
use rustty_render::{CellMetrics, PixelRect, grid_geometry};

/// Interstice entre deux panneaux, où le fond de la fenêtre reste visible.
pub const PANE_GAP: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowGeometry {
    pub tab_bar_y: Option<u32>,
    pub content: Rect,
}

pub fn window_geometry(
    width: u32,
    height: u32,
    position: TabBarPosition,
    tab_count: usize,
    min_tabs: u32,
    bar_height: u32,
) -> WindowGeometry {
    let visible = position != TabBarPosition::Hidden && tab_count as u64 >= u64::from(min_tabs);
    if !visible {
        return WindowGeometry {
            tab_bar_y: None,
            content: Rect::new(0, 0, width, height),
        };
    }
    let content_height = height.saturating_sub(bar_height);
    match position {
        TabBarPosition::Bottom => WindowGeometry {
            tab_bar_y: Some(content_height),
            content: Rect::new(0, 0, width, content_height),
        },
        _ => WindowGeometry {
            tab_bar_y: Some(0),
            content: Rect::new(0, bar_height.min(height), width, content_height),
        },
    }
}

pub fn pane_rects(layout: &TabLayout, content: Rect) -> Vec<(WindowId, PixelRect)> {
    layout
        .rects(content, PANE_GAP)
        .into_iter()
        .map(|(id, r)| (id, PixelRect::new(r.x, r.y, r.width, r.height)))
        .collect()
}

pub fn grid_size(rect: PixelRect, metrics: CellMetrics, padding: u32) -> (usize, usize) {
    let g = grid_geometry(rect, metrics, padding);
    (g.cols.max(1), g.rows.max(1))
}

/// La cellule sous le point `(x, y)` en pixels, ou `None` hors de la grille.
pub fn cell_at(
    rect: PixelRect,
    metrics: CellMetrics,
    padding: u32,
    x: f64,
    y: f64,
) -> Option<(usize, usize)> {
    let g = grid_geometry(rect, metrics, padding);
    if g.cols == 0 || g.rows == 0 || x < f64::from(g.origin_x) || y < f64::from(g.origin_y) {
        return None;
    }
    let col = ((x - f64::from(g.origin_x)) / f64::from(metrics.width)) as usize;
    let row = ((y - f64::from(g.origin_y)) / f64::from(metrics.height)) as usize;
    (col < g.cols && row < g.rows).then_some((col, row))
}

pub fn pane_at(rects: &[(WindowId, PixelRect)], x: f64, y: f64) -> Option<WindowId> {
    rects
        .iter()
        .find(|(_, r)| {
            x >= f64::from(r.x)
                && x < f64::from(r.x + r.width)
                && y >= f64::from(r.y)
                && y < f64::from(r.y + r.height)
        })
        .map(|(id, _)| *id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustty_layout::Axis;

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
    fn tab_bar_on_top_pushes_the_content_down() {
        let g = window_geometry(800, 600, TabBarPosition::Top, 1, 1, 24);
        assert_eq!(g.tab_bar_y, Some(0));
        assert_eq!(g.content, Rect::new(0, 24, 800, 576));
    }

    #[test]
    fn tab_bar_at_the_bottom_keeps_the_content_on_top() {
        let g = window_geometry(800, 600, TabBarPosition::Bottom, 2, 1, 24);
        assert_eq!(g.tab_bar_y, Some(576));
        assert_eq!(g.content, Rect::new(0, 0, 800, 576));
    }

    #[test]
    fn hidden_bar_or_too_few_tabs_gives_the_whole_window() {
        assert_eq!(
            window_geometry(800, 600, TabBarPosition::Hidden, 3, 1, 24).tab_bar_y,
            None
        );
        let g = window_geometry(800, 600, TabBarPosition::Top, 1, 2, 24);
        assert_eq!(g.tab_bar_y, None);
        assert_eq!(g.content, Rect::new(0, 0, 800, 600));
    }

    #[test]
    fn tiny_windows_never_underflow() {
        let g = window_geometry(10, 10, TabBarPosition::Top, 1, 1, 24);
        assert_eq!(g.content.height, 0);
        assert_eq!(g.tab_bar_y, Some(0));
        let b = window_geometry(10, 10, TabBarPosition::Bottom, 1, 1, 24);
        assert_eq!(b.tab_bar_y, Some(0));
    }

    #[test]
    fn pane_rects_follow_the_layout_with_a_gap() {
        let (mut layout, first) = TabLayout::new();
        let second = layout.split(first, Axis::Vertical).unwrap();
        let rects = pane_rects(&layout, Rect::new(0, 24, 802, 500));
        assert_eq!(rects.len(), 2);
        let left = rects.iter().find(|(id, _)| *id == first).unwrap().1;
        let right = rects.iter().find(|(id, _)| *id == second).unwrap().1;
        assert_eq!(left, PixelRect::new(0, 24, 400, 500));
        assert_eq!(right, PixelRect::new(402, 24, 400, 500));
    }

    #[test]
    fn grid_size_is_at_least_one_cell() {
        assert_eq!(
            grid_size(PixelRect::new(0, 0, 108, 48), metrics(), 4),
            (10, 2)
        );
        assert_eq!(grid_size(PixelRect::new(0, 0, 3, 3), metrics(), 4), (1, 1));
    }

    #[test]
    fn cell_at_maps_pixels_inside_the_grid() {
        let rect = PixelRect::new(100, 50, 108, 48);
        assert_eq!(cell_at(rect, metrics(), 4, 104.0, 54.0), Some((0, 0)));
        assert_eq!(cell_at(rect, metrics(), 4, 113.9, 73.9), Some((0, 0)));
        assert_eq!(cell_at(rect, metrics(), 4, 114.0, 74.0), Some((1, 1)));
        assert_eq!(cell_at(rect, metrics(), 4, 203.0, 93.0), Some((9, 1)));
    }

    #[test]
    fn cell_at_outside_rect_is_none() {
        let rect = PixelRect::new(100, 50, 108, 48);
        assert_eq!(
            cell_at(rect, metrics(), 4, 101.0, 60.0),
            None,
            "dans la marge"
        );
        assert_eq!(cell_at(rect, metrics(), 4, 99.0, 60.0), None);
        assert_eq!(cell_at(rect, metrics(), 4, -5.0, -5.0), None);
        assert_eq!(
            cell_at(rect, metrics(), 4, 204.0, 60.0),
            None,
            "après la dernière colonne"
        );
        assert_eq!(
            cell_at(PixelRect::new(0, 0, 5, 5), metrics(), 0, 1.0, 1.0),
            None,
            "fenêtre plus petite qu'une cellule"
        );
    }

    #[test]
    fn pane_at_finds_the_pane_under_the_point() {
        let rects = vec![
            (WindowId(1), PixelRect::new(0, 0, 400, 500)),
            (WindowId(2), PixelRect::new(402, 0, 400, 500)),
        ];
        assert_eq!(pane_at(&rects, 10.0, 10.0), Some(WindowId(1)));
        assert_eq!(pane_at(&rects, 500.0, 10.0), Some(WindowId(2)));
        assert_eq!(pane_at(&rects, 401.0, 10.0), None, "dans l'interstice");
        assert_eq!(pane_at(&rects, 10.0, 600.0), None);
    }
}
