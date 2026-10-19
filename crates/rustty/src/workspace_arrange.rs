//! Déplacements de panneaux et d'onglets, couleur d'onglet et noms de
//! panneaux. Enfant de `workspace` pour toucher ses champs privés. Pur :
//! seuls les arbres et les tables `WindowId → TermId` changent.

#![allow(dead_code)] // consommé par le glisser-déposer et les menus (tâches suivantes)

use rustty_layout::Side;

use super::Workspace;
use crate::tab::{Tab, TermId};

impl Workspace {
    /// Déplace `term` du côté `side` du panneau `target` (même onglet ou autre).
    pub fn move_term_to_pane(&mut self, term: TermId, target: TermId, side: Side) -> bool {
        if term == target {
            return false;
        }
        let (Some(from), Some(to)) = (self.tab_of(term), self.tab_of(target)) else {
            return false;
        };
        self.tabs[from].close_term(term);
        let accent = self.picker.next();
        self.tabs[to].insert_term(term, target, side, accent);
        self.finish_move(term, from);
        true
    }

    /// Déplace `term` à droite de tout l'onglet `tab` (permis vers son propre onglet).
    pub fn move_term_to_tab(&mut self, term: TermId, tab: usize) -> bool {
        let Some(from) = self.tab_of(term) else {
            return false;
        };
        if tab >= self.tabs.len() || (from == tab && self.tabs[from].terms().len() < 2) {
            return false;
        }
        self.tabs[from].close_term(term);
        let accent = self.picker.next();
        self.tabs[tab].insert_term_at_root(term, Side::Right, accent);
        self.finish_move(term, from);
        true
    }

    /// Déplace `term` dans un nouvel onglet placé juste après l'onglet d'origine.
    pub fn move_term_to_new_tab(&mut self, term: TermId) -> bool {
        let Some(from) = self.tab_of(term) else {
            return false;
        };
        if self.tabs[from].terms().len() < 2 {
            return false;
        }
        self.tabs[from].close_term(term);
        let accent = self.picker.next();
        self.tabs.insert(from + 1, Tab::new(term, accent));
        self.finish_move(term, from);
        true
    }

    /// Déplace l'onglet `from` à l'index d'insertion `to` (`0..=len`, compté
    /// avant retrait). L'onglet actif reste le même onglet.
    pub fn move_tab(&mut self, from: usize, to: usize) -> bool {
        let len = self.tabs.len();
        if from >= len || to > len || to == from || to == from + 1 {
            return false;
        }
        let dest = if to > from { to - 1 } else { to };
        let tab = self.tabs.remove(from);
        self.tabs.insert(dest, tab);
        self.active = if self.active == from {
            dest
        } else if from < self.active && self.active <= dest {
            self.active - 1
        } else if dest <= self.active && self.active < from {
            self.active + 1
        } else {
            self.active
        };
        true
    }

    /// Ferme tous les onglets sauf `keep` et rend leurs terminaux à tuer.
    pub fn close_other_tabs(&mut self, keep: usize) -> Vec<TermId> {
        if keep >= self.tabs.len() {
            return Vec::new();
        }
        let kept = self.tabs.swap_remove(keep);
        let killed: Vec<TermId> = self.tabs.drain(..).flat_map(|t| t.terms()).collect();
        for term in &killed {
            self.pane_names.remove(term);
        }
        self.tabs.push(kept);
        self.active = 0;
        killed
    }

    /// Couleur d'onglet (index ANSI) ; `None` rend celle de la configuration.
    pub fn set_tab_color(&mut self, tab: usize, color: Option<usize>) {
        if let Some(tab) = self.tabs.get_mut(tab) {
            tab.color = color;
        }
    }

    /// Nom du panneau ; vide ou `None` l'efface.
    pub fn rename_pane(&mut self, term: TermId, name: Option<String>) {
        match name.filter(|n| !n.is_empty()) {
            Some(name) => {
                self.pane_names.insert(term, name);
            }
            None => {
                self.pane_names.remove(&term);
            }
        }
    }

    pub fn pane_name(&self, term: TermId) -> Option<&str> {
        self.pane_names.get(&term).map(String::as_str)
    }

    /// Fin commune d'un déplacement : retire l'onglet d'origine s'il est vidé,
    /// puis active l'onglet d'arrivée et focalise `term`.
    fn finish_move(&mut self, term: TermId, from: usize) {
        if self.tabs[from].is_empty() {
            self.close_tab(from);
        }
        self.focus_term(term);
    }
}

#[cfg(test)]
#[path = "workspace_arrange_tests.rs"]
mod tests;
