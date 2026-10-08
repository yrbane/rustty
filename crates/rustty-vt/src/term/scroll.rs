//! Défilement vertical : saut de ligne, et déplacement de la région dans les
//! deux sens avec alimentation de l'historique quand la région touche le haut.

use super::Term;

impl Term {
    pub(crate) fn scroll_up_region(&mut self, n: usize) {
        let template = self.erase_template();
        let (top, bottom) = (self.region.top, self.region.bottom);
        let keep_history = top == 0 && !self.modes.alt_screen;
        let evicted = self.active_grid_mut().scroll_up(top, bottom, n, template);
        if keep_history {
            self.scrollback.extend(evicted);
        }
    }

    pub(crate) fn scroll_down_region(&mut self, n: usize) {
        let template = self.erase_template();
        let (top, bottom) = (self.region.top, self.region.bottom);
        self.active_grid_mut().scroll_down(top, bottom, n, template);
    }

    /// RI : monte d'une ligne, fait défiler vers le bas en haut de région.
    pub(crate) fn reverse_index(&mut self) {
        self.cursor.pending_wrap = false;
        if self.cursor.row == self.region.top {
            self.scroll_down_region(1);
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
    }

    /// LF / VT / FF / IND : descend d'une ligne, fait défiler en bas de région.
    pub(crate) fn linefeed(&mut self) {
        self.cursor.pending_wrap = false;
        if self.cursor.row == self.region.bottom {
            self.scroll_up_region(1);
        } else if self.cursor.row + 1 < self.rows() {
            self.cursor.row += 1;
        }
        if self.modes.line_feed_new_line {
            self.cursor.col = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::term::test_support::{feed, term};

    #[test]
    fn line_feed_keeps_column_and_moves_down() {
        let mut t = term(10, 3);
        feed(&mut t, "ab\n");
        assert_eq!((t.cursor().col, t.cursor().row), (2, 1));
    }

    #[test]
    fn line_feed_at_bottom_scrolls_into_scrollback() {
        let mut t = term(3, 2);
        feed(&mut t, "a\r\nb\r\nc");
        assert_eq!(t.text(), vec!["b", "c"]);
        assert_eq!(t.scrollback().len(), 1);
        assert_eq!(t.scrollback().get(0).unwrap().text(), "a  ");
    }

    #[test]
    fn line_feed_new_line_mode_also_returns_carriage() {
        let mut t = term(10, 2);
        t.modes.line_feed_new_line = true;
        feed(&mut t, "ab\n");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 1));
    }

    #[test]
    fn scrolling_a_partial_region_does_not_feed_history() {
        let mut t = term(1, 3);
        t.region.try_set(2, 3, 3);
        t.cursor.row = 2;
        feed(&mut t, "\n\n");
        assert!(t.scrollback().is_empty());
    }

    #[test]
    fn index_and_reverse_index_scroll_at_region_edges() {
        let mut t = term(1, 3);
        feed(&mut t, "a\r\nb\r\nc\x1bD");
        assert_eq!(t.text(), vec!["b", "c", ""]);
        feed(&mut t, "\x1b[1;1H\x1bM");
        assert_eq!(t.text(), vec!["", "b", "c"]);
    }

    #[test]
    fn next_line_moves_to_first_column_of_next_row() {
        let mut t = term(5, 2);
        feed(&mut t, "abc\x1bEx");
        assert_eq!(t.text(), vec!["abc", "x"]);
    }
}
