//! Remise à l'état initial (RIS) et redimensionnement : les deux opérations qui
//! refondent l'état entier.

use super::Term;
use crate::cell::Cell;
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
        self.display_offset = 0;
    }

    /// Nouvelle taille en cellules. Les lignes sont tronquées ou complétées,
    /// pas réenroulées (prévu en v0.2).
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let template = Cell::default();
        self.grid.resize(cols, rows, template);
        self.alt_grid.resize(cols, rows, template);
        self.scrollback.resize_lines(cols, template);
        self.region = ScrollRegion::full(rows);
        self.tabs = TabStops::new(cols);
        self.display_offset = 0;
        self.cursor.clamp(cols, rows);
        self.saved_cursor.cursor.clamp(cols, rows);
        self.saved_cursor_alt.cursor.clamp(cols, rows);
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

    #[test]
    fn resize_grow_keeps_content_and_cursor() {
        let mut t = term(3, 2);
        feed(&mut t, "abc\r\nd");
        t.resize(5, 4);
        assert_eq!(t.text(), vec!["abc", "d", "", ""]);
        assert_eq!((t.cursor().col, t.cursor().row), (1, 1));
        assert_eq!((t.region.top, t.region.bottom), (0, 3));
    }

    #[test]
    fn resize_shrink_clamps_cursor_and_region() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[2;4r\x1b[4;9H\x1b7");
        t.resize(4, 2);
        assert_eq!((t.cursor().col, t.cursor().row), (3, 1));
        assert_eq!((t.region.top, t.region.bottom), (0, 1));
        feed(&mut t, "\x1b8");
        assert_eq!(
            (t.cursor().col, t.cursor().row),
            (3, 1),
            "le curseur sauvegardé est borné aussi"
        );
        feed(&mut t, "x");
        assert_eq!(
            t.text(),
            vec!["", "   x"],
            "écrire après resize ne panique pas"
        );
    }

    #[test]
    fn resize_applies_to_scrollback_and_alt_screen() {
        let mut t = term(3, 1);
        feed(&mut t, "a\r\nb");
        t.resize(5, 1);
        assert_eq!(t.scrollback().get(0).unwrap().len(), 5);
        feed(&mut t, "\x1b[?1049h");
        assert_eq!((t.grid().cols(), t.grid().rows()), (5, 1));
    }

    #[test]
    fn resize_resets_tabs_and_pending_wrap() {
        let mut t = term(3, 1);
        feed(&mut t, "abc");
        assert!(t.cursor().pending_wrap);
        t.resize(20, 1);
        assert!(!t.cursor().pending_wrap);
        feed(&mut t, "\x1b[1;1H\t\t");
        assert_eq!(t.cursor().col, 16);
    }

    #[test]
    fn resize_to_zero_is_clamped_to_one() {
        let mut t = term(3, 1);
        t.resize(0, 0);
        assert_eq!((t.grid().cols(), t.grid().rows()), (1, 1));
    }
}
