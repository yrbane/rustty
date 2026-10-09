//! Reflow : redécoupe les lignes à une nouvelle largeur en respectant les
//! retours à la ligne automatiques (lignes `wrapped`), comme le fait kitty
//! quand on redimensionne un panneau. Pur : des lignes en entrée, des lignes
//! en sortie, et la position du curseur reportée sur le même caractère.

use crate::cell::{Attrs, Cell};
use crate::line::Line;

/// Une cellule visible (jamais une seconde moitié de caractère large) et les
/// caractères de largeur nulle qui l'accompagnent.
struct Entry {
    cell: Cell,
    zerowidth: Option<String>,
}

impl Entry {
    fn width(&self) -> usize {
        if self.cell.is_wide() { 2 } else { 1 }
    }

    fn is_blank(&self) -> bool {
        self.cell == Cell::default() && self.zerowidth.is_none()
    }
}

/// Une ligne logique : ce que l'application a écrit entre deux retours à la
/// ligne explicites, et la position du curseur en entrées, s'il y est.
#[derive(Default)]
struct Logical {
    entries: Vec<Entry>,
    cursor: Option<usize>,
}

/// Redécoupe `lines` (de la plus ancienne à la plus récente) à `cols`
/// colonnes. `cursor` est (ligne, colonne) dans `lines` ; le résultat donne
/// sa position dans les nouvelles lignes, la colonne pouvant valoir `cols`
/// quand le curseur suit le dernier caractère d'une ligne pleine.
pub fn reflow(
    lines: Vec<Line>,
    cursor: Option<(usize, usize)>,
    cols: usize,
) -> (Vec<Line>, Option<(usize, usize)>) {
    let cols = cols.max(1);
    let mut out = Vec::with_capacity(lines.len());
    let mut new_cursor = None;
    let mut current: Option<Logical> = None;
    for (row, mut line) in lines.into_iter().enumerate() {
        let on_cursor = cursor.is_some_and(|(r, _)| r == row);
        // Chemin rapide : une ligne isolée qui tient déjà est seulement mise à
        // la largeur (le cas de presque tout l'historique).
        if current.is_none() && !line.wrapped && !on_cursor && used_width(&line) <= cols {
            line.resize(cols, Cell::default());
            line.wrapped = false;
            out.push(line);
            continue;
        }
        let logical = current.get_or_insert_with(Logical::default);
        append(logical, &line, row, cursor);
        if !line.wrapped
            && let Some(done) = current.take()
        {
            layout(trimmed(done), cols, &mut out, &mut new_cursor);
        }
    }
    if let Some(done) = current {
        layout(trimmed(done), cols, &mut out, &mut new_cursor);
    }
    (out, new_cursor)
}

/// Colonnes occupées : jusqu'à la dernière cellule qui n'est pas un blanc par défaut.
fn used_width(line: &Line) -> usize {
    line.cells()
        .iter()
        .rposition(|c| *c != Cell::default())
        .map_or(0, |i| i + 1)
}

fn append(logical: &mut Logical, line: &Line, row: usize, cursor: Option<(usize, usize)>) {
    for (col, cell) in line.cells().iter().enumerate() {
        if cursor == Some((row, col)) {
            // Sur une seconde moitié, le curseur désigne le caractère large.
            let back = usize::from(cell.is_wide_continuation());
            logical.cursor = Some(logical.entries.len().saturating_sub(back));
        }
        if !cell.is_wide_continuation() {
            logical.entries.push(Entry {
                cell: *cell,
                zerowidth: line.zerowidth(col).map(str::to_owned),
            });
        }
    }
    if let Some((r, c)) = cursor
        && r == row
        && c >= line.len()
    {
        logical.cursor = Some(logical.entries.len() + (c - line.len()));
    }
}

/// Retire les blancs de fin, sauf ceux qui précèdent le curseur.
fn trimmed(mut logical: Logical) -> Logical {
    let keep = logical.cursor.unwrap_or(0);
    while logical.entries.len() > keep && logical.entries.last().is_some_and(Entry::is_blank) {
        logical.entries.pop();
    }
    logical
}

