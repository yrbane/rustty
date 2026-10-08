//! Positionnement du curseur : contrôles C0 (CR, BS, HT) puis séquences CSI.

use super::Term;
use crate::cursor::SavedCursor;

impl Term {
    pub(crate) fn carriage_return(&mut self) {
        self.cursor.col = 0;
        self.cursor.pending_wrap = false;
    }

    pub(crate) fn backspace(&mut self) {
        self.cursor.pending_wrap = false;
        self.cursor.col = self.cursor.col.saturating_sub(1);
    }

    pub(crate) fn horizontal_tab(&mut self) {
        self.cursor.pending_wrap = false;
        let last = self.cols() - 1;
        self.cursor.col = self.tabs.next_after(self.cursor.col).unwrap_or(last);
    }
}

impl Term {
    /// Borne haute pour un déplacement vertical : la région si le curseur y est
    /// ou en dessous, l'écran s'il est au-dessus (comportement xterm).
    fn upper_bound(&self) -> usize {
        if self.cursor.row >= self.region.top {
            self.region.top
        } else {
            0
        }
    }

    fn lower_bound(&self) -> usize {
        if self.cursor.row <= self.region.bottom {
            self.region.bottom
        } else {
            self.rows() - 1
        }
    }

    pub(crate) fn cursor_up(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        let bound = self.upper_bound();
        self.cursor.row = self.cursor.row.saturating_sub(n).max(bound);
    }

    pub(crate) fn cursor_down(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        let bound = self.lower_bound();
        self.cursor.row = self.cursor.row.saturating_add(n).min(bound);
    }

    pub(crate) fn cursor_forward(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        self.cursor.col = self.cursor.col.saturating_add(n).min(self.cols() - 1);
    }

    pub(crate) fn cursor_back(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    /// CHA / HPA : colonne absolue 0-indexée.
    pub(crate) fn cursor_to_col(&mut self, col: usize) {
        self.cursor.pending_wrap = false;
        self.cursor.col = col.min(self.cols() - 1);
    }

    /// Position absolue 0-indexée ; en mode origine, `row` est relatif à la
    /// région et y reste borné.
    pub(crate) fn cursor_to(&mut self, col: usize, row: usize) {
        self.cursor.pending_wrap = false;
        let (min_row, max_row) = if self.modes.origin {
            (self.region.top, self.region.bottom)
        } else {
            (0, self.rows() - 1)
        };
        self.cursor.row = (min_row + row).min(max_row);
        self.cursor.col = col.min(self.cols() - 1);
    }

    /// DECSTBM : paramètres 1-indexés, 0 = défaut ; région invalide ignorée.
    pub(crate) fn set_scroll_region(&mut self, top1: u16, bottom1: u16) {
        let rows = self.rows();
        if self.region.try_set(top1, bottom1, rows) {
            self.cursor_to(0, 0);
        }
    }

    pub(crate) fn save_cursor(&mut self) {
        let saved = SavedCursor {
            cursor: self.cursor,
            origin: self.modes.origin,
            charsets: self.charsets,
        };
        if self.modes.alt_screen {
            self.saved_cursor_alt = saved
        } else {
            self.saved_cursor = saved
        }
    }

    pub(crate) fn restore_cursor(&mut self) {
        let saved = if self.modes.alt_screen {
            self.saved_cursor_alt
        } else {
            self.saved_cursor
        };
        self.cursor = saved.cursor;
        self.modes.origin = saved.origin;
        self.charsets = saved.charsets;
        let (cols, rows) = (self.cols(), self.rows());
        self.cursor.clamp(cols, rows);
    }
}

#[cfg(test)]
mod tests {
    use crate::term::test_support::{feed, term};

    #[test]
    fn carriage_return_cancels_pending_wrap() {
        let mut t = term(5, 2);
        feed(&mut t, "abcde\r\nx");
        assert_eq!(t.text(), vec!["abcde", "x"]);
        assert!(!t.grid().line(0).wrapped);
    }

    #[test]
    fn backspace_stops_at_first_column() {
        let mut t = term(5, 1);
        feed(&mut t, "ab\x08\x08\x08x");
        assert_eq!(t.text(), vec!["xb"]);
    }

    #[test]
    fn horizontal_tab_goes_to_next_stop_and_stays_at_edge() {
        let mut t = term(20, 1);
        feed(&mut t, "\t");
        assert_eq!(t.cursor().col, 8);
        feed(&mut t, "\t\t\t");
        assert_eq!(t.cursor().col, 19);
    }

