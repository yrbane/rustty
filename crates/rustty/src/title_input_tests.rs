use super::*;
use winit::event::MouseButton;

#[test]
fn only_the_left_button_arms_a_move() {
    let mut d = TitleDrag::Idle;
    d.press(MouseButton::Right, 10.0, 10.0);
    assert_eq!(d, TitleDrag::Idle);
    d.press(MouseButton::Left, 10.0, 10.0);
    assert_eq!(d, TitleDrag::Armed { x: 10.0, y: 10.0 });
}

#[test]
fn small_motion_is_a_click_large_motion_is_a_move() {
    let mut d = TitleDrag::Idle;
    d.press(MouseButton::Left, 10.0, 10.0);
    assert_eq!(d.moved(12.0, 11.0), TitleDragEvent::None);
    assert_eq!(d.release(), TitleDragEvent::Click);
    assert_eq!(d, TitleDrag::Idle);
    d.press(MouseButton::Left, 10.0, 10.0);
    assert_eq!(
        d.moved(14.0, 10.0),
        TitleDragEvent::StartMove,
        "4 px exactement"
    );
    assert_eq!(
        d.moved(30.0, 10.0),
        TitleDragEvent::None,
        "déjà en mouvement"
    );
    assert_eq!(d.release(), TitleDragEvent::None);
    assert_eq!(d, TitleDrag::Idle);
}

#[test]
fn motion_and_release_while_idle_do_nothing() {
    let mut d = TitleDrag::Idle;
    assert_eq!(d.moved(100.0, 100.0), TitleDragEvent::None);
    assert_eq!(d.release(), TitleDragEvent::None);
    assert_eq!(d, TitleDrag::Idle);
}
