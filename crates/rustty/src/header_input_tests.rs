use super::*;

const T: TermId = TermId(3);

#[test]
fn close_button_closes_title_focuses_double_click_renames() {
    let close = HeaderHit::Close(T);
    let title = HeaderHit::Title(T);
    assert_eq!(
        header_action(close, MouseButton::Left, false),
        HeaderAction::Close(T)
    );
    assert_eq!(
        header_action(title, MouseButton::Left, false),
        HeaderAction::Focus(T)
    );
    assert_eq!(
        header_action(title, MouseButton::Left, true),
        HeaderAction::Rename(T)
    );
    assert_eq!(
        header_action(title, MouseButton::Right, false),
        HeaderAction::Menu(T)
    );
}

#[test]
fn header_press_is_not_reported_to_the_term() {
    // Toute pression a une issue propre au bandeau : aucune variante
    // « transmettre au terminal » n'existe, quel que soit le bouton.
    for hit in [HeaderHit::Title(T), HeaderHit::Close(T)] {
        for button in [MouseButton::Left, MouseButton::Middle, MouseButton::Right] {
            for double in [false, true] {
                let _ = header_action(hit, button, double);
            }
        }
    }
    assert_eq!(
        header_action(HeaderHit::Title(T), MouseButton::Middle, false),
        HeaderAction::Ignore
    );
}
