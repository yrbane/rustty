use super::*;
use winit::window::{CursorIcon, ResizeDirection as D};

#[test]
fn edges_and_corners() {
    let f = |x, y| resize_direction(x, y, 800, 600, 6, false);
    assert_eq!(f(2.0, 300.0), Some(D::West));
    assert_eq!(f(797.0, 300.0), Some(D::East));
    assert_eq!(f(400.0, 1.0), Some(D::North));
    assert_eq!(f(400.0, 599.0), Some(D::South));
    assert_eq!(f(3.0, 3.0), Some(D::NorthWest));
    assert_eq!(f(796.0, 597.0), Some(D::SouthEast));
    assert_eq!(f(796.0, 2.0), Some(D::NorthEast));
    assert_eq!(f(2.0, 597.0), Some(D::SouthWest));
    assert_eq!(f(6.0, 300.0), None, "juste après la marge");
    assert_eq!(f(400.0, 300.0), None);
}

#[test]
fn no_edge_when_maximized_or_border_zero() {
    assert_eq!(resize_direction(1.0, 1.0, 800, 600, 6, true), None);
    assert_eq!(resize_direction(1.0, 1.0, 800, 600, 0, false), None);
}

#[test]
fn cursor_matches_direction() {
    assert_eq!(resize_cursor(D::West), CursorIcon::EwResize);
    assert_eq!(resize_cursor(D::East), CursorIcon::EwResize);
    assert_eq!(resize_cursor(D::North), CursorIcon::NsResize);
    assert_eq!(resize_cursor(D::South), CursorIcon::NsResize);
    assert_eq!(resize_cursor(D::NorthWest), CursorIcon::NwseResize);
    assert_eq!(resize_cursor(D::SouthEast), CursorIcon::NwseResize);
    assert_eq!(resize_cursor(D::NorthEast), CursorIcon::NeswResize);
    assert_eq!(resize_cursor(D::SouthWest), CursorIcon::NeswResize);
}
