//! Façade d'un onglet : l'arbre de divisions, la fenêtre focalisée et le zoom.

use crate::geometry::{Axis, Direction, Rect, WindowId};
use crate::node::Node;

#[derive(Clone, Debug, PartialEq)]
pub struct TabLayout {
    root: Option<Node>,
    focused: Option<WindowId>,
    next_id: u64,
    zoomed: Option<WindowId>,
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
                zoomed: None,
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
        self.zoomed = None;
        Some(new_id)
    }

    pub fn zoomed(&self) -> Option<WindowId> {
        self.zoomed
    }

    /// Zoome `id` sur tout l'onglet, ou rétablit le layout si `id` l'était déjà.
    pub fn toggle_zoom(&mut self, id: WindowId) -> bool {
        if !self.contains(id) {
            return false;
        }
        self.zoomed = if self.zoomed == Some(id) {
            None
        } else {
            Some(id)
        };
        true
    }

    pub fn rects(&self, bounds: Rect, gap: u32) -> Vec<(WindowId, Rect)> {
        if let Some(z) = self.zoomed {
            return vec![(z, bounds)];
        }
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.rects(bounds, gap, &mut out);
        }
        out
    }

    /// Côté de la grille virtuelle utilisée pour les calculs de voisinage.
    const VIRTUAL_SIDE: u32 = 10_000;

    /// Fenêtre adjacente à `id` dans `direction`, celle qui partage le plus
    /// long bord ; à égalité la plus haute ou la plus à gauche.
    pub fn neighbor(&self, id: WindowId, direction: Direction) -> Option<WindowId> {
        let rects = self.rects(Rect::new(0, 0, Self::VIRTUAL_SIDE, Self::VIRTUAL_SIDE), 0);
        let (_, from) = *rects.iter().find(|(w, _)| *w == id)?;
        rects
            .iter()
            .filter(|(w, _)| *w != id)
            .filter_map(|(w, r)| {
                let adjacent = match direction {
                    Direction::Left => r.right() == from.x,
                    Direction::Right => r.x == from.right(),
                    Direction::Up => r.bottom() == from.y,
                    Direction::Down => r.y == from.bottom(),
                };
                let overlap = match direction.axis() {
                    Axis::Vertical => overlap_1d(from.y, from.bottom(), r.y, r.bottom()),
                    Axis::Horizontal => overlap_1d(from.x, from.right(), r.x, r.right()),
                };
                (adjacent && overlap > 0).then_some((overlap, std::cmp::Reverse((r.y, r.x)), *w))
            })
            .max_by_key(|(overlap, pos, _)| (*overlap, *pos))
            .map(|(_, _, w)| w)
    }

    /// Agrandit (`delta > 0`) ou réduit le panneau de `id` le long de `axis`.
    pub fn resize(&mut self, id: WindowId, axis: Axis, delta: f32) -> bool {
        // Un delta NaN ou infini corromprait le ratio pour de bon.
        if !delta.is_finite() {
            return false;
        }
        self.root
            .as_mut()
            .is_some_and(|r| r.resize(id, axis, delta))
    }

    /// Inverse l'orientation de la division la plus proche de `id`.
    pub fn rotate(&mut self, id: WindowId) -> bool {
        self.root.as_mut().is_some_and(|r| r.rotate(id))
    }

    /// Ferme `id` ; son panneau frère reprend l'espace. Si `id` avait le focus,
    /// la première feuille du frère promu le reçoit. Vrai si `id` existait.
    pub fn close(&mut self, id: WindowId) -> bool {
        if !self.contains(id) {
            return false;
        }
        if self.zoomed == Some(id) {
            self.zoomed = None;
        }
        let root = self.root.take().expect("contains(id) garantit une racine");
        let sibling_focus = Self::promoted_sibling_first_leaf(&root, id);
        self.root = root.remove(id);
        if self.focused == Some(id) {
            self.focused = sibling_focus;
        }
        true
    }

    /// Première feuille du panneau frère de `id`, c'est-à-dire ce qui prendra
    /// sa place à l'écran. `None` si `id` est la racine.
    fn promoted_sibling_first_leaf(node: &Node, id: WindowId) -> Option<WindowId> {
        match node {
            Node::Leaf(_) => None,
            Node::Split { first, second, .. } => {
                if **first == Node::Leaf(id) {
                    Some(second.first_leaf())
                } else if **second == Node::Leaf(id) {
                    Some(first.first_leaf())
                } else if first.contains(id) {
                    Self::promoted_sibling_first_leaf(first, id)
                } else {
                    Self::promoted_sibling_first_leaf(second, id)
                }
            }
        }
    }
}

