//! Fermeture (avec confirmation) et renommage des onglets du modèle.

use super::{CloseRequest, Effect, Model};
use crate::rename::{Rename, RenameKey, RenameOutcome};
use crate::tab::{Tab, TermId};

impl Model {
    pub(super) fn terms_of(&self, req: CloseRequest) -> Vec<TermId> {
        match req {
            CloseRequest::Tab(i) => self
                .workspace
                .tabs()
                .get(i)
                .map(|t| t.terms())
                .unwrap_or_default(),
            CloseRequest::Term(t) => vec![t],
            CloseRequest::Window => self.workspace.tabs().iter().flat_map(Tab::terms).collect(),
        }
    }

    pub fn request_close_window(
        &mut self,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        let needs = confirm_close
            && self
                .terms_of(CloseRequest::Window)
                .iter()
                .any(|t| running(*t));
        self.request_close(CloseRequest::Window, needs)
    }

    pub fn request_close(&mut self, req: CloseRequest, needs_confirm: bool) -> Vec<Effect> {
        if needs_confirm {
            self.pending_close = Some(req);
            return vec![Effect::Redraw];
        }
        self.execute_close(req)
    }

    /// Ouvre l'édition du nom de l'onglet `tab`, pré-remplie avec son nom actuel.
    pub fn start_rename(&mut self, tab: usize) -> Vec<Effect> {
        let Some(t) = self.workspace.tabs().get(tab) else {
            return Vec::new();
        };
        let current = t.custom_title.clone().unwrap_or_default();
        self.renaming = Some(Rename::new(tab, &current));
        vec![Effect::Redraw]
    }

    pub fn rename_key(&mut self, key: RenameKey) -> Vec<Effect> {
        let Some(rename) = self.renaming.as_mut() else {
            return Vec::new();
        };
        match rename.apply(key) {
            // La largeur de l'onglet suit le nom tapé : mise en page complète.
            RenameOutcome::Editing => vec![Effect::Relayout],
            RenameOutcome::Commit(title) => {
                let tab = rename.tab;
                self.renaming = None;
                self.workspace.rename(tab, title);
                vec![Effect::Relayout]
            }
            RenameOutcome::Cancel => {
                self.renaming = None;
                vec![Effect::Redraw]
            }
        }
    }

    pub(super) fn execute_close(&mut self, req: CloseRequest) -> Vec<Effect> {
        // Les index d'onglet bougent : une édition en cours n'a plus de cible sûre.
        self.renaming = None;
        let killed = match req {
            CloseRequest::Tab(i) => self.workspace.close_tab(i),
            CloseRequest::Term(t) => self
                .workspace
                .close_term(t)
                .then_some(t)
                .into_iter()
                .collect(),
            CloseRequest::Window => {
                let mut all = Vec::new();
                while !self.workspace.is_empty() {
                    all.extend(self.workspace.close_tab(0));
                }
                all
            }
        };
        let mut effects: Vec<Effect> = killed.iter().map(|t| Effect::CloseTerm(*t)).collect();
        for t in &killed {
            self.dead.remove(t);
        }
        effects.push(if self.workspace.is_empty() {
            Effect::Exit
        } else {
            Effect::Relayout
        });
        effects
    }

    pub fn confirm_pending(&mut self) -> Vec<Effect> {
        match self.pending_close.take() {
            Some(req) => self.execute_close(req),
            None => Vec::new(),
        }
    }

    pub fn cancel_pending(&mut self) -> Vec<Effect> {
        match self.pending_close.take() {
            Some(_) => vec![Effect::Redraw],
            None => Vec::new(),
        }
    }
}
