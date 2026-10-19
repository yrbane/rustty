//! Exécution des dépôts de glisser-déposer : seuls les arbres changent,
//! aucun shell n'est lancé, fermé ni vidé.

use super::{Effect, Model};
use crate::drag::PaneDrop;
use crate::tab::TermId;

#[allow(dead_code)] // câblé par la tâche 10
impl Model {
    /// Déplace le panneau `term` ; un terme ou une cible disparus annulent le dépôt.
    pub fn move_pane(&mut self, term: TermId, drop: PaneDrop) -> Vec<Effect> {
        self.renaming = None;
        let moved = match drop {
            PaneDrop::Pane { target, side } => self.workspace.move_term_to_pane(term, target, side),
            PaneDrop::Tab(tab) => self.workspace.move_term_to_tab(term, tab),
            PaneDrop::NewTab => self.workspace.move_term_to_new_tab(term),
        };
        Self::relayout_if(moved)
    }

    /// Déplace l'onglet `from` à l'index d'insertion `to`.
    pub fn move_tab(&mut self, from: usize, to: usize) -> Vec<Effect> {
        self.renaming = None;
        Self::relayout_if(self.workspace.move_tab(from, to))
    }

    /// Un glisser peut démarrer : ni confirmation ni renommage en cours.
    pub fn accepts_drag(&self) -> bool {
        self.pending_close.is_none() && self.renaming.is_none()
    }

    fn relayout_if(changed: bool) -> Vec<Effect> {
        if changed {
            vec![Effect::Relayout]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
#[path = "arrange_tests.rs"]
mod tests;
