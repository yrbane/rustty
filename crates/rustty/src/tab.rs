//! Un onglet : un arbre de panneaux et, pour chaque feuille, le terminal
//! qui l'habite. Pur : aucun PTY ici, seulement des identifiants.

use std::collections::HashMap;

use rustty_layout::{Axis, Side, SplitId, TabLayout, WindowId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TermId(pub u64);

#[derive(Clone, Debug)]
pub struct Tab {
    pub layout: TabLayout,
    panes: HashMap<WindowId, TermId>,
    /// Accent tiré à la création (voir `accent::ACCENT_SLOTS`).
    pub accent: usize,
    /// Nom choisi par l'utilisateur ; remplace le titre du shell.
    pub custom_title: Option<String>,
    /// Couleur choisie au nuancier (index ANSI) ; `None` = celle de la configuration.
    pub color: Option<usize>,
    split_accents: HashMap<SplitId, usize>,
}

impl Tab {
    pub fn new(first: TermId, accent: usize) -> Self {
        let (layout, window) = TabLayout::new();
        Self {
            layout,
            panes: HashMap::from([(window, first)]),
            accent,
            custom_title: None,
            color: None,
            split_accents: HashMap::new(),
        }
    }

    /// Accent de la barre d'une division ; 0 si elle est inconnue.
    pub fn split_accent(&self, split: SplitId) -> usize {
        self.split_accents.get(&split).copied().unwrap_or(0)
    }

    pub fn term_at(&self, window: WindowId) -> Option<TermId> {
        self.panes.get(&window).copied()
    }

    pub fn window_of(&self, term: TermId) -> Option<WindowId> {
        self.panes
            .iter()
            .find(|(_, t)| **t == term)
            .map(|(w, _)| *w)
    }

    pub fn focused_term(&self) -> Option<TermId> {
        self.layout.focused().and_then(|w| self.term_at(w))
    }

    /// Les terminaux de l'onglet, dans l'ordre de l'arbre.
    pub fn terms(&self) -> Vec<TermId> {
        self.layout
            .windows()
            .into_iter()
            .filter_map(|w| self.term_at(w))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.layout.is_empty()
    }

    pub fn focus_term(&mut self, term: TermId) -> bool {
        self.window_of(term).is_some_and(|w| self.layout.focus(w))
    }

    /// Divise la fenêtre focalisée ; le nouveau panneau reçoit `new` et le
    /// focus, la nouvelle barre l'accent `accent`.
    pub fn split(&mut self, axis: Axis, new: TermId, accent: usize) -> bool {
        let Some(focused) = self.layout.focused() else {
            return false;
        };
        match self.layout.split(focused, axis) {
            Some(window) => {
                self.panes.insert(window, new);
                // Contrat de `TabLayout::split` : la division porte l'id de la fenêtre créée.
                self.split_accents.insert(SplitId(window.0), accent);
                true
            }
            None => false,
        }
    }

    /// Insère `term` (déjà détaché d'ailleurs) du côté `side` de `target` ;
    /// il reçoit le focus, la nouvelle barre l'accent `accent`.
    #[allow(dead_code)] // consommé par le glisser-déposer et les menus (tâches suivantes)
    pub fn insert_term(&mut self, term: TermId, target: TermId, side: Side, accent: usize) -> bool {
        let Some(target) = self.window_of(target) else {
            return false;
        };
        match self.layout.insert(target, side) {
            Some(window) => {
                self.register(window, term, accent);
                true
            }
            None => false,
        }
    }

    /// Comme `insert_term`, mais autour de toute la racine de l'onglet.
    #[allow(dead_code)] // consommé par le glisser-déposer et les menus (tâches suivantes)
    pub fn insert_term_at_root(&mut self, term: TermId, side: Side, accent: usize) {
        let window = self.layout.insert_root(side);
        self.register(window, term, accent);
    }

    /// Division qui contient directement le panneau de `term` (`None` à la racine).
    pub fn parent_split_of(&self, term: TermId) -> Option<SplitId> {
        self.window_of(term)
            .and_then(|w| self.layout.parent_split(w))
    }

    #[allow(dead_code)] // consommé par le glisser-déposer et les menus (tâches suivantes)
    fn register(&mut self, window: WindowId, term: TermId, accent: usize) {
        self.panes.insert(window, term);
        // Contrat de `TabLayout::insert` : la division porte l'id de la fenêtre créée.
        self.split_accents.insert(SplitId(window.0), accent);
    }

    pub fn close_term(&mut self, term: TermId) -> bool {
        let Some(window) = self.window_of(term) else {
            return false;
        };
        self.panes.remove(&window);
        self.layout.close(window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustty_layout::SplitId;

    #[test]
    fn a_split_remembers_its_accent_under_the_split_id() {
        let mut tab = Tab::new(TermId(1), 4);
        assert_eq!(tab.accent, 4);
        assert!(tab.split(Axis::Vertical, TermId(2), 7));
        let window = tab.window_of(TermId(2)).unwrap();
        assert_eq!(tab.split_accent(SplitId(window.0)), 7);
        assert_eq!(tab.split_accent(SplitId(999)), 0, "inconnu");
    }

    #[test]
    fn a_new_tab_has_one_focused_term() {
        let tab = Tab::new(TermId(7), 3);
        assert_eq!(tab.focused_term(), Some(TermId(7)));
        assert_eq!(tab.terms(), vec![TermId(7)]);
        assert!(!tab.is_empty());
    }

    #[test]
    fn split_adds_and_focuses_the_new_term() {
        let mut tab = Tab::new(TermId(1), 0);
        assert!(tab.split(Axis::Vertical, TermId(2), 5));
        assert_eq!(tab.focused_term(), Some(TermId(2)));
        assert_eq!(tab.terms().len(), 2);
        let w2 = tab.window_of(TermId(2)).unwrap();
        assert_eq!(tab.term_at(w2), Some(TermId(2)));
    }

    #[test]
    fn closing_terms_empties_the_tab() {
        let mut tab = Tab::new(TermId(1), 0);
        tab.split(Axis::Horizontal, TermId(2), 5);
        assert!(tab.close_term(TermId(2)));
        assert_eq!(tab.focused_term(), Some(TermId(1)));
        assert!(!tab.close_term(TermId(9)));
        assert!(tab.close_term(TermId(1)));
        assert!(tab.is_empty());
        assert_eq!(tab.focused_term(), None);
    }

    #[test]
    fn focus_term_moves_the_focus() {
        let mut tab = Tab::new(TermId(1), 0);
        tab.split(Axis::Vertical, TermId(2), 5);
        assert!(tab.focus_term(TermId(1)));
        assert_eq!(tab.focused_term(), Some(TermId(1)));
        assert!(!tab.focus_term(TermId(3)));
    }
}
