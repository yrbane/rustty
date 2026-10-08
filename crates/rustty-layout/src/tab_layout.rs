//! Façade d'un onglet : l'arbre de divisions, la fenêtre focalisée et le zoom.

use crate::geometry::{Rect, WindowId};
use crate::node::Node;

#[derive(Clone, Debug, PartialEq)]
pub struct TabLayout {
    root: Option<Node>,
    focused: Option<WindowId>,
}

impl TabLayout {
    /// Un onglet naît avec une fenêtre, focalisée.
    pub fn new() -> (Self, WindowId) {
        let first = WindowId(1);
        (
            Self {
                root: Some(Node::Leaf(first)),
                focused: Some(first),
            },
            first,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub fn windows(&self) -> Vec<WindowId> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.leaves(&mut out);
        }
        out
    }

    pub fn focused(&self) -> Option<WindowId> {
        self.focused
    }

    pub fn contains(&self, id: WindowId) -> bool {
        self.root.as_ref().is_some_and(|r| r.contains(id))
    }

    /// Vrai si la fenêtre existe et a pris le focus.
    pub fn focus(&mut self, id: WindowId) -> bool {
        if !self.contains(id) {
            return false;
        }
        self.focused = Some(id);
        true
    }

    pub fn rects(&self, bounds: Rect, gap: u32) -> Vec<(WindowId, Rect)> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.rects(bounds, gap, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_layout_has_one_focused_window() {
        let (layout, first) = TabLayout::new();
        assert_eq!(first, WindowId(1));
        assert!(!layout.is_empty());
        assert_eq!(layout.windows(), vec![first]);
        assert_eq!(layout.focused(), Some(first));
        assert!(layout.contains(first));
        assert!(!layout.contains(WindowId(99)));
    }

    #[test]
    fn single_window_takes_the_whole_bounds() {
        let (layout, first) = TabLayout::new();
        let bounds = Rect::new(5, 5, 800, 600);
        assert_eq!(layout.rects(bounds, 4), vec![(first, bounds)]);
    }

    #[test]
    fn focus_only_accepts_known_windows() {
        let (mut layout, first) = TabLayout::new();
        assert!(!layout.focus(WindowId(42)));
        assert_eq!(layout.focused(), Some(first));
        assert!(layout.focus(first));
    }
}
