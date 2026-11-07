//! Copie figée de ce qui doit être dessiné. Le renderer ne touche jamais
//! `Term` directement : il reçoit un `Snapshot` pris sous verrou.

use std::sync::Arc;

use crate::cursor::{Cursor, CursorShape};
use crate::line::Line;
use crate::links::{LinkId, LinkTable};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub cols: usize,
    pub rows: usize,
    /// Les `rows` lignes visibles, de haut en bas, historique compris.
    pub lines: Vec<Line>,
    /// Position à l'écran, `None` si invisible ou hors de la zone affichée.
    pub cursor: Option<Cursor>,
    pub cursor_shape: CursorShape,
    pub display_offset: usize,
    pub scrollback_len: usize,
    pub title: String,
    /// Table des hyperliens, partagée avec le terminal sans copie.
    pub links: Arc<LinkTable>,
}

impl Snapshot {
    pub fn link_uri(&self, id: LinkId) -> Option<&str> {
        self.links.uri(id)
    }
}
