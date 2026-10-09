//! Cellule sous un point en pixels : exacte (`None` hors grille) ou bornée
//! au panneau (pour un relâchement de souris hors de celui-ci). Pur.

use rustty_render::{CellMetrics, PixelRect, grid_geometry};

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

/// Comme `cell_at`, mais un point hors de la grille retombe sur la cellule la
/// plus proche : le relâchement d'un bouton tenu doit toujours être rapporté.
pub fn cell_at_clamped(
    rect: PixelRect,
    metrics: CellMetrics,
    padding: u32,
    x: f64,
    y: f64,
) -> (usize, usize) {
    let g = grid_geometry(rect, metrics, padding);
    let axis = |v: f64, origin: u32, size: u32, count: usize| {
        let cell = ((v - f64::from(origin)) / f64::from(size)).max(0.0) as usize;
        cell.min(count.saturating_sub(1))
    };
    (
        axis(x, g.origin_x, metrics.width, g.cols),
        axis(y, g.origin_y, metrics.height, g.rows),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn clamped_cell_stays_inside_the_grid() {
        let rect = PixelRect::new(100, 50, 108, 48);
        // Dedans : identique à `cell_at`.
        assert_eq!(cell_at_clamped(rect, metrics(), 4, 114.0, 74.0), (1, 1));
        // Hors du panneau, dans chaque direction.
        assert_eq!(cell_at_clamped(rect, metrics(), 4, -50.0, -50.0), (0, 0));
        assert_eq!(cell_at_clamped(rect, metrics(), 4, 900.0, 900.0), (9, 1));
        assert_eq!(cell_at_clamped(rect, metrics(), 4, 150.0, 900.0), (4, 1));
        assert_eq!(cell_at_clamped(rect, metrics(), 4, 900.0, 60.0), (9, 0));
        // Grille dégénérée : jamais de dépassement.
        assert_eq!(
            cell_at_clamped(PixelRect::new(0, 0, 5, 5), metrics(), 0, 50.0, 50.0),
            (0, 0)
        );
    }
}
