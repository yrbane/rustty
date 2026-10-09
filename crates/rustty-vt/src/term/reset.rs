//! Remise à l'état initial (RIS) et redimensionnement : les deux opérations qui
//! refondent l'état entier.

use super::Term;
use crate::cell::Cell;
use crate::charset::Charsets;
use crate::cursor::{Cursor, CursorShape, SavedCursor};
use crate::grid::Grid;
use crate::modes::Modes;
use crate::reflow::reflow;
use crate::region::ScrollRegion;
use crate::tabs::TabStops;

impl Term {
    /// RIS : état initial, taille et historique conservés.
    pub(crate) fn reset(&mut self) {
        let (cols, rows) = (self.cols(), self.rows());
        let before = self.modes;
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
        self.display_offset = 0;
        self.set_title(String::new());
        self.notify_if_host_modes_changed(before);
    }

    /// Nouvelle taille en cellules. Les lignes sont tronquées ou complétées,
    /// pas réenroulées (prévu en v0.2).
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let template = Cell::default();
        let old_rows = self.rows();
        let mut reflowed_cursor = None;
        let mut reflowed_saved = None;
        if cols != self.cols() {
            // L'écran principal est redécoupé même sous l'écran alternatif : son
            // curseur est alors celui sauvegardé à l'entrée (mode 1049).
            if self.modes.alt_screen {
                let saved = self.saved_cursor.cursor;
                reflowed_saved = Some(self.reflow_primary(cols, rows, saved));
            } else {
                let cursor = self.cursor;
                reflowed_cursor = Some(self.reflow_primary(cols, rows, cursor));
            }
        } else {
            self.scrollback.resize_lines(cols, template);
            if !self.modes.alt_screen && rows < old_rows {
                self.shrink_rows_into_history(rows, template);
            }
            self.grid.resize(cols, rows, template);
            if !self.modes.alt_screen && rows > old_rows {
                self.grow_rows_from_history(rows - old_rows, template);
            }
        }
        self.alt_grid.resize(cols, rows, template);
        self.region = ScrollRegion::full(rows);
        self.tabs = TabStops::new(cols);
        self.display_offset = 0;
        self.cursor.clamp(cols, rows);
        self.saved_cursor.cursor.clamp(cols, rows);
        self.saved_cursor_alt.cursor.clamp(cols, rows);
        if let Some((row, col)) = reflowed_saved {
            self.saved_cursor.cursor.row = row;
            self.saved_cursor.cursor.col = col.min(cols - 1);
        }
        if let Some((row, col)) = reflowed_cursor {
            // Après `clamp`, qui annule le retour à la ligne en attente.
            self.cursor.row = row;
            self.cursor.col = col.min(cols - 1);
            self.cursor.pending_wrap = col >= cols;
        }
    }

    /// Redécoupe l'historique et l'écran principal à `cols` colonnes et rend
    /// la nouvelle position (ligne, colonne) du curseur dans la grille.
    fn reflow_primary(&mut self, cols: usize, rows: usize, cursor: Cursor) -> (usize, usize) {
        let mut lines = self.scrollback.drain_all();
        let history = lines.len();
        let screen = self.grid.lines();
        let used = screen
            .iter()
            .rposition(|l| l.cells().iter().any(|c| *c != Cell::default()))
            .map_or(0, |i| i + 1)
            .max(cursor.row + 1)
            .min(screen.len());
        lines.extend(screen[..used].iter().cloned());
        let cursor_col = cursor.col + usize::from(cursor.pending_wrap);
        let (lines, moved) = reflow(lines, Some((history + cursor.row, cursor_col)), cols);
        let (row, col) = moved.unwrap_or((lines.len().saturating_sub(1), 0));
        // Les `rows` dernières lignes à l'écran : aucun texte n'est perdu. Si
        // le curseur était plus haut, il reste en haut de l'écran (l'application
        // qui l'y a mis redessine à la réception du nouveau format).
        let start = lines.len().saturating_sub(rows);
        let mut lines = lines.into_iter();
        self.scrollback.extend(lines.by_ref().take(start).collect());
        self.grid = Grid::from_lines(cols, rows, lines.take(rows).collect());
        (row.saturating_sub(start), col)
    }

    /// Avant de tronquer le bas de l'écran, pousse assez de lignes du haut dans
    /// l'historique pour que la ligne du curseur reste visible.
    fn shrink_rows_into_history(&mut self, rows: usize, template: Cell) {
        if self.cursor.row < rows {
            return;
        }
        let k = self.cursor.row + 1 - rows;
        let old_bottom = self.rows() - 1;
        let evicted = self.grid.scroll_up(0, old_bottom, k, template);
        self.scrollback.extend(evicted);
        self.cursor.row -= k;
        self.saved_cursor.cursor.row = self.saved_cursor.cursor.row.saturating_sub(k);
    }

    /// Après agrandissement, rapatrie depuis l'historique autant de lignes que
    /// de rangées gagnées, au-dessus du contenu existant.
    fn grow_rows_from_history(&mut self, gained: usize, template: Cell) {
        let k = gained.min(self.scrollback.len());
        if k == 0 {
            return;
        }
        let bottom = self.rows() - 1;
        self.grid.scroll_down(0, bottom, k, template);
        for row in (0..k).rev() {
            if let Some(line) = self.scrollback.pop_newest() {
                *self.grid.line_mut(row) = line;
            }
        }
        self.cursor.row += k;
        self.saved_cursor.cursor.row += k;
    }
}

