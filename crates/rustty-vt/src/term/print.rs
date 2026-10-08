//! Écriture des caractères imprimables : largeur, retour à la ligne différé,
//! mode insertion, caractères combinants et caractères larges sur deux cellules.

use unicode_width::UnicodeWidthChar;

use super::Term;
use crate::cell::{Attrs, Cell};

impl Term {
    /// Point d'entrée depuis `Perform::print`.
    pub(crate) fn print_char(&mut self, c: char) {
        let c = self.charsets.map(c);
        match c.width().unwrap_or(1) {
            0 => self.put_zerowidth(c),
            2 => self.put_char(c, 2),
            _ => self.put_char(c, 1),
        }
    }

    /// Si la cellule `col` est une moitié de caractère large, efface l'autre moitié.
    fn clear_wide_partner(&mut self, col: usize, row: usize) {
        let template = self.erase_template();
        let line = self.active_grid_mut().line_mut(row);
        if line.get(col).is_wide() && col + 1 < line.len() {
            line.set(col + 1, template);
        } else if line.get(col).is_wide_continuation() && col > 0 {
            line.set(col - 1, template);
        }
    }

    /// Écrit un caractère de largeur `width` (1 ou 2) sous le curseur.
    pub(crate) fn put_char(&mut self, c: char, width: usize) {
        let cols = self.cols();
        // Une grille d'une colonne ne peut pas contenir un large : on le traite
        // comme étroit plutôt que d'indexer hors de la ligne.
        let width = if width == 2 && cols < 2 { 1 } else { width };
        if self.cursor.pending_wrap {
            if self.modes.autowrap {
                self.wrap_line();
            } else {
                self.cursor.pending_wrap = false;
            }
        }
        if width == 2 && self.cursor.col + 1 >= cols {
            if !self.modes.autowrap {
                return; // pas de place et pas de retour à la ligne : abandonné
            }
            // Une seule colonne libre : on la blanchit et on passe à la ligne.
            let (col, row) = (self.cursor.col, self.cursor.row);
            let template = self.erase_template();
            self.active_grid_mut().line_mut(row).set(col, template);
            self.wrap_line();
        }
        let (col, row) = (self.cursor.col, self.cursor.row);
        let template = self.erase_template();
        if self.modes.insert {
            self.active_grid_mut()
                .line_mut(row)
                .insert_blank(col, width, template);
        }
        self.clear_wide_partner(col, row);
        if width == 2 {
            self.clear_wide_partner(col + 1, row);
        }
        let mut style = self.cursor.style;
        if width == 2 {
            style.attrs.insert(Attrs::WIDE);
        }
        let line = self.active_grid_mut().line_mut(row);
        line.set(col, Cell::new(c, style));
        if width == 2 {
            let mut cont = template;
            cont.style.attrs.insert(Attrs::WIDE_CONTINUATION);
            line.set(col + 1, cont);
        }
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

    #[test]
    fn wide_char_takes_two_cells() {
        let mut t = term(5, 1);
        feed(&mut t, "漢a");
        assert!(t.grid().cell(0, 0).is_wide());
        assert!(t.grid().cell(1, 0).is_wide_continuation());
        assert_eq!(t.grid().cell(2, 0).c, 'a');
        assert_eq!(t.text(), vec!["漢a"]);
        assert_eq!(t.cursor().col, 3);
    }

    #[test]
    fn wide_char_at_last_column_wraps() {
        let mut t = term(4, 2);
        feed(&mut t, "abc漢");
        assert_eq!(t.text(), vec!["abc", "漢"]);
        assert_eq!(
            t.grid().cell(3, 0).c,
            ' ',
            "la dernière cellule de la ligne 1 est un blanc"
        );
        assert!(t.grid().line(0).wrapped);
        assert_eq!(t.cursor().col, 2);
    }

    #[test]
    fn wide_char_at_last_column_without_autowrap_is_dropped() {
        let mut t = term(4, 1);
        t.modes.autowrap = false;
        feed(&mut t, "abc漢x");
        assert_eq!(
            t.text(),
            vec!["abcx"],
            "le large n'a pas tenu et est abandonné, x prend la dernière colonne"
        );
    }

    #[test]
    fn overwriting_half_of_a_wide_char_clears_the_other_half() {
        let mut t = term(5, 1);
        feed(&mut t, "漢漢");
        t.cursor.col = 1;
        feed(&mut t, "x");
        assert_eq!(t.text(), vec![" x漢"]);
        assert!(!t.grid().cell(0, 0).is_wide());
        t.cursor.col = 2;
        feed(&mut t, "y");
        assert_eq!(t.text(), vec![" xy"]);
        assert!(!t.grid().cell(3, 0).is_wide_continuation());
    }

    #[test]
    fn wide_char_fills_exactly_the_last_two_columns() {
        let mut t = term(4, 1);
        feed(&mut t, "ab漢");
        assert_eq!(t.text(), vec!["ab漢"]);
        assert!(t.cursor().pending_wrap);
    }

    #[test]
    fn combining_char_after_wide_char_attaches_to_its_first_half() {
        let mut t = term(5, 1);
        feed(&mut t, "漢\u{301}");
        assert_eq!(t.grid().line(0).zerowidth(0), Some("\u{301}"));
    }

    #[test]
    fn wide_char_in_single_column_grid_is_narrowed_not_panicking() {
        let mut t = term(1, 2);
        feed(&mut t, "漢x");
        assert_eq!(t.text(), vec!["漢", "x"]);
    }
}
