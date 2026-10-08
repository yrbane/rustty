//! Positionnement du curseur : contrôles C0 (CR, BS, HT) puis séquences CSI.

use super::Term;

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
}
