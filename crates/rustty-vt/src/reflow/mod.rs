//! Reflow : redécoupe les lignes à une nouvelle largeur en respectant les
//! retours à la ligne automatiques (lignes `wrapped`), comme le fait kitty
//! quand on redimensionne un panneau. Pur : des lignes en entrée, des lignes
//! en sortie, et la position des curseurs reportée sur le même caractère.

use crate::cell::{Attrs, Cell};
use crate::graphics::ImageStrip;
use crate::line::Line;

/// Largeur en colonnes d'une entrée (jamais une seconde moitié de large).
fn width_of(cell: &Cell) -> usize {
    if cell.is_wide() { 2 } else { 1 }
}

/// Une espace sans attribut est un blanc, quelles que soient ses couleurs : le
/// fond d'une queue colorée est perdu au reflow plutôt que d'y fabriquer des
/// lignes vides. Une seconde moitié de caractère large n'en est jamais un.
pub(crate) fn is_blank(cell: &Cell) -> bool {
    cell.c == ' ' && cell.style.attrs.is_empty()
}

/// Une ligne logique : ce que l'application a écrit entre deux retours à la
/// ligne explicites, la position de chaque curseur en entrées, s'il y est, et
/// les bandes d'image de toutes ses lignes source. Les combinants sont à part,
/// indexés par entrée et croissants : presque toujours vides, ils laissent
/// les entrées à 16 octets.
struct Logical {
    entries: Vec<Cell>,
    zerowidth: Vec<(usize, String)>,
    cursors: Vec<Option<usize>>,
    images: Vec<ImageStrip>,
}

impl Logical {
    fn new(cursors: usize) -> Self {
        Self {
            entries: Vec::new(),
            zerowidth: Vec::new(),
            cursors: vec![None; cursors],
            images: Vec::new(),
        }
    }

    /// Une entrée est un blanc si sa cellule l'est et qu'aucun combinant n'y tient.
    fn is_blank_at(&self, k: usize) -> bool {
        is_blank(&self.entries[k]) && self.zerowidth.last().is_none_or(|&(i, _)| i != k)
    }
}

/// Redécoupe `lines` (de la plus ancienne à la plus récente) à `cols`
/// colonnes. Seul le premier curseur retient les blancs de fin. Chaque curseur est (ligne, colonne) dans `lines` ; le résultat
/// donne, dans le même ordre, sa position dans les nouvelles lignes, la
/// colonne pouvant valoir `cols` quand il suit le dernier caractère d'une
/// ligne pleine, ou `None` s'il ne désignait aucune ligne.
pub fn reflow(
    lines: Vec<Line>,
    cursors: &[(usize, usize)],
    cols: usize,
) -> (Vec<Line>, Vec<Option<(usize, usize)>>) {
    let cols = cols.max(1);
    let mut out = Vec::with_capacity(lines.len());
    let mut moved = vec![None; cursors.len()];
    let mut current: Option<Logical> = None;
    for (row, mut line) in lines.into_iter().enumerate() {
        let on_cursor = cursors.iter().any(|&(r, _)| r == row);
        // Chemin rapide : une ligne isolée qui tient déjà est seulement mise à
        // la largeur (le cas de presque tout l'historique). Ses bandes d'image
        // restent toutes, même au-delà de la largeur : le rendu les rogne.
        if current.is_none() && !line.wrapped && !on_cursor && used_width(&line) <= cols {
            let images = std::mem::take(&mut line.images);
            line.resize(cols, Cell::default());
            line.images = images;
            line.wrapped = false;
            out.push(line);
            continue;
        }
        let logical = current.get_or_insert_with(|| Logical::new(cursors.len()));
        append(logical, &mut line, row, cursors);
        if !line.wrapped
            && let Some(done) = current.take()
        {
            layout(trimmed(done), cols, &mut out, &mut moved);
        }
    }
    if let Some(done) = current {
        layout(trimmed(done), cols, &mut out, &mut moved);
    }
    (out, moved)
}

/// Colonnes occupées : jusqu'à la dernière cellule qui n'est pas un blanc.
fn used_width(line: &Line) -> usize {
    line.cells()
        .iter()
        .rposition(|c| !is_blank(c))
        .map_or(0, |i| i + 1)
}

