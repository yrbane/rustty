//! Écriture des caractères imprimables : largeur, retour à la ligne différé,
//! mode insertion, caractères combinants. Les caractères larges arrivent en
//! tâche 14.

use unicode_width::UnicodeWidthChar;

use super::Term;
use crate::cell::Cell;

impl Term {
    /// Point d'entrée depuis `Perform::print`.
    pub(crate) fn print_char(&mut self, c: char) {
        let c = self.charsets.map(c);
        match c.width().unwrap_or(1) {
            0 => self.put_zerowidth(c),
            _ => self.put_char(c, 1),
        }
    }

    /// Écrit un caractère de largeur `width` sous le curseur.
    pub(crate) fn put_char(&mut self, c: char, width: usize) {
        let cols = self.cols();
        if self.cursor.pending_wrap {
            if self.modes.autowrap {
                self.wrap_line();
            } else {
                self.cursor.pending_wrap = false;
            }
        }
        let (col, row) = (self.cursor.col, self.cursor.row);
        let cell = Cell::new(c, self.cursor.style);
        let template = self.erase_template();
        let insert = self.modes.insert;
        let line = self.active_grid_mut().line_mut(row);
        if insert {
            line.insert_blank(col, width, template);
        }
        line.set(col, cell);
        if col + width < cols {
            self.cursor.col += width;
        } else {
            self.cursor.col = cols - 1;
            self.cursor.pending_wrap = true;
        }
    }

    /// Attache un caractère combinant à la dernière cellule écrite.
    pub(crate) fn put_zerowidth(&mut self, c: char) {
        let (col, row) = (self.cursor.col, self.cursor.row);
        // Après un caractère en dernière colonne le curseur n'a pas avancé :
        // la cible est la cellule sous le curseur, sinon celle juste avant.
        let target = if self.cursor.pending_wrap || col == 0 {
            col
        } else {
            col - 1
        };
        let line = self.active_grid_mut().line_mut(row);
        let target = if line.get(target).is_wide_continuation() {
            target.saturating_sub(1)
        } else {
            target
        };
        line.push_zerowidth(target, c);
    }

    /// Retour à la ligne implicite : marque la ligne et descend.
    pub(crate) fn wrap_line(&mut self) {
        let row = self.cursor.row;
        self.active_grid_mut().line_mut(row).wrapped = true;
        self.carriage_return();
        self.linefeed();
    }
}

#[cfg(test)]
mod tests {
    use crate::term::test_support::{feed, term};

    #[test]
    fn prints_text_and_advances_cursor() {
        let mut t = term(10, 2);
        feed(&mut t, "hello");
        assert_eq!(t.text(), vec!["hello", ""]);
        assert_eq!((t.cursor().col, t.cursor().row), (5, 0));
    }

    #[test]
    fn autowrap_marks_line_and_continues_on_next_row() {
        let mut t = term(5, 2);
        feed(&mut t, "abcde");
        assert_eq!((t.cursor().col, t.cursor().row), (4, 0));
        assert!(t.cursor().pending_wrap);
        feed(&mut t, "f");
        assert_eq!(t.text(), vec!["abcde", "f"]);
        assert!(t.grid().line(0).wrapped);
        assert!(!t.cursor().pending_wrap);
    }

    #[test]
    fn autowrap_off_overwrites_last_column() {
        let mut t = term(3, 1);
        t.modes.autowrap = false;
        feed(&mut t, "abcdef");
        assert_eq!(t.text(), vec!["abf"]);
        assert_eq!(t.cursor().col, 2);
    }

    #[test]
    fn combining_char_attaches_to_previous_cell() {
        let mut t = term(5, 1);
        feed(&mut t, "e\u{301}x");
        assert_eq!(t.grid().line(0).zerowidth(0), Some("\u{301}"));
        assert_eq!(t.text(), vec!["e\u{301}x"]);
    }

    #[test]
    fn insert_mode_shifts_existing_text() {
        let mut t = term(5, 1);
        feed(&mut t, "abc");
        t.modes.insert = true;
        t.cursor.col = 0;
        feed(&mut t, "X");
        assert_eq!(t.text(), vec!["Xabc"]);
    }
}
