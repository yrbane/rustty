//! Bandes d'image rattachées à une ligne : accès et invalidation par plage.

use std::ops::Range;

use crate::graphics::ImageStrip;
use crate::line::Line;

impl Line {
    /// Bandes de la ligne ; invariant : chaque `placement.cols` vaut au moins 1.
    pub fn images(&self) -> &[ImageStrip] {
        &self.images
    }

    /// Ajoute une bande ; `strip.placement.cols` doit valoir au moins 1.
    pub fn push_image(&mut self, strip: ImageStrip) {
        self.images.push(strip);
    }

    /// Retire les bandes dont l'étendue `[col, col + cols)` coupe `range`.
    pub(crate) fn drop_strips_in(&mut self, range: Range<usize>) {
        self.images.retain(|s| {
            let start = s.col as usize;
            let end = start + s.placement.cols as usize;
            end <= range.start || start >= range.end
        });
    }

    /// Retire les bandes qui commencent à partir de `cols` (réduction).
    pub(crate) fn drop_strips_from(&mut self, cols: usize) {
        self.images.retain(|s| (s.col as usize) < cols);
    }
}

#[cfg(test)]
mod tests {
    use crate::cell::Cell;
    use crate::graphics::{ImageData, ImageStrip, Placement};
    use crate::line::Line;
    use std::sync::Arc;

    fn placement(cols: u16) -> Arc<Placement> {
        Arc::new(Placement {
            image: Arc::new(ImageData::new(1, 1, vec![0; 4])),
            cols,
            rows: 1,
            width_cells: cols as f32,
            height_cells: 1.0,
        })
    }

    fn strip(p: &Arc<Placement>, col: u16) -> ImageStrip {
        ImageStrip {
            placement: p.clone(),
            col,
            row: 0,
        }
    }

    #[test]
    fn reset_and_erase_drop_overlapping_strips() {
        let mut l = Line::new(10);
        l.push_image(strip(&placement(2), 3));
        l.erase_range(4..6, Cell::default());
        assert!(l.images().is_empty());
        l.push_image(strip(&placement(2), 3));
        l.reset(Cell::default());
        assert!(l.images().is_empty());
        l.push_image(strip(&placement(3), 2));
        l.insert_blank(4, 1, Cell::default());
        assert!(l.images().is_empty());
        l.push_image(strip(&placement(3), 2));
        l.delete(4, 1, Cell::default());
        assert!(l.images().is_empty());
    }

    #[test]
    fn erase_outside_keeps_the_strip() {
        let mut l = Line::new(10);
        l.push_image(strip(&placement(2), 3));
        l.erase_range(5..8, Cell::default());
        l.erase_range(0..3, Cell::default());
        l.insert_blank(5, 1, Cell::default());
        l.delete(5, 1, Cell::default());
        assert_eq!(l.images().len(), 1);
        l.set(3, Cell::default());
        *l.get_mut(4) = Cell::default();
        assert_eq!(l.images().len(), 1);
    }

    #[test]
    fn resize_keeps_strips_that_start_inside() {
        let mut l = Line::new(10);
        l.push_image(strip(&placement(4), 3));
        l.push_image(strip(&placement(2), 6));
        l.resize(5, Cell::default());
        assert_eq!(l.images().len(), 1);
        assert_eq!(l.images()[0].col, 3);
    }

    #[test]
    fn lines_with_the_same_strip_are_equal() {
        let p = placement(2);
        let mut a = Line::new(4);
        let mut b = Line::new(4);
        a.push_image(strip(&p, 1));
        b.push_image(strip(&p, 1));
        assert_eq!(a, b);
        let mut c = Line::new(4);
        c.push_image(strip(&placement(2), 1));
        assert_ne!(a, c);
        assert_eq!(a.clone(), a);
    }
}