    #[test]
    fn cup_is_one_indexed_and_clamped() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[3;4H");
        assert_eq!((t.cursor().col, t.cursor().row), (3, 2));
        feed(&mut t, "\x1b[99;99H");
        assert_eq!((t.cursor().col, t.cursor().row), (9, 4));
        feed(&mut t, "\x1b[H");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 0));
    }

    #[test]
    fn csi_zero_and_huge_params_are_clamped() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[2;2H\x1b[0A");
        assert_eq!(t.cursor().row, 0, "0 vaut 1");
        feed(&mut t, "\x1b[99999C");
        assert_eq!(t.cursor().col, 9);
        feed(&mut t, "\x1b[;;H");
        assert_eq!(
            (t.cursor().col, t.cursor().row),
            (0, 0),
            "paramètres vides = défauts"
        );
    }

    #[test]
    fn relative_moves() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[3;3H\x1b[B\x1b[2C\x1b[A\x1b[D");
        assert_eq!((t.cursor().col, t.cursor().row), (3, 2));
        feed(&mut t, "\x1b[5G\x1b[2d");
        assert_eq!((t.cursor().col, t.cursor().row), (4, 1), "CHA et VPA");
        feed(&mut t, "\x1b[E\x1b[F");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 1), "CNL puis CPL");
    }

    #[test]
    fn vertical_moves_stop_at_region_edges_like_xterm() {
        let mut t = term(10, 6);
        feed(&mut t, "\x1b[2;5r\x1b[3;1H\x1b[9A");
        assert_eq!(t.cursor().row, 1, "dans la région, on s'arrête à son haut");
        feed(&mut t, "\x1b[1;1H\x1b[9B");
        assert_eq!(
            t.cursor().row,
            4,
            "depuis le dessus de la région, on s'arrête à son bas"
        );
        feed(&mut t, "\x1b[6;1H\x1b[9A");
        assert_eq!(
            t.cursor().row,
            1,
            "depuis le dessous, on s'arrête à son haut"
        );
    }

    #[test]
    fn decstbm_moves_cursor_home_and_scrolls_within_region() {
        let mut t = term(3, 4);
        feed(&mut t, "a\r\nb\r\nc\r\nd\x1b[2;3r");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 0));
        feed(&mut t, "\x1b[3;1H\n");
        assert_eq!(t.text(), vec!["a", "c", "", "d"]);
        assert_eq!(
            t.scrollback().len(),
            0,
            "une région qui ne touche pas le haut n'alimente pas l'historique"
        );
    }

    #[test]
    fn decstbm_invalid_region_is_ignored() {
        let mut t = term(3, 4);
        feed(&mut t, "\x1b[10;5r");
        assert_eq!((t.region.top, t.region.bottom), (0, 3));
        feed(&mut t, "\x1b[1;999r");
        assert_eq!(
            (t.region.top, t.region.bottom),
            (0, 3),
            "bas hors écran : borné à l'écran"
        );
        feed(&mut t, "\x1b[3;3r");
        assert_eq!(
            (t.region.top, t.region.bottom),
            (0, 3),
            "région d'une ligne : ignorée"
        );
        feed(&mut t, "\x1b[r");
        assert_eq!((t.region.top, t.region.bottom), (0, 3));
    }

    #[test]
    fn origin_mode_makes_cup_relative_to_region() {
        let mut t = term(5, 6);
        t.modes.origin = true;
        feed(&mut t, "\x1b[3;5r\x1b[1;1H");
        assert_eq!(t.cursor().row, 2);
        feed(&mut t, "\x1b[99;1H");
        assert_eq!(t.cursor().row, 4, "borné à la région");
    }

    #[test]
    fn save_and_restore_cursor_with_style() {
        let mut t = term(10, 3);
        feed(&mut t, "\x1b[2;3H\x1b7\x1b[1;1Hzz\x1b8");
        assert_eq!((t.cursor().col, t.cursor().row), (2, 1));
        feed(&mut t, "\x1b[s\x1b[3;9H\x1b[u");
        assert_eq!(
            (t.cursor().col, t.cursor().row),
            (2, 1),
            "CSI s/u équivalents"
        );
    }

    #[test]
    fn moves_cancel_pending_wrap() {
        let mut t = term(3, 2);
        feed(&mut t, "abc\x1b[Dx");
        assert_eq!(t.text(), vec!["axc", ""]);
    }
}
