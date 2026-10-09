//! Façade du terminal : compose l'état et expose l'API publique. Toute la
//! logique d'interprétation vit dans les sous-modules par famille d'opérations ;
//! `perform.rs` ne fait que du dispatch.

mod edit;
mod graphics;
#[cfg(test)]
mod graphics_tests;
mod image_budget;
#[cfg(test)]
mod image_budget_tests;
mod mode_ops;
mod movement;
mod osc;
mod perform;
mod print;
mod reports;
mod reset;
mod resize;
#[cfg(test)]
mod resize_tests;
mod scroll;
mod view;

use vte::Parser;

use crate::apc::{ApcSplitter, Chunk};
use crate::cell::Cell;
use crate::charset::Charsets;
use crate::cursor::{Cursor, CursorShape, SavedCursor};
use crate::graphics::{Chunks, ImageStore};
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
    /// Décalage d'affichage dans le scrollback : 0 = écran vivant.
    pub(crate) display_offset: usize,
    pub(crate) outbox: Outbox,
    /// Morceaux graphiques en cours de réassemblage.
    pub(crate) chunks: Chunks,
    /// Images transmises avec un identifiant.
    pub(crate) images: ImageStore,
    /// Taille d'une cellule en pixels (largeur, hauteur), pour dimensionner les images.
    pub(crate) cell_pixels: (u32, u32),
    /// Plafond d'octets des images posées (`PLACED_BUDGET`, réduit en test).
    pub(crate) placed_budget: usize,
    parser: Parser,
    apc: ApcSplitter,
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
            display_offset: 0,
            outbox: Outbox::default(),
            chunks: Chunks::default(),
            images: ImageStore::default(),
            cell_pixels: (10, 20),
            placed_budget: image_budget::PLACED_BUDGET,
            parser: Parser::new(),
            apc: ApcSplitter::default(),
        }
    }

    /// Interprète des octets venus du PTY.
    pub fn input(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::take(&mut self.parser);
        let mut chunks = Vec::new();
        self.apc.feed(bytes, &mut chunks);
        for chunk in chunks {
            match chunk {
                Chunk::Bytes(b) => parser.advance(self, b),
                Chunk::Owned(b) => parser.advance(self, &b),
                Chunk::Apc(payload) => self.apc(&payload),
            }
        }
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