fn layout(logical: Logical, cols: usize, out: &mut Vec<Line>, cursor: &mut Option<(usize, usize)>) {
    let mut line = Line::new(cols);
    let mut col = 0;
    let count = logical.entries.len();
    for (k, entry) in logical.entries.into_iter().enumerate() {
        // Un caractère large ne tient pas dans une colonne : il y est réduit.
        let width = entry.width().min(cols);
        if col + width > cols {
            line.wrapped = true;
            out.push(std::mem::replace(&mut line, Line::new(cols)));
            col = 0;
        }
        if logical.cursor == Some(k) {
            *cursor = Some((out.len(), col));
        }
        let mut cell = entry.cell;
        if width == 1 {
            cell.style.attrs.remove(Attrs::WIDE);
        }
        line.set(col, cell);
        for ch in entry.zerowidth.iter().flat_map(|s| s.chars()) {
            line.push_zerowidth(col, ch);
        }
        if width == 2 {
            let mut half = Cell::new(' ', cell.style);
            half.style.attrs.remove(Attrs::WIDE);
            half.style.attrs.insert(Attrs::WIDE_CONTINUATION);
            line.set(col + 1, half);
        }
        col += width;
    }
    if logical.cursor.is_some_and(|c| c >= count) {
        *cursor = Some((out.len(), col));
    }
    out.push(line);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une ligne de `cols` colonnes contenant `text` (ASCII).
    fn line(text: &str, cols: usize, wrapped: bool) -> Line {
        let mut l = Line::new(cols);
        for (i, c) in text.chars().enumerate() {
            l.set(i, Cell::new(c, Default::default()));
        }
        l.wrapped = wrapped;
        l
    }

    fn texts(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|l| l.text().trim_end().to_string())
            .collect()
    }

    fn wraps(lines: &[Line]) -> Vec<bool> {
        lines.iter().map(|l| l.wrapped).collect()
    }

    #[test]
    fn a_long_line_rewraps_narrower() {
        let (out, _) = reflow(vec![line("abcdefgh", 8, false)], None, 3);
        assert_eq!(texts(&out), ["abc", "def", "gh"]);
        assert_eq!(wraps(&out), [true, true, false]);
        assert!(out.iter().all(|l| l.len() == 3));
    }

    #[test]
    fn wrapped_lines_join_when_wider() {
        let input = vec![
            line("abc", 3, true),
            line("def", 3, true),
            line("g", 3, false),
        ];
        let (out, _) = reflow(input, None, 10);
        assert_eq!(texts(&out), ["abcdefg"]);
        assert_eq!(wraps(&out), [false]);
    }

    #[test]
    fn hard_line_breaks_are_kept() {
        let input = vec![
            line("ab", 4, false),
            line("", 4, false),
            line("cd", 4, false),
        ];
        let (out, _) = reflow(input, None, 10);
        assert_eq!(texts(&out), ["ab", "", "cd"]);
    }

    #[test]
    fn trailing_blanks_are_dropped() {
        let (out, _) = reflow(vec![line("ab", 10, false)], None, 1);
        assert_eq!(
            texts(&out),
            ["a", "b"],
            "les 8 blancs de fin ne deviennent pas 8 lignes"
        );
    }

    #[test]
    fn wide_char_never_splits() {
        let mut l = Line::new(4);
        l.set(0, Cell::new('a', Default::default()));
        let mut wide = Cell::new('漢', Default::default());
        wide.style.attrs |= Attrs::WIDE;
        let mut cont = Cell::new(' ', Default::default());
        cont.style.attrs |= Attrs::WIDE_CONTINUATION;
        l.set(1, wide);
        l.set(2, cont);
        l.set(3, Cell::new('b', Default::default()));
        let (out, _) = reflow(vec![l], None, 2);
        assert_eq!(
            texts(&out),
            ["a", "漢", "b"],
            "le caractère large passe entier à la ligne suivante"
        );
        assert!(out[1].get(0).is_wide() && out[1].get(1).is_wide_continuation());
        assert!(out[0].wrapped && out[1].wrapped && !out[2].wrapped);
    }

    #[test]
    fn width_one_keeps_every_char() {
        let (out, _) = reflow(vec![line("xyz", 3, false)], None, 1);
        assert_eq!(texts(&out), ["x", "y", "z"]);
    }

    #[test]
    fn cursor_follows_its_character() {
        let input = vec![line("abcdef", 6, true), line("gh", 6, false)];
        let (out, cursor) = reflow(input.clone(), Some((1, 1)), 4);
        assert_eq!(texts(&out), ["abcd", "efgh"]);
        assert_eq!(cursor, Some((1, 3)), "sur le h");
        let (_, after_end) = reflow(input, Some((1, 2)), 4);
        assert_eq!(
            after_end,
            Some((1, 4)),
            "après le dernier caractère, colonne = largeur"
        );
    }

    #[test]
    fn blanks_before_the_cursor_are_kept() {
        let (out, cursor) = reflow(vec![line("$", 10, false)], Some((0, 2)), 5);
        assert_eq!(cursor, Some((0, 2)), "l'espace après l'invite reste");
        assert_eq!(texts(&out), ["$"]);
    }

    #[test]
    fn empty_input_is_empty() {
        let (out, cursor) = reflow(Vec::new(), None, 5);
        assert!(out.is_empty());
        assert_eq!(cursor, None);
    }
}

#[cfg(test)]
mod perf {
    use super::*;

    /// Un historique de 10 000 lignes courtes, redécoupé à chaque colonne
    /// franchie pendant un glisser : doit rester bien sous la frame.
    #[test]
    fn reflowing_ten_thousand_short_lines_is_fast() {
        let lines: Vec<Line> = (0..10_000)
            .map(|i| {
                let mut l = Line::new(120);
                for (c, ch) in format!("ligne {i} avec un peu de texte")
                    .chars()
                    .enumerate()
                {
                    l.set(c, Cell::new(ch, Default::default()));
                }
                l
            })
            .collect();
        let start = std::time::Instant::now();
        let (out, _) = reflow(lines, None, 119);
        let elapsed = start.elapsed();
        assert_eq!(out.len(), 10_000);
        assert_eq!(
            out[9_999].text().trim_end(),
            "ligne 9999 avec un peu de texte"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "reflow de 10 000 lignes : {elapsed:?}"
        );
    }
}