/// Longueur du recouvrement de `[a0, a1)` et `[b0, b1)`.
fn overlap_1d(a0: u32, a1: u32, b0: u32, b1: u32) -> u32 {
    a1.min(b1).saturating_sub(a0.max(b0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Axis;
    use crate::geometry::Direction;

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

    #[test]
    fn closing_a_pane_gives_its_space_to_the_sibling() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        assert!(layout.close(b));
        assert_eq!(layout.windows(), vec![a]);
        assert_eq!(layout.focused(), Some(a));
        let bounds = Rect::new(0, 0, 100, 50);
        assert_eq!(layout.rects(bounds, 0), vec![(a, bounds)]);
    }

    #[test]
    fn closing_a_nested_pane_promotes_the_sibling_subtree() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        assert!(layout.close(a));
        assert_eq!(layout.windows(), vec![b, c]);
        let bounds = Rect::new(0, 0, 100, 100);
        assert_eq!(rect_of(&layout, b, bounds, 0), Rect::new(0, 0, 100, 50));
        assert_eq!(rect_of(&layout, c, bounds, 0), Rect::new(0, 50, 100, 50));
    }

    #[test]
    fn closing_the_focused_pane_moves_focus_to_the_promoted_sibling() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        layout.focus(a);
        assert!(layout.close(a));
        assert_eq!(layout.focused(), Some(b), "première feuille du frère promu");
        assert!(layout.close(b));
        assert_eq!(layout.focused(), Some(c));
    }

    #[test]
    fn closing_an_unfocused_pane_keeps_focus() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        assert!(layout.close(a));
        assert_eq!(layout.focused(), Some(b));
    }

    #[test]
    fn closing_the_last_window_empties_the_tab() {
        let (mut layout, a) = TabLayout::new();
        assert!(layout.close(a));
        assert!(layout.is_empty());
        assert_eq!(layout.focused(), None);
        assert!(layout.windows().is_empty());
        assert!(layout.rects(Rect::new(0, 0, 10, 10), 0).is_empty());
        assert!(!layout.close(a), "déjà fermée");
        assert_eq!(layout.split(a, Axis::Vertical), None);
    }

    #[test]
    fn resize_grows_the_pane_containing_the_window() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let bounds = Rect::new(0, 0, 100, 10);
        assert!(layout.resize(a, Axis::Vertical, 0.1));
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 60);
        assert!(layout.resize(b, Axis::Vertical, 0.3));
        assert_eq!(
            rect_of(&layout, a, bounds, 0).width,
            30,
            "agrandir b réduit a"
        );
    }

    #[test]
    fn resize_targets_the_nearest_split_with_that_axis() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        let bounds = Rect::new(0, 0, 100, 100);
        assert!(
            layout.resize(c, Axis::Vertical, 0.2),
            "c n'a pas de division verticale directe : on remonte"
        );
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 30);
        assert_eq!(rect_of(&layout, c, bounds, 0).width, 70);
        assert!(layout.resize(c, Axis::Horizontal, 0.2));
        assert_eq!(rect_of(&layout, c, bounds, 0).height, 70);
        assert_eq!(rect_of(&layout, b, bounds, 0).height, 30);
    }

    #[test]
    fn resize_ignores_non_finite_delta() {
        let (mut layout, a) = TabLayout::new();
        let _ = layout.split(a, Axis::Vertical).unwrap();
        let bounds = Rect::new(0, 0, 100, 10);
        assert!(!layout.resize(a, Axis::Vertical, f32::NAN));
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 50);
        assert!(layout.resize(a, Axis::Vertical, 0.1));
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 60);
    }

    #[test]
    fn resize_is_clamped() {
        let (mut layout, a) = TabLayout::new();
        let _ = layout.split(a, Axis::Vertical).unwrap();
        let bounds = Rect::new(0, 0, 100, 10);
        for _ in 0..20 {
            layout.resize(a, Axis::Vertical, 0.1);
        }
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 90);
        for _ in 0..40 {
            layout.resize(a, Axis::Vertical, -0.1);
        }
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 10);
    }

    #[test]
    fn resize_without_a_matching_split_or_unknown_window_does_nothing() {
        let (mut layout, a) = TabLayout::new();
        assert!(!layout.resize(a, Axis::Vertical, 0.1), "une seule fenêtre");
        let _ = layout.split(a, Axis::Vertical).unwrap();
        assert!(
            !layout.resize(a, Axis::Horizontal, 0.1),
            "pas de division horizontale"
        );
        assert!(!layout.resize(WindowId(9), Axis::Vertical, 0.1));
    }

    #[test]
    fn rotate_flips_the_nearest_split() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        let bounds = Rect::new(0, 0, 100, 100);
        assert!(layout.rotate(c));
        assert_eq!(
            rect_of(&layout, b, bounds, 0),
            Rect::new(50, 0, 25, 100),
            "b et c passent côte à côte"
        );
        assert_eq!(rect_of(&layout, c, bounds, 0), Rect::new(75, 0, 25, 100));
        assert_eq!(
            rect_of(&layout, a, bounds, 0),
            Rect::new(0, 0, 50, 100),
            "la division racine n'a pas bougé"
        );
        let (mut single, s) = TabLayout::new();
        assert!(!single.rotate(s));
    }

    #[test]
    fn neighbor_in_a_simple_split() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        assert_eq!(layout.neighbor(a, Direction::Right), Some(b));
        assert_eq!(layout.neighbor(b, Direction::Left), Some(a));
        assert_eq!(layout.neighbor(a, Direction::Left), None);
        assert_eq!(layout.neighbor(a, Direction::Up), None);
        assert_eq!(layout.neighbor(a, Direction::Down), None);
    }

    #[test]
    fn neighbor_picks_the_pane_sharing_the_longest_edge() {
        // a | b   avec b découpé en b (haut, 70 %) et c (bas, 30 %), puis a
        //   | c   découpé en a (haut) et d (bas) à 50 % : d touche b et c.
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        layout.resize(b, Axis::Horizontal, 0.2);
        let d = layout.split(a, Axis::Horizontal).unwrap();
        assert_eq!(layout.neighbor(a, Direction::Right), Some(b));
        assert_eq!(
            layout.neighbor(d, Direction::Right),
            Some(c),
            "d (50-100 %) recouvre c (70-100 %) sur 30 % et b sur 20 %"
        );
        assert_eq!(layout.neighbor(c, Direction::Left), Some(d));
        assert_eq!(
            layout.neighbor(b, Direction::Left),
            Some(a),
            "b (0-70 %) recouvre a sur 50 % et d sur 20 %"
        );
        assert_eq!(layout.neighbor(b, Direction::Down), Some(c));
        assert_eq!(layout.neighbor(c, Direction::Up), Some(b));
    }

    #[test]
    fn neighbor_ties_go_to_the_topmost_or_leftmost() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        assert_eq!(
            layout.neighbor(a, Direction::Right),
            Some(b),
            "b et c partagent 50 % chacun : le plus haut gagne"
        );
        let _ = c;
    }

    #[test]
    fn neighbor_of_unknown_or_single_window_is_none() {
        let (layout, a) = TabLayout::new();
        assert_eq!(layout.neighbor(a, Direction::Right), None);
        assert_eq!(layout.neighbor(WindowId(5), Direction::Right), None);
    }

    #[test]
    fn zoom_shows_only_one_window_full_size_and_toggles_back() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let bounds = Rect::new(0, 0, 100, 50);
        assert!(layout.toggle_zoom(a));
        assert_eq!(layout.zoomed(), Some(a));
        assert_eq!(layout.rects(bounds, 4), vec![(a, bounds)]);
        assert_eq!(
            layout.windows(),
            vec![a, b],
            "les fenêtres existent toujours"
        );
        assert!(layout.toggle_zoom(a));
        assert_eq!(layout.zoomed(), None);
        assert_eq!(layout.rects(bounds, 4).len(), 2);
    }

    #[test]
    fn zooming_another_window_moves_the_zoom() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        layout.toggle_zoom(a);
        assert!(layout.toggle_zoom(b));
        assert_eq!(layout.zoomed(), Some(b));
        assert!(!layout.toggle_zoom(WindowId(9)));
    }

    #[test]
    fn split_and_close_cancel_the_zoom() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        layout.toggle_zoom(a);
        let _ = layout.split(a, Axis::Horizontal).unwrap();
        assert_eq!(layout.zoomed(), None, "diviser rompt le zoom");
        layout.toggle_zoom(b);
        layout.close(b);
        assert_eq!(
            layout.zoomed(),
            None,
            "fermer la fenêtre zoomée rompt le zoom"
        );
        layout.toggle_zoom(a);
        layout.close(layout.windows()[1]);
        assert_eq!(
            layout.zoomed(),
            Some(a),
            "fermer une autre fenêtre conserve le zoom"
        );
    }
}
