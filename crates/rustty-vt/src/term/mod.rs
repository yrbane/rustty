//! Façade du terminal : compose l'état et expose l'API publique. Toute la
//! logique d'interprétation vit dans les sous-modules par famille d'opérations ;
//! `perform.rs` ne fait que du dispatch.

mod edit;
mod mode_ops;
mod movement;
mod osc;
mod perform;
mod print;
mod reports;
mod reset;
mod scroll;

use vte::Parser;

use crate::cell::Cell;
use crate::charset::Charsets;
use crate::cursor::{Cursor, CursorShape, SavedCursor};
use crate::grid::Grid;
use crate::modes::Modes;
use crate::outbox::{Outbox, TermEvent};
use crate::region::ScrollRegion;
use crate::scrollback::Scrollback;
use crate::tabs::TabStops;

pub struct Term {
    pub(crate) grid: Grid,
    pub(crate) alt_grid: Grid,
    pub(crate) scrollback: Scrollback,
    pub(crate) cursor: Cursor,
    pub(crate) saved_cursor: SavedCursor,
    pub(crate) saved_cursor_alt: SavedCursor,
    pub(crate) modes: Modes,
    pub(crate) cursor_shape: CursorShape,
    pub(crate) region: ScrollRegion,
    pub(crate) tabs: TabStops,
    pub(crate) charsets: Charsets,
    pub(crate) title: String,
    pub(crate) outbox: Outbox,
    parser: Parser,
}

impl Term {
    pub fn new(cols: usize, rows: usize, scrollback_lines: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            grid: Grid::new(cols, rows),
            alt_grid: Grid::new(cols, rows),
            scrollback: Scrollback::new(scrollback_lines),
            cursor: Cursor::default(),
            saved_cursor: SavedCursor::default(),
            saved_cursor_alt: SavedCursor::default(),
            modes: Modes::default(),
            cursor_shape: CursorShape::default(),
            region: ScrollRegion::full(rows),
            tabs: TabStops::new(cols),
            charsets: Charsets::default(),
            title: String::new(),
            outbox: Outbox::default(),
            parser: Parser::new(),
        }
    }

    /// Interprète des octets venus du PTY.
    pub fn input(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::take(&mut self.parser);
        parser.advance(self, bytes);
        self.parser = parser;
    }

    pub fn grid(&self) -> &Grid {
        self.active_grid()
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn cursor_shape(&self) -> CursorShape {
        self.cursor_shape
    }

    pub fn modes(&self) -> &Modes {
        &self.modes
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    pub fn drain_events(&mut self) -> Vec<TermEvent> {
        self.outbox.drain_events()
    }

    /// Octets à renvoyer à l'application (réponses aux requêtes).
    pub fn drain_responses(&mut self) -> Vec<u8> {
        self.outbox.drain_responses()
    }

    /// Texte de l'écran actif, pour les tests.
    pub fn text(&self) -> Vec<String> {
        self.active_grid().text()
    }

    pub(crate) fn active_grid(&self) -> &Grid {
        if self.modes.alt_screen {
            &self.alt_grid
        } else {
            &self.grid
        }
    }

    pub(crate) fn active_grid_mut(&mut self) -> &mut Grid {
        if self.modes.alt_screen {
            &mut self.alt_grid
        } else {
            &mut self.grid
        }
    }

    pub(crate) fn cols(&self) -> usize {
        self.grid.cols()
    }

    pub(crate) fn rows(&self) -> usize {
        self.grid.rows()
    }

    /// Cellule utilisée pour effacer : couleurs courantes, pas d'attribut.
    pub(crate) fn erase_template(&self) -> Cell {
        Cell::erased(self.cursor.style)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::Term;

    pub fn term(cols: usize, rows: usize) -> Term {
        Term::new(cols, rows, 100)
    }

    pub fn feed(t: &mut Term, s: &str) {
        t.input(s.as_bytes());
    }
}
