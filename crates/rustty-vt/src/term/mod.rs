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

use std::sync::Arc;

use vte::Parser;

use crate::apc::{ApcSplitter, Chunk};
use crate::cell::Cell;
use crate::charset::Charsets;
use crate::cursor::{Cursor, CursorShape, SavedCursor};
use crate::graphics::{Chunks, ImageStore};
use crate::grid::Grid;
use crate::links::{LinkId, LinkTable};
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
    /// Hyperliens OSC 8 déclarés, partagés avec les instantanés (copie à
    /// l'écriture) ; une seule table pour les deux écrans.
    pub(crate) links: Arc<LinkTable>,
    /// Lien que portent les cellules imprimées jusqu'au prochain `OSC 8 ; ;`.
    pub(crate) current_link: Option<LinkId>,
    /// Compactages de la table des liens (tests).
    pub(crate) link_compactions: u64,
    /// Liens refusés de suite, table pleine : le balayage complet n'est
    /// retenté qu'un refus sur `LINK_RESCAN_EVERY`.
    pub(crate) link_refusals: u32,
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
            links: Arc::default(),
            current_link: None,
            link_compactions: 0,
            link_refusals: 0,
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

    pub fn links(&self) -> &Arc<LinkTable> {
        &self.links
    }

    pub fn current_link(&self) -> Option<LinkId> {
        self.current_link
    }

    /// Table pleine de liens vivants : un rebalayage par autant de refus.
    pub(crate) const LINK_RESCAN_EVERY: u32 = 1024;

    pub fn link_compactions(&self) -> u64 {
        self.link_compactions
    }

    /// Réduit la table aux liens encore portés par une cellule (écrans et
    /// historique) et renumérote ces cellules.
    pub fn compact_links(&mut self) {
        self.link_compactions += 1;
        let mut used = vec![false; self.links.len() + 1];
        let mark = |line: &crate::line::Line, used: &mut Vec<bool>| {
            for cell in line.cells() {
                if let Some(id) = cell.link {
                    used[usize::from(id.get())] = true;
                }
            }
        };
        // L'écran alternatif inactif est vidé à sa prochaine entrée (mode
        // 1049) : ses cellules ne retiennent aucun lien.
        let alt_live = self.modes.alt_screen;
        for line in self
            .grid
            .lines()
            .iter()
            .chain(self.alt_grid.lines().iter().filter(|_| alt_live))
        {
            mark(line, &mut used);
        }
        for line in self.scrollback.iter() {
            mark(line, &mut used);
        }
        let (kept, map) = self.links.retain_remapped(|id| used[usize::from(id.get())]);
        let remap = |cell: &mut Cell| {
            if let Some(id) = cell.link {
                cell.link = map[usize::from(id.get())];
            }
        };
        for line in self
            .grid
            .lines_mut()
            .iter_mut()
            .chain(self.alt_grid.lines_mut())
            .chain(self.scrollback.iter_mut())
        {
            for cell in line.cells_mut() {
                remap(cell);
            }
        }
        self.current_link = self.current_link.and_then(|id| map[usize::from(id.get())]);
        self.links = Arc::new(kept);
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
