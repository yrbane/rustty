//! Les onglets d'une fenêtre : lequel est actif, quels terminaux ils
//! portent, et les opérations de la configuration (onglets, splits, focus).

use rustty_config::{FocusDirection, ResizeDir, SplitAxis};
use rustty_layout::{Axis, Direction, WindowId};

use crate::tab::{Tab, TermId};

/// Pas d'un redimensionnement au clavier, en fraction du parent.
const RESIZE_STEP: f32 = 0.05;

#[derive(Clone, Debug)]
pub struct Workspace {
    tabs: Vec<Tab>,
    active: usize,
    next_term: u64,
}

impl Workspace {
    pub fn new() -> (Self, TermId) {
        let first = TermId(1);
        (
            Self {
                tabs: vec![Tab::new(first)],
                active: 0,
                next_term: 2,
            },
            first,
        )
    }

    fn alloc_term(&mut self) -> TermId {
        let id = TermId(self.next_term);
        self.next_term += 1;
        id
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active(&self) -> usize {
        self.active
    }

    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn focused_term(&self) -> Option<TermId> {
        self.tabs.get(self.active).and_then(Tab::focused_term)
    }

    pub fn window_of_focused(&self) -> Option<WindowId> {
        self.tabs.get(self.active).and_then(|t| t.layout.focused())
    }

    pub fn tab_of(&self, term: TermId) -> Option<usize> {
        self.tabs.iter().position(|t| t.window_of(term).is_some())
    }

    pub fn new_tab(&mut self) -> TermId {
        let term = self.alloc_term();
        let index = if self.tabs.is_empty() {
            0
        } else {
            self.active + 1
        };
        self.tabs.insert(index, Tab::new(term));
        self.active = index;
        term
    }

    /// Retire l'onglet et rend les terminaux à tuer.
    pub fn close_tab(&mut self, index: usize) -> Vec<TermId> {
        if index >= self.tabs.len() {
            return Vec::new();
        }
        let tab = self.tabs.remove(index);
        if self.active >= index {
            self.active = self.active.saturating_sub(1);
        }
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len().saturating_sub(1);
        }
        tab.terms()
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
        }
    }

    /// Onglet `n` (à partir de 1) ; ignoré hors limites.
    pub fn go_to_tab(&mut self, n: u8) {
        let index = usize::from(n).wrapping_sub(1);
        if n >= 1 && index < self.tabs.len() {
            self.active = index;
        }
    }

    pub fn split_focused(&mut self, axis: SplitAxis) -> Option<TermId> {
        if self.tabs.is_empty() {
            return None;
        }
        let term = self.alloc_term();
        let layout_axis = match axis {
            SplitAxis::Horizontal => Axis::Horizontal,
            SplitAxis::Vertical => Axis::Vertical,
        };
        self.active_tab_mut()
            .split(layout_axis, term)
            .then_some(term)
    }

    /// Retire le panneau de `term` ; un onglet vidé disparaît.
    pub fn close_term(&mut self, term: TermId) -> bool {
        let Some(index) = self.tab_of(term) else {
            return false;
        };
        let closed = self.tabs[index].close_term(term);
        if self.tabs[index].is_empty() {
            self.close_tab(index);
        }
        closed
    }

    pub fn focus_term(&mut self, term: TermId) -> bool {
        let Some(index) = self.tab_of(term) else {
            return false;
        };
        self.active = index;
        self.tabs[index].focus_term(term)
    }

    pub fn focus(&mut self, direction: FocusDirection) -> bool {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return false;
        };
        let Some(focused) = tab.layout.focused() else {
            return false;
        };
        let dir = match direction {
            FocusDirection::Left => Direction::Left,
            FocusDirection::Right => Direction::Right,
            FocusDirection::Up => Direction::Up,
            FocusDirection::Down => Direction::Down,
        };
        tab.layout
            .neighbor(focused, dir)
            .is_some_and(|n| tab.layout.focus(n))
    }

    pub fn resize(&mut self, dir: ResizeDir) -> bool {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return false;
        };
        let Some(focused) = tab.layout.focused() else {
            return false;
        };
        let (axis, delta) = match dir {
            ResizeDir::Narrower => (Axis::Vertical, -RESIZE_STEP),
            ResizeDir::Wider => (Axis::Vertical, RESIZE_STEP),
            ResizeDir::Shorter => (Axis::Horizontal, -RESIZE_STEP),
            ResizeDir::Taller => (Axis::Horizontal, RESIZE_STEP),
        };
        tab.layout.resize(focused, axis, delta)
    }

    pub fn rotate(&mut self) -> bool {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return false;
        };
        tab.layout.focused().is_some_and(|f| tab.layout.rotate(f))
    }

    pub fn toggle_zoom(&mut self) -> bool {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return false;
        };
        tab.layout
            .focused()
            .is_some_and(|f| tab.layout.toggle_zoom(f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_one_tab_and_one_term() {
        let (ws, first) = Workspace::new();
        assert_eq!(first, TermId(1));
        assert_eq!(ws.tabs().len(), 1);
        assert_eq!(ws.active(), 0);
        assert_eq!(ws.focused_term(), Some(TermId(1)));
    }

    #[test]
    fn new_tab_is_inserted_after_the_active_one_and_activated() {
        let (mut ws, _) = Workspace::new();
        let t2 = ws.new_tab();
        assert_eq!((t2, ws.active()), (TermId(2), 1));
        ws.go_to_tab(1);
        let t3 = ws.new_tab();
        assert_eq!(
            (t3, ws.active()),
            (TermId(3), 1),
            "inséré après l'onglet 1, avant l'ancien onglet 2"
        );
        assert_eq!(ws.tabs()[2].focused_term(), Some(TermId(2)));
    }

    #[test]
    fn next_prev_and_go_to_wrap_and_clamp() {
        let (mut ws, _) = Workspace::new();
        ws.new_tab();
        ws.new_tab();
        ws.go_to_tab(1);
        ws.prev_tab();
        assert_eq!(ws.active(), 2, "circulaire vers la fin");
        ws.next_tab();
        assert_eq!(ws.active(), 0);
        ws.go_to_tab(9);
        assert_eq!(ws.active(), 0, "numéro hors limites ignoré");
        ws.go_to_tab(0);
        assert_eq!(ws.active(), 0);
    }

    #[test]
    fn closing_a_tab_returns_its_terms_and_moves_the_active_index() {
        let (mut ws, t1) = Workspace::new();
        let t2 = ws.new_tab();
        let t3 = ws.split_focused(SplitAxis::Vertical).unwrap();
        assert_eq!(ws.tab_of(t3), Some(1));
        let killed = ws.close_tab(1);
        assert_eq!(killed.len(), 2);
        assert!(killed.contains(&t2) && killed.contains(&t3));
        assert_eq!((ws.tabs().len(), ws.active()), (1, 0));
        assert_eq!(ws.focused_term(), Some(t1));
        assert!(ws.close_tab(5).is_empty(), "index invalide");
        assert_eq!(ws.close_tab(0), vec![t1]);
        assert!(ws.is_empty());
        assert_eq!(ws.focused_term(), None);
    }

    #[test]
    fn closing_the_first_tab_keeps_the_next_one_active() {
        let (mut ws, _) = Workspace::new();
        ws.new_tab();
        ws.go_to_tab(1);
        ws.close_tab(0);
        assert_eq!((ws.tabs().len(), ws.active()), (1, 0));
    }

    #[test]
    fn closing_terms_removes_empty_tabs() {
        let (mut ws, t1) = Workspace::new();
        let t2 = ws.split_focused(SplitAxis::Horizontal).unwrap();
        assert!(ws.close_term(t2));
        assert_eq!(ws.tabs().len(), 1);
        assert!(!ws.close_term(TermId(42)));
        assert!(ws.close_term(t1));
        assert!(ws.is_empty());
    }

    #[test]
    fn focus_resize_rotate_zoom_delegate_to_the_layout() {
        let (mut ws, t1) = Workspace::new();
        let t2 = ws.split_focused(SplitAxis::Vertical).unwrap();
        assert_eq!(ws.focused_term(), Some(t2));
        assert!(ws.focus(FocusDirection::Left));
        assert_eq!(ws.focused_term(), Some(t1));
        assert!(!ws.focus(FocusDirection::Up), "pas de voisin au-dessus");
        assert!(ws.resize(ResizeDir::Wider));
        assert!(ws.rotate());
        assert!(ws.toggle_zoom());
        assert_eq!(ws.active_tab().layout.zoomed(), ws.window_of_focused());
        assert!(ws.toggle_zoom());
        assert!(ws.focus_term(t2));
        assert_eq!(ws.focused_term(), Some(t2));
    }

    #[test]
    fn focus_term_switches_tabs() {
        let (mut ws, t1) = Workspace::new();
        ws.new_tab();
        assert!(ws.focus_term(t1));
        assert_eq!(ws.active(), 0);
        assert!(!ws.focus_term(TermId(99)));
    }
}
