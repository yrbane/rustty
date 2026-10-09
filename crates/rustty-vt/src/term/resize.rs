//! Redimensionnement : reflow de l'écran principal et de son historique quand
//! la largeur change, simple ajustement des rangées sinon.

use super::Term;
use crate::cell::Cell;
use crate::cursor::Cursor;
use crate::grid::Grid;
use crate::line::Line;
use crate::reflow::{is_blank, reflow};
use crate::region::ScrollRegion;
use crate::tabs::TabStops;

impl Term {
    /// Nouvelle taille en cellules. Un changement de largeur redécoupe l'écran
    /// principal et son historique (reflow) ; sinon les lignes sont seulement
    /// complétées ou tronquées.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let template = Cell::default();
        let old_rows = self.rows();
        let mut reflowed_cursor = None;
        let mut reflowed_saved = None;
        if cols != self.cols() {
            // L'écran principal est redécoupé même sous l'écran alternatif : son
            // curseur est alors celui sauvegardé à l'entrée (mode 1049). Sinon,
            // le curseur vivant et celui de DECSC suivent tous deux leur caractère.
            if self.modes.alt_screen {
                let saved = self.saved_cursor.cursor;
                reflowed_saved = self.reflow_primary(cols, rows, &[saved])[0];
            } else {
                let cursors = [self.cursor, self.saved_cursor.cursor];
                let moved = self.reflow_primary(cols, rows, &cursors);
                (reflowed_cursor, reflowed_saved) = (moved[0], moved[1]);
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
        // Après `clamp`, qui annule le retour à la ligne en attente.
        if let Some(at) = reflowed_saved {
            place(&mut self.saved_cursor.cursor, at, cols);
        }
        if let Some(at) = reflowed_cursor {
            place(&mut self.cursor, at, cols);
        }
    }

    /// Redécoupe l'historique et l'écran principal à `cols` colonnes et rend,
    /// pour chaque curseur, sa nouvelle position (ligne, colonne) dans la grille.
    /// Le premier curseur (le vivant) borne les rangées reprises ; un suivant
    /// au-delà, comme un DECSC périmé, rend `None` et sera seulement borné.
    fn reflow_primary(
        &mut self,
        cols: usize,
        rows: usize,
        cursors: &[Cursor],
    ) -> Vec<Option<(usize, usize)>> {
        let mut lines = self.scrollback.drain_all();
        let history = lines.len();
        let screen = self.grid.lines();
        let used = screen
            .iter()
            .rposition(|l| !l.images().is_empty() || !l.cells().iter().all(is_blank))
            .map_or(0, |i| i + 1)
            .max(cursors.first().map_or(0, |c| c.row + 1))
            .min(screen.len());
        lines.extend(screen[..used].iter().cloned());
        let positions: Vec<(usize, usize)> = cursors
            .iter()
            .filter(|c| c.row < used)
            .map(|c| (history + c.row, c.col + usize::from(c.pending_wrap)))
            .collect();
        let (mut lines, moved) = reflow(lines, &positions, cols);
        // Les curseurs hors des rangées reprises n'ont pas de position.
        let mut moved_iter = moved.into_iter();
        let moved: Vec<_> = cursors
            .iter()
            .map(|c| {
                if c.row < used {
                    moved_iter.next().flatten()
                } else {
                    None
                }
            })
            .collect();
        // Les `rows` dernières lignes à l'écran : aucun texte n'est perdu. Si
        // un curseur était plus haut, il reste en haut de l'écran (l'application
        // qui l'y a mis redessine à la réception du nouveau format).
        let start = lines.len().saturating_sub(rows);
        let kept = whole_lines_start(&lines[..start], self.scrollback.max_lines());
        let mut rest = lines.split_off(start);
        self.scrollback.extend(lines.split_off(kept));
        rest.truncate(rows);
        self.grid = Grid::from_lines(cols, rows, rest);
        moved
            .into_iter()
            .map(|at| at.map(|(row, col)| (row.saturating_sub(start), col)))
            .collect()
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

/// Pose un curseur reporté par le reflow ; une colonne égale à la largeur
/// devient un retour à la ligne en attente sur la dernière colonne.
fn place(cursor: &mut Cursor, (row, col): (usize, usize), cols: usize) {
    cursor.row = row;
    cursor.col = col.min(cols - 1);
    cursor.pending_wrap = col >= cols;
}

/// Premier indice de `history` à garder dans un historique de `max` lignes :
/// on retire en tête jusqu'au début d'une ligne logique, pour qu'aucune ligne
/// gardée ne soit la suite d'une ligne enroulée perdue.
fn whole_lines_start(history: &[Line], max: usize) -> usize {
    let mut first = history.len().saturating_sub(max);
    while first > 0 && first < history.len() && history[first - 1].wrapped {
        first += 1;
    }
    first
}