fn append(logical: &mut Logical, line: &mut Line, row: usize, cursors: &[(usize, usize)]) {
    logical.images.append(&mut line.images);
    if logical.entries.is_empty() {
        logical.entries.reserve(line.len());
    }
    let has_marks = line.has_zerowidth();
    let on_row = cursors.iter().any(|&(r, _)| r == row);
    if !on_row && !has_marks && !has_wide(line.cells()) {
        logical.entries.extend_from_slice(line.cells());
        return;
    }
    for (col, cell) in line.cells().iter().enumerate() {
        for (slot, &cursor) in logical.cursors.iter_mut().zip(cursors) {
            if cursor == (row, col) {
                // Sur une seconde moitié, le curseur désigne le caractère large.
                let back = usize::from(cell.is_wide_continuation());
                *slot = Some(logical.entries.len().saturating_sub(back));
            }
        }
        if !cell.is_wide_continuation() {
            if has_marks && let Some(marks) = line.zerowidth(col) {
                logical
                    .zerowidth
                    .push((logical.entries.len(), marks.to_owned()));
            }
            logical.entries.push(*cell);
        }
    }
    for (slot, &(r, c)) in logical.cursors.iter_mut().zip(cursors) {
        if r == row && c >= line.len() {
            *slot = Some(logical.entries.len() + (c - line.len()));
        }
    }
}

/// Une cellule large (ou sa moitié droite) dans la tranche.
fn has_wide(cells: &[Cell]) -> bool {
    let halves = Attrs::WIDE | Attrs::WIDE_CONTINUATION;
    cells.iter().any(|c| c.style.attrs.intersects(halves))
}

/// Retire les blancs de fin, sauf ceux qui précèdent le premier curseur (le
/// curseur vivant) ; les suivants, au-delà du texte, s'y posent en fin.
fn trimmed(mut logical: Logical) -> Logical {
    let keep = logical.cursors.first().copied().flatten().unwrap_or(0);
    while logical.entries.len() > keep && logical.is_blank_at(logical.entries.len() - 1) {
        logical.entries.pop();
    }
    logical
}

fn layout(
    logical: Logical,
    cols: usize,
    out: &mut Vec<Line>,
    moved: &mut [Option<(usize, usize)>],
) {
    if logical.zerowidth.is_empty() && !has_wide(&logical.entries) {
        return layout_narrow(logical, cols, out, moved);
    }
    let mut line = Line::new(cols);
    // Les bandes vont à la première ligne produite, colonne inchangée.
    line.images = logical.images;
    let mut col = 0;
    let count = logical.entries.len();
    let mut marks = logical.zerowidth.into_iter().peekable();
    for (k, mut cell) in logical.entries.into_iter().enumerate() {
        // Un caractère large ne tient pas dans une colonne : il y est réduit.
        let width = width_of(&cell).min(cols);
        if col + width > cols {
            line.wrapped = true;
            out.push(std::mem::replace(&mut line, Line::new(cols)));
            col = 0;
        }
        for (target, cursor) in moved.iter_mut().zip(&logical.cursors) {
            if *cursor == Some(k) {
                *target = Some((out.len(), col));
            }
        }
        if width == 1 {
            cell.style.attrs.remove(Attrs::WIDE);
        }
        line.cells_mut()[col] = cell;
        if marks.peek().is_some_and(|&(i, _)| i == k)
            && let Some((_, s)) = marks.next()
        {
            for ch in s.chars() {
                line.push_zerowidth(col, ch);
            }
        }
        if width == 2 {
            let mut half = Cell::new(' ', cell.style);
            half.style.attrs.remove(Attrs::WIDE);
            half.style.attrs.insert(Attrs::WIDE_CONTINUATION);
            line.cells_mut()[col + 1] = half;
        }
        col += width;
    }
    for (target, cursor) in moved.iter_mut().zip(&logical.cursors) {
        if cursor.is_some_and(|c| c >= count) {
            *target = Some((out.len(), col));
        }
    }
    out.push(line);
}

/// Même découpage que `layout` pour une ligne logique sans caractère large ni
/// combinant (presque toutes) : les cellules sont copiées par tranche de
/// `cols` et les curseurs placés par arithmétique.
fn layout_narrow(
    logical: Logical,
    cols: usize,
    out: &mut Vec<Line>,
    moved: &mut [Option<(usize, usize)>],
) {
    let first = out.len();
    let count = logical.entries.len();
    let mut chunks = logical.entries.chunks(cols).peekable();
    if chunks.peek().is_none() {
        out.push(Line::new(cols));
    }
    while let Some(chunk) = chunks.next() {
        let mut cells = Vec::with_capacity(cols);
        cells.extend_from_slice(chunk);
        cells.resize(cols, Cell::default());
        let mut line = Line::from_cells(cells);
        line.wrapped = chunks.peek().is_some();
        out.push(line);
    }
    // Les bandes vont à la première ligne produite, colonne inchangée.
    out[first].images = logical.images;
    let last = out.len() - 1;
    let end_col = count - (last - first) * cols;
    for (target, cursor) in moved.iter_mut().zip(&logical.cursors) {
        if let Some(k) = *cursor {
            *target = Some(if k < count {
                (first + k / cols, k % cols)
            } else {
                (last, end_col)
            });
        }
    }
}

#[cfg(test)]
mod perf;
#[cfg(test)]
mod tests;
