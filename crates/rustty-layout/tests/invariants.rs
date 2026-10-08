//! Propriétés géométriques : quelle que soit la suite d'opérations, les
//! rectangles couvrent les bornes sans se chevaucher et chaque fenêtre en a un.

use proptest::prelude::*;
use rustty_layout::{Axis, Direction, Rect, TabLayout, WindowId};

#[derive(Clone, Debug)]
enum Op {
    Split(usize, Axis),
    Close(usize),
    Resize(usize, Axis, f32),
    Rotate(usize),
    Zoom(usize),
    Focus(usize, Direction),
}

fn op() -> impl Strategy<Value = Op> {
    let axis = prop_oneof![Just(Axis::Horizontal), Just(Axis::Vertical)];
    let dir = prop_oneof![
        Just(Direction::Left),
        Just(Direction::Right),
        Just(Direction::Up),
        Just(Direction::Down)
    ];
    prop_oneof![
        (0..8usize, axis.clone()).prop_map(|(i, a)| Op::Split(i, a)),
        (0..8usize).prop_map(Op::Close),
        (0..8usize, axis, -0.5f32..0.5).prop_map(|(i, a, d)| Op::Resize(i, a, d)),
        (0..8usize).prop_map(Op::Rotate),
        (0..8usize).prop_map(Op::Zoom),
        (0..8usize, dir).prop_map(|(i, d)| Op::Focus(i, d)),
    ]
}

/// Choisit une fenêtre existante à partir d'un index arbitraire.
fn pick(layout: &TabLayout, i: usize) -> Option<WindowId> {
    let windows = layout.windows();
    (!windows.is_empty()).then(|| windows[i % windows.len()])
}

fn apply(layout: &mut TabLayout, op: &Op) {
    match *op {
        Op::Split(i, axis) => {
            if let Some(w) = pick(layout, i) {
                let _ = layout.split(w, axis);
            }
        }
        Op::Close(i) => {
            if let Some(w) = pick(layout, i) {
                let _ = layout.close(w);
            }
        }
        Op::Resize(i, axis, delta) => {
            if let Some(w) = pick(layout, i) {
                let _ = layout.resize(w, axis, delta);
            }
        }
        Op::Rotate(i) => {
            if let Some(w) = pick(layout, i) {
                let _ = layout.rotate(w);
            }
        }
        Op::Zoom(i) => {
            if let Some(w) = pick(layout, i) {
                let _ = layout.toggle_zoom(w);
            }
        }
        Op::Focus(i, dir) => {
            if let Some(w) = pick(layout, i)
                && let Some(n) = layout.neighbor(w, dir)
            {
                let _ = layout.focus(n);
            }
        }
    }
}

proptest! {
    #[test]
    fn rects_cover_bounds_without_overlap(ops in prop::collection::vec(op(), 0..40), w in 0u32..300, h in 0u32..300, gap in 0u32..6) {
        let (mut layout, _) = TabLayout::new();
        for o in &ops {
            apply(&mut layout, o);
        }
        let bounds = Rect::new(7, 11, w, h);
        let rects = layout.rects(bounds, gap);
        let visible: Vec<WindowId> = match layout.zoomed() {
            Some(z) => vec![z],
            None => layout.windows(),
        };
        prop_assert_eq!(rects.iter().map(|(id, _)| *id).collect::<Vec<_>>(), visible, "une entrée par fenêtre visible, dans l'ordre");
        for (i, (_, a)) in rects.iter().enumerate() {
            prop_assert!(bounds.contains_rect(a), "{:?} déborde de {:?}", a, bounds);
            for (_, b) in &rects[i + 1..] {
                prop_assert!(!a.intersects(b), "{:?} chevauche {:?}", a, b);
            }
        }
        // Un onglet vidé (dernière fenêtre fermée) n'a plus rien à couvrir.
        if gap == 0 && !layout.is_empty() {
            let total: u64 = rects.iter().map(|(_, r)| r.area()).sum();
            prop_assert_eq!(total, bounds.area(), "sans espace, les aires se somment exactement");
        }
        if let Some(f) = layout.focused() {
            prop_assert!(layout.contains(f), "le focus pointe toujours une fenêtre existante");
        } else {
            prop_assert!(layout.is_empty());
        }
    }
}
