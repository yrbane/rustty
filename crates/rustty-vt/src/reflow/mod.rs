//! Reflow : redécoupe les lignes à une nouvelle largeur en respectant les
//! retours à la ligne automatiques (lignes `wrapped`), comme le fait kitty
//! quand on redimensionne un panneau. Pur : des lignes en entrée, des lignes
//! en sortie, et la position des curseurs reportée sur le même caractère.

use crate::cell::{Attrs, Cell};
use crate::graphics::ImageStrip;
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
        is_blank(&self.cell) && self.zerowidth.is_none()
    }
}

/// Une espace sans attribut est un blanc, quelles que soient ses couleurs : le
/// fond d'une queue colorée est perdu au reflow plutôt que d'y fabriquer des
/// lignes vides. Une seconde moitié de caractère large n'en est jamais un.
pub(crate) fn is_blank(cell: &Cell) -> bool {
    cell.c == ' ' && cell.style.attrs.is_empty()
}

/// Une ligne logique : ce que l'application a écrit entre deux retours à la
/// ligne explicites, la position de chaque curseur en entrées, s'il y est, et
/// les bandes d'image de toutes ses lignes source.
struct Logical {
    entries: Vec<Entry>,
    cursors: Vec<Option<usize>>,
    images: Vec<ImageStrip>,
}

impl Logical {
    fn new(cursors: usize) -> Self {
        Self {
            entries: Vec::new(),
            cursors: vec![None; cursors],
            images: Vec::new(),
        }
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
    for (col, cell) in line.cells().iter().enumerate() {
        for (slot, &cursor) in logical.cursors.iter_mut().zip(cursors) {
            if cursor == (row, col) {
                // Sur une seconde moitié, le curseur désigne le caractère large.
                let back = usize::from(cell.is_wide_continuation());
                *slot = Some(logical.entries.len().saturating_sub(back));
            }
        }
        if !cell.is_wide_continuation() {
            logical.entries.push(Entry {
                cell: *cell,
                zerowidth: line.zerowidth(col).map(str::to_owned),
            });
        }
    }
    for (slot, &(r, c)) in logical.cursors.iter_mut().zip(cursors) {
        if r == row && c >= line.len() {
            *slot = Some(logical.entries.len() + (c - line.len()));
        }
    }
}

/// Retire les blancs de fin, sauf ceux qui précèdent le premier curseur (le
/// curseur vivant) ; les suivants, au-delà du texte, s'y posent en fin.
fn trimmed(mut logical: Logical) -> Logical {
    let keep = logical.cursors.first().copied().flatten().unwrap_or(0);
    while logical.entries.len() > keep && logical.entries.last().is_some_and(Entry::is_blank) {
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
    let mut line = Line::new(cols);
    // Les bandes vont à la première ligne produite, colonne inchangée.
    line.images = logical.images;
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
        for (target, cursor) in moved.iter_mut().zip(&logical.cursors) {
            if *cursor == Some(k) {
                *target = Some((out.len(), col));
            }
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
    for (target, cursor) in moved.iter_mut().zip(&logical.cursors) {
        if cursor.is_some_and(|c| c >= count) {
            *target = Some((out.len(), col));
        }
    }
    out.push(line);
}

#[cfg(test)]
mod perf;
#[cfg(test)]
mod tests;
