//! Effacement et édition : lignes et caractères, sans déplacer le curseur.

use super::Term;

impl Term {
    pub(crate) fn erase_in_line(&mut self, mode: u16) {
        let (col, row, cols) = (self.cursor.col, self.cursor.row, self.cols());
        let template = self.erase_template();
        let range = match mode {
            0 => col..cols,
            1 => 0..col + 1,
            2 => 0..cols,
            _ => return,
        };
        self.active_grid_mut()
            .line_mut(row)
            .erase_range(range, template);
    }

    pub(crate) fn erase_in_display(&mut self, mode: u16) {
        let (row, rows) = (self.cursor.row, self.rows());
        let template = self.erase_template();
        match mode {
            0 => {
                self.erase_in_line(0);
                for r in row + 1..rows {
                    self.active_grid_mut().line_mut(r).reset(template);
                }
            }
            1 => {
                for r in 0..row {
                    self.active_grid_mut().line_mut(r).reset(template);
                }
                self.erase_in_line(1);
            }
            2 => self.active_grid_mut().clear(template),
            3 => self.scrollback.clear(),
            _ => {}
        }
    }

    pub(crate) fn erase_chars(&mut self, n: usize) {
        let (col, row, cols) = (self.cursor.col, self.cursor.row, self.cols());
        let template = self.erase_template();
        self.active_grid_mut()
            .line_mut(row)
            .erase_range(col..(col + n).min(cols), template);
    }

    pub(crate) fn insert_blank_chars(&mut self, n: usize) {
        let (col, row) = (self.cursor.col, self.cursor.row);
        let template = self.erase_template();
        self.active_grid_mut()
            .line_mut(row)
            .insert_blank(col, n, template);
    }

    pub(crate) fn delete_chars(&mut self, n: usize) {
        let (col, row) = (self.cursor.col, self.cursor.row);
        let template = self.erase_template();
        self.active_grid_mut()
            .line_mut(row)
            .delete(col, n, template);
    }

    pub(crate) fn insert_lines(&mut self, n: usize) {
        if !self.region.contains(self.cursor.row) {
            return;
        }
        let (row, bottom) = (self.cursor.row, self.region.bottom);
        let template = self.erase_template();
        self.active_grid_mut().scroll_down(row, bottom, n, template);
    }

    pub(crate) fn delete_lines(&mut self, n: usize) {
        if !self.region.contains(self.cursor.row) {
            return;
        }
        let (row, bottom) = (self.cursor.row, self.region.bottom);
        let template = self.erase_template();
        // Les lignes supprimées ne vont jamais dans l'historique.
        let _ = self.active_grid_mut().scroll_up(row, bottom, n, template);
    }
}

#[cfg(test)]
mod tests {
    use crate::color::Color;
    use crate::term::test_support::{feed, term};

    #[test]
    fn erase_in_line_modes() {
        let mut t = term(5, 3);
        feed(&mut t, "abcde\x1b[1;3H\x1b[K");
        assert_eq!(t.text()[0], "ab");
        feed(&mut t, "\x1b[2;1Habcde\x1b[2;3H\x1b[1K");
        assert_eq!(t.text()[1], "   de");
        feed(&mut t, "\x1b[3;1Habcde\x1b[2K");
        assert_eq!(t.text()[2], "");
        assert_eq!(t.cursor().row, 2, "EL ne déplace pas le curseur");
    }

    #[test]
    fn erase_in_display_modes() {
        let mut t = term(3, 3);
        feed(&mut t, "aaa\r\nbbb\r\nccc\x1b[2;2H\x1b[J");
        assert_eq!(t.text(), vec!["aaa", "b", ""]);
        feed(&mut t, "\x1b[1;1Haaa\r\nbbb\r\nccc\x1b[2;2H\x1b[1J");
        assert_eq!(t.text(), vec!["", "  b", "ccc"]);
        feed(&mut t, "\x1b[2J");
        assert_eq!(t.text(), vec!["", "", ""]);
        assert_eq!(t.cursor().row, 1, "ED ne déplace pas le curseur");
    }

    #[test]
    fn erase_in_display_3_clears_scrollback() {
        let mut t = term(3, 1);
        feed(&mut t, "a\r\nb\r\nc");
        assert_eq!(t.scrollback().len(), 2);
        feed(&mut t, "\x1b[3J");
        assert!(t.scrollback().is_empty());
        assert_eq!(t.text(), vec!["c"], "l'écran est intact");
    }

    #[test]
    fn erase_uses_current_background() {
        let mut t = term(3, 1);
        feed(&mut t, "abc\x1b[44m\x1b[2K");
        assert_eq!(t.grid().cell(1, 0).style.bg, Color::Indexed(4));
        assert!(t.grid().cell(1, 0).style.attrs.is_empty());
    }

    #[test]
    fn erase_chars_does_not_shift() {
        let mut t = term(5, 1);
        feed(&mut t, "abcde\x1b[1;2H\x1b[2X");
        assert_eq!(t.text(), vec!["a  de"]);
    }

    #[test]
    fn insert_and_delete_chars_shift_within_line() {
        let mut t = term(5, 1);
        feed(&mut t, "abcde\x1b[1;2H\x1b[2@");
        assert_eq!(t.text(), vec!["a  bc"]);
        feed(&mut t, "\x1b[3P");
        assert_eq!(t.text(), vec!["ac"]);
    }

    #[test]
    fn insert_and_delete_lines_respect_scroll_region() {
        let mut t = term(1, 5);
        feed(&mut t, "a\r\nb\r\nc\r\nd\r\ne\x1b[2;4r\x1b[3;1H\x1b[L");
        assert_eq!(t.text(), vec!["a", "b", "", "c", "e"]);
        feed(&mut t, "\x1b[2M");
        assert_eq!(t.text(), vec!["a", "b", "", "", "e"]);
        assert!(
            t.scrollback().is_empty(),
            "DL n'alimente jamais l'historique"
        );
    }

    #[test]
    fn insert_lines_outside_region_is_ignored() {
        let mut t = term(1, 4);
        feed(&mut t, "a\r\nb\r\nc\r\nd\x1b[2;3r\x1b[4;1H\x1b[L");
        assert_eq!(t.text(), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn scroll_up_and_down_commands() {
        let mut t = term(1, 3);
        feed(&mut t, "a\r\nb\r\nc\x1b[S");
        assert_eq!(t.text(), vec!["b", "c", ""]);
        assert_eq!(
            t.scrollback().len(),
            1,
            "SU sur tout l'écran alimente l'historique"
        );
        feed(&mut t, "\x1b[2T");
        assert_eq!(t.text(), vec!["", "", "b"]);
    }
}
