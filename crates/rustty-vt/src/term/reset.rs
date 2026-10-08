//! Remise à l'état initial (RIS) et, en tâche 15, redimensionnement.

use super::Term;
use crate::charset::Charsets;
use crate::cursor::{Cursor, CursorShape, SavedCursor};
use crate::grid::Grid;
use crate::modes::Modes;
use crate::region::ScrollRegion;
use crate::tabs::TabStops;

impl Term {
    /// RIS : état initial, taille et historique conservés.
    pub(crate) fn reset(&mut self) {
        let (cols, rows) = (self.cols(), self.rows());
        self.grid = Grid::new(cols, rows);
        self.alt_grid = Grid::new(cols, rows);
        self.cursor = Cursor::default();
        self.saved_cursor = SavedCursor::default();
        self.saved_cursor_alt = SavedCursor::default();
        self.modes = Modes::default();
        self.cursor_shape = CursorShape::default();
        self.region = ScrollRegion::full(rows);
        self.tabs = TabStops::new(cols);
        self.charsets = Charsets::default();
        self.title.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::cursor::Cursor;
    use crate::modes::Modes;
    use crate::term::test_support::{feed, term};

    #[test]
    fn ris_resets_everything_but_keeps_size_and_history() {
        let mut t = term(5, 2);
        feed(
            &mut t,
            "a\r\nb\r\nc\x1b[?25l\x1b[31m\x1b[1;2r\x1b]0;T\x07\x1bc",
        );
        assert_eq!(t.text(), vec!["", ""]);
        assert_eq!(*t.modes(), Modes::default());
        assert_eq!(t.cursor(), Cursor::default());
        assert_eq!((t.region.top, t.region.bottom), (0, 1));
        assert_eq!(t.title(), "");
        assert_eq!((t.grid().cols(), t.grid().rows()), (5, 2));
        assert_eq!(
            t.scrollback().len(),
            1,
            "l'historique survit à RIS, CSI 3 J existe pour lui"
        );
    }
}
