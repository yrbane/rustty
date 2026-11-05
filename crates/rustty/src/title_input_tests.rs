use super::*;
use crate::mouse::MouseButton;
use rustty_config::TitleButton;

#[test]
fn maximize_button_restores_when_maximized() {
    // L'effet est le même ; c'est effects.rs qui lit is_maximized() : on vérifie le mappage.
    assert_eq!(
        button_effects(TitleButton::Maximize),
        vec![Effect::ToggleMaximize]
    );
    assert_eq!(
        button_effects(TitleButton::Minimize),
        vec![Effect::Minimize]
    );
    assert!(button_effects(TitleButton::Settings).is_empty(), "jalon 2");
    assert!(button_effects(TitleButton::Color).is_empty(), "jalon 2");
    assert!(button_effects(TitleButton::Folder).is_empty(), "jalon 2");
    assert!(
        button_effects(TitleButton::Close).is_empty(),
        "fermer passe par request_close_window"
    );
}

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
