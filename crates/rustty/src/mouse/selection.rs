//! Sélection à la souris en coordonnées de cellules et extraction du texte
//! correspondant, pures : le presse-papiers et le rendu les consomment.

use rustty_vt::Snapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellPos {
    pub col: usize,
    pub row: usize,
}

impl PartialOrd for CellPos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CellPos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.row, self.col).cmp(&(other.row, other.col))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    anchor: CellPos,
    head: CellPos,
}

impl Selection {
    pub fn start(pos: CellPos) -> Self {
        Self {
            anchor: pos,
            head: pos,
        }
    }

    pub fn extend(&mut self, pos: CellPos) {
        self.head = pos;
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// (début, fin) en ordre de lecture, bornes incluses.
    pub fn bounds(&self) -> (CellPos, CellPos) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    pub fn contains(&self, pos: CellPos) -> bool {
        let (start, end) = self.bounds();
        start <= pos && pos <= end
    }

    /// Le texte sélectionné, lignes de fin nettoyées, retours à la ligne
    /// seulement entre deux lignes logiques distinctes.
    pub fn text(&self, snapshot: &Snapshot) -> String {
        let (start, end) = self.bounds();
        let last_row = snapshot.rows.saturating_sub(1);
        let (start_row, end_row) = (start.row.min(last_row), end.row.min(last_row));
        let mut out = String::new();
        for row in start_row..=end_row {
            let Some(line) = snapshot.lines.get(row) else {
                break;
            };
            let cells = line.cells();
            let first = if row == start_row {
                start.col.min(cells.len())
            } else {
                0
            };
            let last = if row == end_row {
                end.col.min(cells.len().saturating_sub(1))
            } else {
                cells.len().saturating_sub(1)
            };
            let mut segment = String::new();
            for cell in cells.iter().take(last + 1).skip(first) {
                if !cell.is_wide_continuation() {
                    segment.push(cell.c);
                }
            }
            if !line.wrapped || row == end_row {
                out.push_str(segment.trim_end());
            } else {
                out.push_str(&segment);
            }
            if row != end_row && !line.wrapped {
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustty_vt::Term;

    fn snap(cols: usize, rows: usize, input: &str) -> Snapshot {
        let mut t = Term::new(cols, rows, 0);
        t.input(input.as_bytes());
        t.snapshot()
    }

    fn pos(col: usize, row: usize) -> CellPos {
        CellPos { col, row }
    }

    #[test]
    fn bounds_are_ordered_whatever_the_drag_direction() {
        let mut s = Selection::start(pos(5, 2));
        s.extend(pos(1, 0));
        assert_eq!(s.bounds(), (pos(1, 0), pos(5, 2)));
        assert!(!s.is_empty());
        assert!(Selection::start(pos(3, 3)).is_empty());
    }

    #[test]
    fn contains_follows_reading_order() {
        let mut s = Selection::start(pos(4, 0));
        s.extend(pos(2, 2));
        assert!(
            s.contains(pos(4, 0)) && s.contains(pos(9, 0)),
            "fin de la première ligne"
        );
        assert!(
            s.contains(pos(0, 1)) && s.contains(pos(9, 1)),
            "ligne entière au milieu"
        );
        assert!(s.contains(pos(0, 2)) && s.contains(pos(2, 2)));
        assert!(!s.contains(pos(3, 2)) && !s.contains(pos(3, 0)));
    }

    #[test]
    fn text_trims_line_ends_and_joins_with_newlines() {
        let snapshot = snap(10, 3, "hello\r\nworld  \r\n  end");
        let mut s = Selection::start(pos(0, 0));
        s.extend(pos(9, 2));
        assert_eq!(s.text(&snapshot), "hello\nworld\n  end");
        let mut partial = Selection::start(pos(1, 0));
        partial.extend(pos(2, 1));
        assert_eq!(partial.text(&snapshot), "ello\nwor");
    }

    #[test]
    fn wrapped_lines_are_joined_without_newline() {
        let snapshot = snap(5, 3, "abcdefgh");
        assert!(snapshot.lines[0].wrapped);
        let mut s = Selection::start(pos(0, 0));
        s.extend(pos(4, 1));
        assert_eq!(s.text(&snapshot), "abcdefgh");
    }

    #[test]
    fn wide_characters_are_copied_once() {
        let snapshot = snap(10, 1, "a漢b");
        let mut s = Selection::start(pos(0, 0));
        s.extend(pos(3, 0));
        assert_eq!(s.text(&snapshot), "a漢b");
    }

    #[test]
    fn positions_outside_the_snapshot_are_clamped() {
        let snapshot = snap(4, 2, "ab\r\ncd");
        let mut s = Selection::start(pos(0, 0));
        s.extend(pos(40, 40));
        assert_eq!(s.text(&snapshot), "ab\ncd");
    }
}
