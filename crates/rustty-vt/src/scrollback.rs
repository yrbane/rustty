//! Historique des lignes sorties par le haut de l'écran principal. Anneau
//! borné ; l'écran alternatif n'en a pas.

use std::collections::VecDeque;

use crate::cell::Cell;
use crate::line::Line;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scrollback {
    lines: VecDeque<Line>,
    max_lines: usize,
}

impl Scrollback {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            max_lines,
        }
    }

    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn push(&mut self, line: Line) {
        if self.max_lines == 0 {
            return;
        }
        if self.lines.len() == self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn extend(&mut self, lines: Vec<Line>) {
        for line in lines {
            self.push(line);
        }
    }

    /// `0` est la ligne la plus récente, c'est-à-dire celle juste au-dessus de l'écran.
    pub fn get(&self, idx_from_newest: usize) -> Option<&Line> {
        let len = self.lines.len();
        if idx_from_newest >= len {
            return None;
        }
        self.lines.get(len - 1 - idx_from_newest)
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Retire et rend la ligne la plus récente (celle juste au-dessus de l'écran).
    /// Vide l'historique et rend ses lignes, de la plus ancienne à la plus récente.
    pub fn drain_all(&mut self) -> Vec<Line> {
        self.lines.drain(..).collect()
    }

    pub fn pop_newest(&mut self) -> Option<Line> {
        self.lines.pop_back()
    }

    pub fn resize_lines(&mut self, cols: usize, template: Cell) {
        for line in &mut self.lines {
            line.resize(cols, template);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;

    fn line(c: char) -> Line {
        let mut l = Line::new(2);
        l.set(0, Cell::new(c, Default::default()));
        l
    }

    #[test]
    fn newest_line_is_index_zero() {
        let mut sb = Scrollback::new(10);
        sb.push(line('a'));
        sb.push(line('b'));
        assert_eq!(sb.len(), 2);
        assert_eq!(sb.get(0).unwrap().text(), "b ");
        assert_eq!(sb.get(1).unwrap().text(), "a ");
        assert!(sb.get(2).is_none());
    }

    #[test]
    fn oldest_lines_are_evicted_beyond_capacity() {
        let mut sb = Scrollback::new(2);
        sb.extend(vec![line('a'), line('b'), line('c')]);
        assert_eq!(sb.len(), 2);
        assert_eq!(sb.get(1).unwrap().text(), "b ");
    }

    #[test]
    fn zero_capacity_keeps_nothing() {
        let mut sb = Scrollback::new(0);
        sb.push(line('a'));
        assert!(sb.is_empty());
    }

    #[test]
    fn resize_lines_applies_to_history() {
        let mut sb = Scrollback::new(5);
        sb.push(line('a'));
        sb.resize_lines(4, Cell::default());
        assert_eq!(sb.get(0).unwrap().len(), 4);
    }

    #[test]
    fn clear_empties_history() {
        let mut sb = Scrollback::new(5);
        sb.push(line('a'));
        sb.clear();
        assert!(sb.is_empty());
    }

    #[test]
    fn pop_newest_returns_lines_from_the_bottom_of_history() {
        let mut sb = Scrollback::new(5);
        sb.extend(vec![line('a'), line('b')]);
        assert_eq!(sb.pop_newest().unwrap().text(), "b ");
        assert_eq!(sb.pop_newest().unwrap().text(), "a ");
        assert!(sb.pop_newest().is_none());
    }
}
