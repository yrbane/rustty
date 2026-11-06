use super::*;
use crate::mouse::MouseButton;
use rustty_config::{TitleBar, TitleButton};
use rustty_render::{TitleBarStyle, TitleTarget, layout_title_bar};

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
    assert_eq!(
        button_effects(TitleButton::Settings),
        vec![Effect::OpenConfig]
    );
    assert_eq!(button_effects(TitleButton::Folder), vec![Effect::OpenCwd]);
    assert!(button_effects(TitleButton::Color).is_empty(), "jalon 3");
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
    assert_eq!(d.moved(12.0, 11.0, MOVE_THRESHOLD), TitleDragEvent::None);
    assert_eq!(d.release(), TitleDragEvent::Click);
    assert_eq!(d, TitleDrag::Idle);
    d.press(MouseButton::Left, 10.0, 10.0);
    assert_eq!(
        d.moved(14.0, 10.0, MOVE_THRESHOLD),
        TitleDragEvent::StartMove,
        "4 px exactement"
    );
    assert_eq!(
        d,
        TitleDrag::Idle,
        "le système tient le déplacement, le relâchement ne reviendra pas"
    );
    assert_eq!(
        d.moved(30.0, 10.0, MOVE_THRESHOLD),
        TitleDragEvent::None,
        "déjà en mouvement"
    );
    assert_eq!(d.release(), TitleDragEvent::None);
    assert_eq!(d, TitleDrag::Idle);
}

#[test]
fn a_click_after_a_move_is_not_swallowed() {
    let mut d = TitleDrag::Idle;
    d.press(MouseButton::Left, 10.0, 10.0);
    assert_eq!(
        d.moved(40.0, 10.0, MOVE_THRESHOLD),
        TitleDragEvent::StartMove
    );
    // Le relâchement du déplacement n'arrive jamais ; l'appui suivant
    // (hors de la barre) ne doit trouver aucun état en cours.
    assert_eq!(d, TitleDrag::Idle);
}

#[test]
fn motion_and_release_while_idle_do_nothing() {
    let mut d = TitleDrag::Idle;
    assert_eq!(d.moved(100.0, 100.0, MOVE_THRESHOLD), TitleDragEvent::None);
    assert_eq!(d.release(), TitleDragEvent::None);
    assert_eq!(d, TitleDrag::Idle);
}

#[test]
fn the_threshold_scales_with_the_screen() {
    // Écran à 200 % : 4 px logiques font 8 px physiques.
    let mut d = TitleDrag::Idle;
    d.press(MouseButton::Left, 10.0, 10.0);
    assert_eq!(
        d.moved(14.0, 10.0, MOVE_THRESHOLD * 2.0),
        TitleDragEvent::None
    );
    assert_eq!(
        d.moved(18.0, 10.0, MOVE_THRESHOLD * 2.0),
        TitleDragEvent::StartMove
    );
}

#[test]
fn a_point_outside_the_window_is_not_in_the_bar() {
    assert_eq!(bar_point(-1.0, 5.0), None, "à gauche de la fenêtre");
    assert_eq!(bar_point(5.0, -1.0), None, "au-dessus de la fenêtre");
    assert_eq!(bar_point(0.0, 0.0), Some((0, 0)));
    assert_eq!(bar_point(12.7, 3.2), Some((12, 3)));
}

#[test]
fn only_a_click_on_the_title_counts_towards_a_rename() {
    assert!(title_click_renames(
        TitleDragEvent::Click,
        TitleTarget::Title
    ));
    assert!(!title_click_renames(
        TitleDragEvent::Click,
        TitleTarget::Background
    ));
    assert!(!title_click_renames(
        TitleDragEvent::None,
        TitleTarget::Title
    ));
    assert!(!title_click_renames(
        TitleDragEvent::StartMove,
        TitleTarget::Title
    ));
}

#[test]
fn the_palette_hangs_under_the_color_button_or_the_title() {
    let style = TitleBarStyle::from_config(&TitleBar::default(), 1.0);
    let layout = layout_title_bar(400, &style);
    let (_, rect) = layout
        .buttons
        .iter()
        .find(|(b, _)| *b == TitleButton::Color)
        .unwrap();
    assert_eq!(palette_anchor(&layout), (rect.x, rect.y + rect.height));
    let mut cfg = TitleBar::default();
    cfg.buttons.color.visible = false;
    let hidden = layout_title_bar(400, &TitleBarStyle::from_config(&cfg, 1.0));
    assert_eq!(palette_anchor(&hidden), (hidden.title.x, hidden.bar.height));
}