#[cfg(test)]
mod tests {
    use crate::cursor::Cursor;
    use crate::modes::Modes;
    use crate::term::test_support::{feed, term};

    #[test]
    fn lines_below_the_cursor_are_never_lost() {
        let mut t = term(10, 4);
        feed(
            &mut t,
            "aaaaaaaaaa\r\nbbbbbbbbbb\r\ncccccccccc\r\ndddddddddd\x1b[H",
        );
        t.resize(5, 4);
        let history: Vec<String> = (0..t.scrollback().len())
            .rev()
            .map(|i| t.scrollback().get(i).unwrap().text().trim_end().to_string())
            .collect();
        let all = [history, t.text()].concat().join("|");
        for part in ["aaaaa", "bbbbb", "ccccc", "ddddd"] {
            assert_eq!(all.matches(part).count(), 2, "{part} perdu : {all}");
        }
    }

    #[test]
    fn alt_screen_resize_reflows_the_primary_screen() {
        let mut t = term(10, 3);
        feed(&mut t, "0123456789AB\x1b[?1049hvim");
        t.resize(6, 3);
        feed(&mut t, "\x1b[?1049l");
        assert_eq!(
            t.text(),
            ["012345", "6789AB", ""],
            "l'écran principal n'est pas tronqué"
        );
    }

    #[test]
    fn resize_narrower_reflows_screen_and_history() {
        let mut t = term(10, 3);
        feed(&mut t, "0123456789AB");
        t.resize(6, 3);
        assert_eq!(t.text(), ["012345", "6789AB", ""]);
        let c = t.cursor();
        assert_eq!(
            (c.row, c.col, c.pending_wrap),
            (1, 5, true),
            "après le B, retour à la ligne en attente"
        );
    }

    #[test]
    fn resize_wider_unwraps() {
        let mut t = term(4, 3);
        feed(&mut t, "abcdef");
        assert_eq!(t.text(), ["abcd", "ef", ""]);
        t.resize(8, 3);
        assert_eq!(t.text(), ["abcdef", "", ""]);
        let c = t.cursor();
        assert_eq!((c.row, c.col), (0, 6));
    }

    #[test]
    fn history_is_reflowed_too() {
        let mut t = term(4, 2);
        feed(&mut t, "abcdefgh\r\nzz\r\nyy");
        assert_eq!(
            t.scrollback().len(),
            2,
            "abcd et efgh sont sortis par le haut"
        );
        t.resize(8, 2);
        assert_eq!(t.text(), ["zz", "yy"]);
        assert_eq!(t.scrollback().len(), 1);
        assert_eq!(t.scrollback().get(0).unwrap().text().trim_end(), "abcdefgh");
    }

    #[test]
    fn alt_screen_is_not_reflowed() {
        let mut t = term(4, 2);
        feed(&mut t, "\x1b[?1049habcdef");
        t.resize(8, 2);
        assert_eq!(
            t.text(),
            ["abcd", "ef"],
            "l'application redessine elle-même l'écran alternatif"
        );
    }

    #[test]
    fn cursor_stays_visible_after_reflow() {
        let mut t = term(10, 2);
        feed(&mut t, "0123456789\r\n0123456789\r\nab");
        t.resize(3, 2);
        let c = t.cursor();
        assert!(c.row < 2);
        assert_eq!(t.text()[c.row], "ab");
        assert!(
            t.scrollback().len() >= 6,
            "les lignes repliées sont parties dans l'historique"
        );
    }

    #[test]
    fn rows_only_change_keeps_previous_behavior() {
        let mut t = term(5, 2);
        feed(&mut t, "a\r\nb\r\nc");
        t.resize(5, 3);
        assert_eq!(
            t.text(),
            ["a", "b", "c"],
            "une ligne rapatriée de l'historique"
        );
    }

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

    #[test]
    fn resize_shrink_rows_keeps_cursor_line_by_scrolling_into_history() {
        let mut t = term(10, 4);
        feed(&mut t, "line1\r\nline2\r\nline3\r\n$ ");
        t.resize(10, 2);
        assert_eq!(t.text(), vec!["line3", "$"]);
        assert_eq!((t.cursor().col, t.cursor().row), (2, 1));
        assert_eq!(t.scrollback().len(), 2);
        assert_eq!(t.scrollback().get(0).unwrap().text().trim_end(), "line2");
    }

    #[test]
    fn resize_grow_rows_pulls_lines_back_from_history() {
        let mut t = term(10, 4);
        feed(&mut t, "line1\r\nline2\r\nline3\r\n$ ");
        t.resize(10, 2);
        t.resize(10, 4);
        assert_eq!(t.text(), vec!["line1", "line2", "line3", "$"]);
        assert_eq!((t.cursor().col, t.cursor().row), (2, 3));
        assert!(t.scrollback().is_empty());
    }

    #[test]
    fn resize_shrink_rows_on_alt_screen_never_touches_history() {
        let mut t = term(10, 3);
        feed(&mut t, "\x1b[?1049ha\r\nb\r\nc");
        t.resize(10, 1);
        assert!(t.scrollback().is_empty());
        assert_eq!(t.cursor().row, 0);
    }

    #[test]
    fn ris_emits_an_empty_title_event() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]0;T\x07");
        t.drain_events();
        feed(&mut t, "\x1bc");
        assert!(
            t.drain_events()
                .contains(&crate::outbox::TermEvent::Title(String::new()))
        );
    }
}
