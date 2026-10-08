//! Région de défilement DECSTBM, bornes incluses, indexée à 0.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollRegion {
    pub top: usize,
    pub bottom: usize,
}

impl ScrollRegion {
    pub fn full(rows: usize) -> Self {
        Self {
            top: 0,
            bottom: rows.saturating_sub(1),
        }
    }

    pub fn is_full(&self, rows: usize) -> bool {
        *self == Self::full(rows)
    }

    pub fn height(&self) -> usize {
        self.bottom - self.top + 1
    }

    pub fn contains(&self, row: usize) -> bool {
        (self.top..=self.bottom).contains(&row)
    }

    /// Applique des paramètres DECSTBM 1-indexés (0 = défaut). Le bas est
    /// borné à l'écran ; une région inversée ou d'une seule ligne est refusée.
    pub fn try_set(&mut self, top1: u16, bottom1: u16, rows: usize) -> bool {
        let top = if top1 == 0 { 1 } else { usize::from(top1) };
        let bottom = if bottom1 == 0 {
            rows
        } else {
            usize::from(bottom1).min(rows)
        };
        if top >= bottom {
            return false;
        }
        self.top = top - 1;
        self.bottom = bottom - 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_region_covers_screen() {
        let r = ScrollRegion::full(5);
        assert_eq!((r.top, r.bottom), (0, 4));
        assert!(r.is_full(5));
        assert_eq!(r.height(), 5);
        assert!(r.contains(0) && r.contains(4) && !r.contains(5));
    }

    #[test]
    fn try_set_accepts_valid_one_indexed_bounds() {
        let mut r = ScrollRegion::full(6);
        assert!(r.try_set(2, 5, 6));
        assert_eq!((r.top, r.bottom), (1, 4));
        assert!(r.try_set(0, 0, 6), "0;0 = tout l'écran");
        assert!(r.is_full(6));
    }

    #[test]
    fn try_set_rejects_inverted_or_degenerate_and_clamps_bottom() {
        let mut r = ScrollRegion::full(4);
        assert!(!r.try_set(10, 5, 4));
        assert!(!r.try_set(3, 3, 4), "une ligne : refusé");
        assert!(r.is_full(4), "inchangée après refus");
        assert!(r.try_set(1, 999, 4));
        assert_eq!((r.top, r.bottom), (0, 3), "bas borné à l'écran");
    }
}
