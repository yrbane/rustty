//! Façade d'un onglet : l'arbre de divisions, la fenêtre focalisée et le zoom.

use crate::geometry::{Axis, Rect, WindowId};
use crate::node::Node;

#[derive(Clone, Debug, PartialEq)]
pub struct TabLayout {
    root: Option<Node>,
    focused: Option<WindowId>,
    next_id: u64,
}

impl TabLayout {
    /// Un onglet naît avec une fenêtre, focalisée.
    pub fn new() -> (Self, WindowId) {
        let first = WindowId(1);
        (
            Self {
                root: Some(Node::Leaf(first)),
                focused: Some(first),
                next_id: 2,
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

    /// Divise `target` selon `axis` ; la nouvelle fenêtre est le second
    /// panneau (droite ou bas) et prend le focus.
    pub fn split(&mut self, target: WindowId, axis: Axis) -> Option<WindowId> {
        let root = self.root.as_mut()?;
        let new_id = WindowId(self.next_id);
        if !root.split_leaf(target, axis, new_id) {
            return None;
        }
        self.next_id += 1;
        self.focused = Some(new_id);
        Some(new_id)
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
    use crate::geometry::Axis;

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

    fn rect_of(layout: &TabLayout, id: WindowId, bounds: Rect, gap: u32) -> Rect {
        layout
            .rects(bounds, gap)
            .into_iter()
            .find(|(w, _)| *w == id)
            .map(|(_, r)| r)
            .unwrap()
    }

    #[test]
    fn vertical_split_puts_the_new_window_on_the_right_and_focuses_it() {
        let (mut layout, first) = TabLayout::new();
        let second = layout.split(first, Axis::Vertical).unwrap();
        assert_eq!(second, WindowId(2));
        assert_eq!(layout.windows(), vec![first, second]);
        assert_eq!(layout.focused(), Some(second));
        let bounds = Rect::new(0, 0, 100, 50);
        assert_eq!(rect_of(&layout, first, bounds, 0), Rect::new(0, 0, 50, 50));
        assert_eq!(
            rect_of(&layout, second, bounds, 0),
            Rect::new(50, 0, 50, 50)
        );
    }

    #[test]
    fn horizontal_split_stacks_the_new_window_below() {
        let (mut layout, first) = TabLayout::new();
        let second = layout.split(first, Axis::Horizontal).unwrap();
        let bounds = Rect::new(0, 0, 100, 50);
        assert_eq!(rect_of(&layout, first, bounds, 0), Rect::new(0, 0, 100, 25));
        assert_eq!(
            rect_of(&layout, second, bounds, 0),
            Rect::new(0, 25, 100, 25)
        );
    }

    #[test]
    fn gap_is_taken_between_panes_and_odd_pixels_go_to_the_second() {
        let (mut layout, first) = TabLayout::new();
        let second = layout.split(first, Axis::Vertical).unwrap();
        let bounds = Rect::new(10, 10, 101, 40);
        assert_eq!(
            rect_of(&layout, first, bounds, 4),
            Rect::new(10, 10, 48, 40),
            "(101-4)*0.5 = 48.5 tronqué à 48, l'impair va au second"
        );
        assert_eq!(
            rect_of(&layout, second, bounds, 4),
            Rect::new(62, 10, 49, 40)
        );
    }

    #[test]
    fn nested_splits_divide_the_target_only() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        assert_eq!(layout.windows(), vec![a, b, c]);
        let bounds = Rect::new(0, 0, 100, 100);
        assert_eq!(rect_of(&layout, a, bounds, 0), Rect::new(0, 0, 50, 100));
        assert_eq!(rect_of(&layout, b, bounds, 0), Rect::new(50, 0, 50, 50));
        assert_eq!(rect_of(&layout, c, bounds, 0), Rect::new(50, 50, 50, 50));
    }

    #[test]
    fn splitting_an_unknown_window_does_nothing() {
        let (mut layout, first) = TabLayout::new();
        assert_eq!(layout.split(WindowId(7), Axis::Vertical), None);
        assert_eq!(layout.windows(), vec![first]);
    }

    #[test]
    fn rects_survive_degenerate_bounds() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let _ = layout.split(b, Axis::Horizontal).unwrap();
        for bounds in [
            Rect::new(0, 0, 1, 1),
            Rect::new(0, 0, 3, 3),
            Rect::new(0, 0, 0, 0),
        ] {
            let rects = layout.rects(bounds, 4);
            assert_eq!(rects.len(), 3, "{bounds:?}");
            for (_, r) in &rects {
                assert!(bounds.contains_rect(r), "{r:?} déborde de {bounds:?}");
            }
        }
    }

    #[test]
    fn focus_only_accepts_known_windows() {
        let (mut layout, first) = TabLayout::new();
        assert!(!layout.focus(WindowId(42)));
        assert_eq!(layout.focused(), Some(first));
        assert!(layout.focus(first));
    }
}
