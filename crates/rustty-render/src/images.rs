//! Géométrie des images : des bandes rattachées aux lignes d'un panneau aux
//! rectangles en pixels à dessiner, avec leurs coordonnées de texture.
//! Pur, sans GPU.

use std::sync::Arc;

use rustty_vt::graphics::ImageData;

use crate::font::CellMetrics;
use crate::frame::PaneFrame;
use crate::grid::grid_geometry;

/// Une bande d'image à dessiner : où (`x, y, w, h` en pixels) et quelle
/// portion de la texture (`u0, v0, u1, v1`).
#[derive(Clone, Debug)]
pub struct ImageDraw {
    pub image: Arc<ImageData>,
    pub dest: [f32; 4],
    pub uv: [f32; 4],
}

/// Les bandes d'image visibles du panneau. Une bande couvre au plus une
/// cellule de haut (la dernière peut être partielle) ; celles qui débordent
/// à droite du panneau sont rognées, celles qui commencent au-delà ignorées.
pub fn pane_images(pane: &PaneFrame, metrics: CellMetrics, padding: u32) -> Vec<ImageDraw> {
    let g = grid_geometry(pane.rect, metrics, padding);
    let (cw, ch) = (metrics.width as f32, metrics.height as f32);
    let right = (pane.rect.x + pane.rect.width) as f32;
    let rows = g.rows.min(pane.snapshot.rows);
    let mut draws = Vec::new();
    for (r, line) in pane.snapshot.lines.iter().enumerate().take(rows) {
        for strip in line.images() {
            let p = &strip.placement;
            let top = f32::from(strip.row);
            let bottom = (top + 1.0).min(p.height_cells);
            if bottom <= top {
                continue;
            }
            let x = g.origin_x as f32 + f32::from(strip.col) * cw;
            let y = g.origin_y as f32 + r as f32 * ch;
            if x >= right {
                continue;
            }
            let (mut w, h) = (p.width_cells * cw, (bottom - top) * ch);
            let mut u1 = 1.0;
            if x + w > right {
                u1 = (right - x) / w;
                w = right - x;
            }
            draws.push(ImageDraw {
                image: p.image.clone(),
                dest: [x, y, w, h],
                uv: [0.0, top / p.height_cells, u1, bottom / p.height_cells],
            });
        }
    }
    draws
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::PixelRect;
    use rustty_vt::Term;
    use rustty_vt::graphics::{ImageStrip, Placement};

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

    fn placement(cols: u16, rows: u16, width_cells: f32, height_cells: f32) -> Arc<Placement> {
        Arc::new(Placement {
            image: Arc::new(ImageData::new(1, 1, vec![0; 4])),
            cols,
            rows,
            width_cells,
            height_cells,
        })
    }

    fn strip(p: &Arc<Placement>, col: u16, row: u16) -> ImageStrip {
        ImageStrip {
            placement: p.clone(),
            col,
            row,
        }
    }

    fn draws(snapshot: &rustty_vt::Snapshot, rect: PixelRect) -> Vec<ImageDraw> {
        let pane = PaneFrame {
            rect,
            snapshot,
            focused: true,
        };
        pane_images(&pane, metrics(), 0)
    }

    fn assert_close(actual: [f32; 4], expected: [f32; 4]) {
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-3, "{actual:?} au lieu de {expected:?}");
        }
    }

    #[test]
    fn strip_rows_map_to_image_bands() {
        let mut snap = Term::new(10, 3, 0).snapshot();
        let p = placement(2, 2, 2.0, 1.5);
        snap.lines[0].push_image(strip(&p, 1, 0));
        snap.lines[1].push_image(strip(&p, 1, 1));
        let d = draws(&snap, PixelRect::new(0, 0, 100, 60));
        assert_eq!(d.len(), 2);
        assert_close(d[0].dest, [10.0, 0.0, 20.0, 20.0]);
        assert_close(d[0].uv, [0.0, 0.0, 1.0, 1.0 / 1.5]);
        assert_close(d[1].dest, [10.0, 20.0, 20.0, 10.0]);
        assert_close(d[1].uv, [0.0, 0.667, 1.0, 1.0]);
        assert!(Arc::ptr_eq(&d[0].image, &p.image));
    }

    #[test]
    fn strips_are_clipped_to_the_pane() {
        let mut snap = Term::new(4, 2, 0).snapshot();
        let p = placement(4, 1, 4.0, 1.0);
        snap.lines[1].push_image(strip(&p, 2, 0));
        let d = draws(&snap, PixelRect::new(100, 50, 40, 40));
        assert_eq!(d.len(), 1);
        assert_close(d[0].dest, [120.0, 70.0, 20.0, 20.0]);
        assert_close(d[0].uv, [0.0, 0.0, 0.5, 1.0]);
    }

    #[test]
    fn strips_left_of_the_pane_right_edge_only() {
        let mut snap = Term::new(4, 2, 0).snapshot();
        let p = placement(2, 1, 2.0, 1.0);
        snap.lines[0].push_image(strip(&p, 4, 0));
        snap.lines[0].push_image(strip(&p, 7, 0));
        snap.lines[1].push_image(strip(&p, 0, 0));
        let d = draws(&snap, PixelRect::new(100, 50, 40, 40));
        assert_eq!(d.len(), 1, "seule la bande dans le panneau est dessinée");
        assert_close(d[0].dest, [100.0, 70.0, 20.0, 20.0]);
    }

    #[test]
    fn lines_below_the_pane_grid_are_not_drawn() {
        let mut snap = Term::new(4, 3, 0).snapshot();
        let p = placement(1, 1, 1.0, 1.0);
        snap.lines[2].push_image(strip(&p, 0, 0));
        // Le panneau ne loge que deux rangées : la troisième ligne est hors champ.
        let d = draws(&snap, PixelRect::new(0, 0, 40, 40));
        assert!(d.is_empty());
    }
}
