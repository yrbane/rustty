//! Taquets de tabulation horizontaux.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabStops {
    stops: Vec<bool>,
}

impl TabStops {
    /// Taquets par défaut : toutes les 8 colonnes.
    pub fn new(cols: usize) -> Self {
        Self {
            stops: (0..cols).map(|c| c % 8 == 0).collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.stops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stops.is_empty()
    }

    /// Premier taquet strictement après `col`.
    pub fn next_after(&self, col: usize) -> Option<usize> {
        (col + 1..self.stops.len()).find(|&c| self.stops[c])
    }

    pub fn set(&mut self, col: usize) {
        if let Some(s) = self.stops.get_mut(col) {
            *s = true;
        }
    }

    pub fn clear(&mut self, col: usize) {
        if let Some(s) = self.stops.get_mut(col) {
            *s = false;
        }
    }

    pub fn clear_all(&mut self) {
        self.stops.fill(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_stops_every_eight_columns() {
        let t = TabStops::new(20);
        assert_eq!(t.next_after(0), Some(8));
        assert_eq!(t.next_after(8), Some(16));
        assert_eq!(t.next_after(16), None);
        assert_eq!(t.len(), 20);
    }

    #[test]
    fn set_clear_and_clear_all() {
        let mut t = TabStops::new(20);
        t.set(3);
        assert_eq!(t.next_after(0), Some(3));
        t.clear(3);
        assert_eq!(t.next_after(0), Some(8));
        t.clear_all();
        assert_eq!(t.next_after(0), None);
        t.set(99);
        assert_eq!(t.next_after(0), None, "hors grille : ignoré");
    }
}
