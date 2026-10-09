//! Une ligne de la grille : des cellules de largeur fixe plus une table annexe
//! pour les caractères combinants, qui n'occupent aucune cellule.

use std::collections::BTreeMap;
use std::ops::Range;

use crate::cell::Cell;
use crate::graphics::ImageStrip;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    cells: Vec<Cell>,
    /// Caractères à largeur nulle attachés à une colonne, dans l'ordre d'arrivée.
    zerowidth: BTreeMap<usize, String>,
    /// Vrai si la ligne a débordé sur la suivante (saut de ligne implicite).
    pub wrapped: bool,
    /// Tranches d'images affichées sur cette ligne (partagées, donc peu coûteuses à cloner).
    pub(crate) images: Vec<ImageStrip>,
}

impl Line {
    pub fn new(cols: usize) -> Self {
        Self::filled(cols, Cell::default())
    }

    pub fn filled(cols: usize, template: Cell) -> Self {
        Self {
            cells: vec![template; cols],
            zerowidth: BTreeMap::new(),
            wrapped: false,
            images: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn get(&self, col: usize) -> &Cell {
        &self.cells[col]
    }

    pub fn get_mut(&mut self, col: usize) -> &mut Cell {
        self.zerowidth.remove(&col);
        &mut self.cells[col]
    }

    pub fn set(&mut self, col: usize, cell: Cell) {
        self.zerowidth.remove(&col);
        self.cells[col] = cell;
    }

    pub fn push_zerowidth(&mut self, col: usize, ch: char) {
        self.zerowidth.entry(col).or_default().push(ch);
    }

    pub fn zerowidth(&self, col: usize) -> Option<&str> {
        self.zerowidth.get(&col).map(String::as_str)
    }

    pub fn reset(&mut self, template: Cell) {
        self.cells.fill(template);
        self.zerowidth.clear();
        self.images.clear();
        self.wrapped = false;
    }

    pub fn erase_range(&mut self, range: Range<usize>, template: Cell) {
        let range = range.start.min(self.len())..range.end.min(self.len());
        self.zerowidth.retain(|col, _| !range.contains(col));
        self.drop_strips_in(range.clone());
        self.cells[range].fill(template);
    }

    /// Insère `n` cellules `template` en `col`, décale le reste à droite et
    /// tronque ce qui dépasse.
    pub fn insert_blank(&mut self, col: usize, n: usize, template: Cell) {
        let cols = self.len();
        if col >= cols {
            return;
        }
        let n = n.min(cols - col);
        self.drop_strips_in(col..cols);
        self.cells[col..].rotate_right(n);
        self.cells[col..col + n].fill(template);
        let moved: Vec<(usize, String)> = self
            .zerowidth
            .range(col..)
            .filter(|(c, _)| **c + n < cols)
            .map(|(c, s)| (c + n, s.clone()))
            .collect();
        self.zerowidth.retain(|c, _| *c < col);
        self.zerowidth.extend(moved);
    }

    /// Supprime `n` cellules en `col`, décale le reste à gauche et complète
    /// avec `template`.
    pub fn delete(&mut self, col: usize, n: usize, template: Cell) {
        let cols = self.len();
        if col >= cols {
            return;
        }
        let n = n.min(cols - col);
        self.drop_strips_in(col..cols);
        self.cells[col..].rotate_left(n);
        self.cells[cols - n..].fill(template);
        let moved: Vec<(usize, String)> = self
            .zerowidth
            .range(col + n..)
            .map(|(c, s)| (c - n, s.clone()))
            .collect();
        self.zerowidth.retain(|c, _| *c < col);
        self.zerowidth.extend(moved);
    }

    pub fn resize(&mut self, cols: usize, template: Cell) {
        self.cells.resize(cols, template);
        self.zerowidth.retain(|c, _| *c < cols);
        self.drop_strips_from(cols);
    }

    /// Texte de la ligne, combinants inclus, continuations de caractères larges exclues.
    pub fn text(&self) -> String {
        let mut out = String::with_capacity(self.len());
        for (col, cell) in self.cells.iter().enumerate() {
            if cell.is_wide_continuation() {
                continue;
            }
            out.push(cell.c);
            if let Some(zw) = self.zerowidth.get(&col) {
                out.push_str(zw);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{Attrs, Style};

    fn styled(c: char) -> Cell {
        Cell::new(
            c,
            Style {
                attrs: Attrs::BOLD,
                ..Style::default()
            },
        )
    }

    #[test]
    fn new_line_is_blank() {
        let l = Line::new(4);
        assert_eq!(l.len(), 4);
        assert_eq!(l.text(), "    ");
        assert!(!l.wrapped);
    }

    #[test]
    fn set_and_get_cells() {
        let mut l = Line::new(3);
        l.set(1, styled('a'));
        assert_eq!(l.get(1).c, 'a');
        assert_eq!(l.text(), " a ");
    }

    #[test]
    fn zerowidth_chars_attach_to_a_column_and_follow_text() {
        let mut l = Line::new(3);
        l.set(0, styled('e'));
        l.push_zerowidth(0, '\u{301}');
        assert_eq!(l.zerowidth(0), Some("\u{301}"));
        assert_eq!(l.text(), "e\u{301}  ");
        l.set(0, styled('x'));
        assert_eq!(
            l.zerowidth(0),
            None,
            "écraser la cellule efface ses combinants"
        );
    }

    #[test]
    fn text_skips_wide_continuations() {
        let mut l = Line::new(3);
        l.set(
            0,
            Cell::new(
                '漢',
                Style {
                    attrs: Attrs::WIDE,
                    ..Style::default()
                },
            ),
        );
        l.set(
            1,
            Cell::new(
                ' ',
                Style {
                    attrs: Attrs::WIDE_CONTINUATION,
                    ..Style::default()
                },
            ),
        );
        assert_eq!(l.text(), "漢 ");
    }

    #[test]
    fn insert_blank_shifts_right_and_truncates() {
        let mut l = Line::new(4);
        for (i, c) in "abcd".chars().enumerate() {
            l.set(i, styled(c));
        }
        l.push_zerowidth(3, '\u{301}');
        l.insert_blank(1, 2, Cell::default());
        assert_eq!(l.text(), "a  b");
        assert_eq!(
            l.zerowidth(3),
            None,
            "le d et son combinant sont sortis de la ligne"
        );
    }

    #[test]
    fn delete_shifts_left_and_pads_with_template() {
        let mut l = Line::new(4);
        for (i, c) in "abcd".chars().enumerate() {
            l.set(i, styled(c));
        }
        l.push_zerowidth(2, '\u{301}');
        l.delete(1, 2, Cell::default());
        assert_eq!(l.text(), "ad  ");
        assert_eq!(l.zerowidth(2), None);
    }

    #[test]
    fn erase_range_uses_template_and_clears_zerowidth() {
        let mut l = Line::new(4);
        l.set(2, styled('c'));
        l.push_zerowidth(2, '\u{301}');
        let tpl = Cell::erased(Style {
            bg: crate::Color::Indexed(4),
            ..Style::default()
        });
        l.erase_range(1..3, tpl);
        assert_eq!(l.get(2).style.bg, crate::Color::Indexed(4));
        assert_eq!(l.zerowidth(2), None);
        assert_eq!(l.text(), "    ");
    }

    #[test]
    fn resize_truncates_or_pads() {
        let mut l = Line::new(3);
        l.set(2, styled('c'));
        l.push_zerowidth(2, '\u{301}');
        l.resize(2, Cell::default());
        assert_eq!(l.len(), 2);
        assert_eq!(l.zerowidth(2), None);
        l.resize(5, Cell::default());
        assert_eq!(l.text(), "     ");
    }

    #[test]
    fn reset_clears_everything() {
        let mut l = Line::new(2);
        l.set(0, styled('a'));
        l.push_zerowidth(0, '\u{301}');
        l.wrapped = true;
        l.reset(Cell::default());
        assert_eq!(l, Line::new(2));
    }
}
