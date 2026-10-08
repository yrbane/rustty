//! Ce que l'interface voit : instantané de la zone affichée et défilement de
//! l'affichage dans l'historique.

use super::Term;
use crate::cursor::Cursor;
use crate::line::Line;
use crate::snapshot::Snapshot;

impl Term {
    pub fn display_offset(&self) -> usize {
        self.display_offset
    }

    /// Décale l'affichage vers l'historique (`delta > 0`) ou vers l'écran vivant.
    pub fn scroll_display(&mut self, delta: isize) {
        let max = self.scrollback.len();
        let current = isize::try_from(self.display_offset).unwrap_or(isize::MAX);
        let wanted = current.saturating_add(delta).max(0);
        self.display_offset = usize::try_from(wanted).unwrap_or(0).min(max);
    }

    pub fn scroll_display_to_bottom(&mut self) {
        self.display_offset = 0;
    }

    pub fn snapshot(&self) -> Snapshot {
        let rows = self.rows();
        let offset = if self.modes.alt_screen {
            0
        } else {
            self.display_offset.min(self.scrollback.len())
        };
        let grid = self.active_grid();
        let lines = (0..rows)
            .map(|r| {
                if r < offset {
                    // Ligne d'historique : offset-1 est la plus récente, affichée en haut.
                    self.scrollback
                        .get(offset - 1 - r)
                        .cloned()
                        .unwrap_or_else(|| Line::new(grid.cols()))
                } else {
                    grid.line(r - offset).clone()
                }
            })
            .collect();
        let cursor = if self.modes.cursor_visible && self.cursor.row + offset < rows {
            Some(Cursor {
                row: self.cursor.row + offset,
                ..self.cursor
            })
        } else {
            None
        };
        Snapshot {
            cols: grid.cols(),
            rows,
            lines,
            cursor,
            cursor_shape: self.cursor_shape,
            display_offset: offset,
            scrollback_len: self.scrollback.len(),
            title: self.title.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::term::Term;
    use crate::term::test_support::{feed, term};

    fn visible(t: &Term) -> Vec<String> {
        t.snapshot()
            .lines
            .iter()
            .map(|l| l.text().trim_end().to_string())
            .collect()
    }

    #[test]
    fn snapshot_shows_live_screen_by_default() {
        let mut t = term(3, 2);
        feed(&mut t, "a\r\nb");
        let s = t.snapshot();
        assert_eq!(visible(&t), vec!["a", "b"]);
        assert_eq!(s.cursor.map(|c| (c.col, c.row)), Some((1, 1)));
        assert_eq!((s.display_offset, s.scrollback_len), (0, 0));
    }

    #[test]
    fn scroll_display_reveals_history_and_shifts_or_hides_cursor() {
        let mut t = term(3, 2);
        feed(&mut t, "a\r\nb\r\nc\r\nd\x1b[H");
        assert_eq!(t.scrollback().len(), 2);
        t.scroll_display(1);
        assert_eq!(visible(&t), vec!["b", "c"]);
        assert_eq!(
            t.snapshot().cursor.map(|c| c.row),
            Some(1),
            "le curseur, en ligne 0 du vivant, descend d'une ligne à l'écran"
        );
        t.scroll_display(5);
        assert_eq!(t.display_offset(), 2, "borné à l'historique");
        assert_eq!(visible(&t), vec!["a", "b"]);
        assert!(t.snapshot().cursor.is_none(), "curseur hors viewport");
        t.scroll_display(-1);
        assert_eq!(t.display_offset(), 1);
        t.scroll_display_to_bottom();
        assert_eq!(t.display_offset(), 0);
    }

    #[test]
    fn hidden_cursor_is_absent_from_snapshot() {
        let mut t = term(3, 1);
        feed(&mut t, "\x1b[?25l");
        assert!(t.snapshot().cursor.is_none());
    }

    #[test]
    fn alt_screen_ignores_display_offset() {
        let mut t = term(3, 1);
        feed(&mut t, "a\r\nb");
        t.scroll_display(1);
        feed(&mut t, "\x1b[?1049h\x1b[Hz");
        assert_eq!(visible(&t), vec!["z"]);
    }

    #[test]
    fn resize_and_reset_return_display_to_bottom() {
        let mut t = term(3, 1);
        feed(&mut t, "a\r\nb");
        t.scroll_display(1);
        t.resize(4, 1);
        assert_eq!(t.display_offset(), 0);
        feed(&mut t, "c\r\nd");
        t.scroll_display(1);
        feed(&mut t, "\x1bc");
        assert_eq!(t.display_offset(), 0);
    }
}
