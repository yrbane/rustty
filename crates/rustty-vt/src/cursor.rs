//! Position et style du curseur d'écriture, et sa forme de sauvegarde.

use crate::cell::Style;
use crate::charset::Charsets;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    pub col: usize,
    pub row: usize,
    /// État SGR courant, appliqué aux cellules écrites.
    pub style: Style,
    /// Le curseur est « au-delà » de la dernière colonne : le prochain
    /// caractère imprimable passe à la ligne (si autowrap).
    pub pending_wrap: bool,
}

impl Cursor {
    /// Ramène le curseur dans une grille `cols × rows` et annule le retour différé.
    pub fn clamp(&mut self, cols: usize, rows: usize) {
        self.col = self.col.min(cols.saturating_sub(1));
        self.row = self.row.min(rows.saturating_sub(1));
        self.pending_wrap = false;
    }
}

/// Forme demandée par DECSCUSR ; le renderer décide de l'apparence exacte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorShape {
    #[default]
    Block,
    Underline,
    Beam,
}

impl CursorShape {
    pub fn from_decscusr(param: u16) -> Self {
        match param {
            3 | 4 => Self::Underline,
            5 | 6 => Self::Beam,
            _ => Self::Block,
        }
    }
}

/// Curseur sauvegardé par DECSC / CSI s, avec le contexte qui l'accompagne.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SavedCursor {
    pub cursor: Cursor,
    pub origin: bool,
    pub charsets: Charsets,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_brings_cursor_inside_grid_and_cancels_wrap() {
        let mut c = Cursor {
            col: 9,
            row: 7,
            pending_wrap: true,
            ..Cursor::default()
        };
        c.clamp(4, 2);
        assert_eq!((c.col, c.row), (3, 1));
        assert!(!c.pending_wrap);
    }

    #[test]
    fn decscusr_mapping() {
        assert_eq!(CursorShape::from_decscusr(0), CursorShape::Block);
        assert_eq!(CursorShape::from_decscusr(1), CursorShape::Block);
        assert_eq!(CursorShape::from_decscusr(2), CursorShape::Block);
        assert_eq!(CursorShape::from_decscusr(3), CursorShape::Underline);
        assert_eq!(CursorShape::from_decscusr(4), CursorShape::Underline);
        assert_eq!(CursorShape::from_decscusr(5), CursorShape::Beam);
        assert_eq!(CursorShape::from_decscusr(6), CursorShape::Beam);
        assert_eq!(CursorShape::from_decscusr(99), CursorShape::Block);
    }
}
