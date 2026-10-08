//! Taille du pseudo-terminal, en cellules et en pixels.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PtySize {
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl PtySize {
    /// Taille en cellules seulement ; un terminal de 0 cellule n'existe pas.
    pub fn new(cols: u16, rows: u16) -> Self {
        Self::with_pixels(cols, rows, 0, 0)
    }

    pub fn with_pixels(cols: u16, rows: u16, pixel_width: u16, pixel_height: u16) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
            pixel_width,
            pixel_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_no_pixel_size() {
        let s = PtySize::new(80, 24);
        assert_eq!(
            (s.cols, s.rows, s.pixel_width, s.pixel_height),
            (80, 24, 0, 0)
        );
    }

    #[test]
    fn zero_cells_are_clamped_to_one() {
        let s = PtySize::new(0, 0);
        assert_eq!((s.cols, s.rows), (1, 1));
        assert_eq!(PtySize::with_pixels(0, 5, 10, 10).cols, 1);
    }
}
