//! Un onglet : un arbre de panneaux et, pour chaque feuille, le terminal
//! qui l'habite. Pur : aucun PTY ici, seulement des identifiants.

use std::collections::HashMap;

use rustty_layout::{Axis, TabLayout, WindowId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TermId(pub u64);

#[derive(Clone, Debug)]
pub struct Tab {
    pub layout: TabLayout,
    panes: HashMap<WindowId, TermId>,
}

impl Tab {
    pub fn new(first: TermId) -> Self {
        let (layout, window) = TabLayout::new();
        Self {
            layout,
            panes: HashMap::from([(window, first)]),
        }
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

    /// Divise la fenêtre focalisée ; le nouveau panneau reçoit `new` et le focus.
    pub fn split(&mut self, axis: Axis, new: TermId) -> bool {
        let Some(focused) = self.layout.focused() else {
            return false;
        };
        match self.layout.split(focused, axis) {
            Some(window) => {
                self.panes.insert(window, new);
                true
            }
            None => false,
        }
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

    #[test]
    fn a_new_tab_has_one_focused_term() {
        let tab = Tab::new(TermId(7));
        assert_eq!(tab.focused_term(), Some(TermId(7)));
        assert_eq!(tab.terms(), vec![TermId(7)]);
        assert!(!tab.is_empty());
    }

    #[test]
    fn split_adds_and_focuses_the_new_term() {
        let mut tab = Tab::new(TermId(1));
        assert!(tab.split(Axis::Vertical, TermId(2)));
        assert_eq!(tab.focused_term(), Some(TermId(2)));
        assert_eq!(tab.terms().len(), 2);
        let w2 = tab.window_of(TermId(2)).unwrap();
        assert_eq!(tab.term_at(w2), Some(TermId(2)));
    }

    #[test]
    fn closing_terms_empties_the_tab() {
        let mut tab = Tab::new(TermId(1));
        tab.split(Axis::Horizontal, TermId(2));
        assert!(tab.close_term(TermId(2)));
        assert_eq!(tab.focused_term(), Some(TermId(1)));
        assert!(!tab.close_term(TermId(9)));
        assert!(tab.close_term(TermId(1)));
        assert!(tab.is_empty());
        assert_eq!(tab.focused_term(), None);
    }

    #[test]
    fn focus_term_moves_the_focus() {
        let mut tab = Tab::new(TermId(1));
        tab.split(Axis::Vertical, TermId(2));
        assert!(tab.focus_term(TermId(1)));
        assert_eq!(tab.focused_term(), Some(TermId(1)));
        assert!(!tab.focus_term(TermId(3)));
    }
}
