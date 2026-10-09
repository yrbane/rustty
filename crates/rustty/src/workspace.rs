//! Les onglets d'une fenêtre : lequel est actif, quels terminaux ils
//! portent, et les opérations de la configuration (onglets, splits, focus).

use rustty_config::{FocusDirection, ResizeDir, SplitAxis};
use rustty_layout::{Axis, Direction};

use crate::accent::AccentPicker;
use crate::tab::{Tab, TermId};

/// Pas d'un redimensionnement au clavier, en fraction du parent.
const RESIZE_STEP: f32 = 0.05;

#[derive(Clone, Debug)]
pub struct Workspace {
    tabs: Vec<Tab>,
    active: usize,
    next_term: u64,
    picker: AccentPicker,
}

impl Workspace {
    pub fn new() -> (Self, TermId) {
        Self::with_picker(AccentPicker::from_clock())
    }

    /// Tirage reproductible des accents (tests).
    #[cfg(test)]
    pub fn with_seed(seed: u64) -> (Self, TermId) {
        Self::with_picker(AccentPicker::new(seed))
    }

    fn with_picker(mut picker: AccentPicker) -> (Self, TermId) {
        let first = TermId(1);
        let tab = Tab::new(first, picker.next());
        let ws = Self {
            tabs: vec![tab],
            active: 0,
            next_term: 2,
            picker,
        };
        (ws, first)
    }

    /// Nom personnalisé de l'onglet `index` ; `None` rend le titre du shell.
    pub fn rename(&mut self, index: usize, title: Option<String>) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.custom_title = title;
        }
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
        let accent = self.picker.next();
        self.tabs.insert(index, Tab::new(term, accent));
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
        let accent = self.picker.next();
        self.active_tab_mut()
            .split(layout_axis, term, accent)
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
#[path = "workspace_tests.rs"]
mod tests;
