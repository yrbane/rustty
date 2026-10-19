use crate::{Axis, Side, SplitId, TabLayout, WindowId};

#[test]
fn insert_on_each_side_places_the_new_window() {
    for (side, new_first) in [
        (Side::Top, true),
        (Side::Left, true),
        (Side::Bottom, false),
        (Side::Right, false),
    ] {
        let (mut l, a) = TabLayout::new();
        let b = l.insert(a, side).unwrap();
        assert_eq!(l.windows(), if new_first { vec![b, a] } else { vec![a, b] });
        assert_eq!(l.focused(), Some(b));
        assert_eq!(l.split_axis(SplitId(b.0)), Some(side.axis()));
        assert_eq!(l.parent_split(a), Some(SplitId(b.0)));
    }
}

#[test]
fn insert_root_wraps_the_whole_tree() {
    let (mut l, a) = TabLayout::new();
    let b = l.split(a, Axis::Horizontal).unwrap();
    let c = l.insert_root(Side::Right);
    assert_eq!(l.windows(), vec![a, b, c]);
    assert_eq!(l.parent_split(c), Some(SplitId(c.0)));
    assert_eq!(l.parent_split(a), Some(SplitId(b.0)));
}

#[test]
fn insert_root_on_an_empty_tree_creates_the_root() {
    let (mut l, a) = TabLayout::new();
    l.close(a);
    let b = l.insert_root(Side::Left);
    assert_eq!(l.windows(), vec![b]);
    assert_eq!(l.focused(), Some(b));
    assert_eq!(l.parent_split(b), None);
}

#[test]
fn insert_on_missing_target_is_refused_and_root_has_no_parent() {
    let (mut l, a) = TabLayout::new();
    assert_eq!(l.insert(WindowId(99), Side::Left), None);
    assert_eq!(l.parent_split(a), None);
}

#[test]
fn insert_lifts_the_zoom() {
    let (mut l, a) = TabLayout::new();
    let b = l.split(a, Axis::Vertical).unwrap();
    l.toggle_zoom(a);
    l.insert(b, Side::Top).unwrap();
    assert_eq!(l.zoomed(), None);
}

#[test]
fn side_maps_to_axis_and_order() {
    assert_eq!(Side::Top.axis(), Axis::Horizontal);
    assert_eq!(Side::Bottom.axis(), Axis::Horizontal);
    assert_eq!(Side::Left.axis(), Axis::Vertical);
    assert_eq!(Side::Right.axis(), Axis::Vertical);
    assert!(Side::Top.new_is_first() && Side::Left.new_is_first());
    assert!(!Side::Bottom.new_is_first() && !Side::Right.new_is_first());
}
