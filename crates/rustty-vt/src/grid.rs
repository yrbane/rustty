//! Grille d'écran : `rows` lignes de `cols` cellules. Ne connaît pas le
//! curseur ni le scrollback ; le `Term` orchestre.

use crate::cell::Cell;
use crate::line::Line;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    cols: usize,
    rows: usize,
    lines: Vec<Line>,
}

impl Grid {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            lines: (0..rows).map(|_| Line::new(cols)).collect(),
        }
    }

    /// Une grille faite de `lines`, mises à la largeur et complétées ou
    /// tronquées à `rows` lignes.
    pub fn from_lines(cols: usize, rows: usize, mut lines: Vec<Line>) -> Self {
        for line in &mut lines {
            line.resize(cols, Cell::default());
        }
        lines.resize_with(rows, || Line::new(cols));
        Self { cols, rows, lines }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub fn line(&self, row: usize) -> &Line {
        &self.lines[row]
    }

    pub fn line_mut(&mut self, row: usize) -> &mut Line {
        &mut self.lines[row]
    }

    pub fn cell(&self, col: usize, row: usize) -> &Cell {
        self.lines[row].get(col)
    }

    pub fn set_cell(&mut self, col: usize, row: usize, cell: Cell) {
        self.lines[row].set(col, cell);
    }

    /// Fait monter `top..=bottom` de `n` lignes. Retourne les lignes sorties
    /// par le haut, de la plus ancienne à la plus récente.
    pub fn scroll_up(&mut self, top: usize, bottom: usize, n: usize, template: Cell) -> Vec<Line> {
        let (top, bottom) = self.clamp_region(top, bottom);
        let height = bottom - top + 1;
        let n = n.min(height);
        if n == 0 {
            return Vec::new();
        }
        let evicted: Vec<Line> = self.lines[top..top + n].to_vec();
        self.lines[top..=bottom].rotate_left(n);
        for line in &mut self.lines[bottom + 1 - n..=bottom] {
            line.reset(template);
        }
        evicted
    }

    /// Fait descendre `top..=bottom` de `n` lignes, remplit le haut avec `template`.
    pub fn scroll_down(&mut self, top: usize, bottom: usize, n: usize, template: Cell) {
        let (top, bottom) = self.clamp_region(top, bottom);
        let height = bottom - top + 1;
        let n = n.min(height);
        if n == 0 {
            return;
        }
        self.lines[top..=bottom].rotate_right(n);
        for line in &mut self.lines[top..top + n] {
            line.reset(template);
        }
    }

    pub fn clear(&mut self, template: Cell) {
        for line in &mut self.lines {
            line.reset(template);
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize, template: Cell) {
        for line in &mut self.lines {
            line.resize(cols, template);
        }
        self.lines
            .resize_with(rows, || Line::filled(cols, template));
        self.cols = cols;
        self.rows = rows;
    }

    /// Texte de chaque ligne, blancs de fin supprimés. Pour les tests.
    pub fn text(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|l| l.text().trim_end().to_string())
            .collect()
    }

    fn clamp_region(&self, top: usize, bottom: usize) -> (usize, usize) {
        let bottom = bottom.min(self.rows.saturating_sub(1));
        (top.min(bottom), bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_with_letters(rows: usize) -> Grid {
        let mut g = Grid::new(3, rows);
        for r in 0..rows {
            let c = char::from(b'a' + u8::try_from(r).unwrap());
            g.set_cell(0, r, Cell::new(c, Default::default()));
        }
        g
    }

    #[test]
    fn new_grid_has_requested_size_and_is_blank() {
        let g = Grid::new(4, 2);
        assert_eq!((g.cols(), g.rows()), (4, 2));
        assert_eq!(g.text(), vec!["", ""]);
    }

    #[test]
    fn scroll_up_whole_screen_returns_evicted_lines() {
        let mut g = grid_with_letters(4);
        let evicted = g.scroll_up(0, 3, 2, Cell::default());
        assert_eq!(
            evicted.iter().map(Line::text).collect::<Vec<_>>(),
            vec!["a  ", "b  "]
        );
        assert_eq!(g.text(), vec!["c", "d", "", ""]);
    }

    #[test]
    fn scroll_up_inside_region_leaves_outside_rows_alone() {
        let mut g = grid_with_letters(5);
        let evicted = g.scroll_up(1, 3, 1, Cell::default());
        assert_eq!(evicted.len(), 1);
        assert_eq!(g.text(), vec!["a", "c", "d", "", "e"]);
    }

    #[test]
    fn scroll_down_inside_region() {
        let mut g = grid_with_letters(5);
        g.scroll_down(1, 3, 1, Cell::default());
        assert_eq!(g.text(), vec!["a", "", "b", "c", "e"]);
    }

    #[test]
    fn scrolling_more_than_region_height_just_clears_it() {
        let mut g = grid_with_letters(3);
        let evicted = g.scroll_up(0, 2, 10, Cell::default());
        assert_eq!(evicted.len(), 3);
        assert_eq!(g.text(), vec!["", "", ""]);
    }

    #[test]
    fn resize_pads_and_truncates_lines_and_rows() {
        let mut g = grid_with_letters(2);
        g.resize(5, 3, Cell::default());
        assert_eq!((g.cols(), g.rows()), (5, 3));
        assert_eq!(g.text(), vec!["a", "b", ""]);
        g.resize(1, 1, Cell::default());
        assert_eq!(g.text(), vec!["a"]);
    }

    #[test]
    fn clear_resets_all_lines() {
        let mut g = grid_with_letters(2);
        g.clear(Cell::default());
        assert_eq!(g, Grid::new(3, 2));
    }
}
