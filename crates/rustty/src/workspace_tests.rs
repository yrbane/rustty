//! Tests de l'espace de travail.

use super::*;
use rustty_layout::SplitId;

#[test]
fn tabs_and_splits_get_accents() {
    let (mut ws, _) = Workspace::with_seed(3);
    let first = ws.tabs()[0].accent;
    ws.new_tab();
    assert_ne!(
        ws.tabs()[1].accent,
        first,
        "deux onglets consécutifs diffèrent"
    );
    let t = ws.split_focused(SplitAxis::Vertical).unwrap();
    let tab = ws.active_tab();
    let window = tab.window_of(t).unwrap();
    let accent = tab.split_accent(SplitId(window.0));
    assert!(accent < crate::accent::ACCENT_SLOTS.len());
}

#[test]
fn rename_sets_and_clears_the_custom_title() {
    let (mut ws, _) = Workspace::with_seed(1);
    ws.rename(0, Some("logs".into()));
    assert_eq!(ws.tabs()[0].custom_title.as_deref(), Some("logs"));
    ws.rename(0, None);
    assert_eq!(ws.tabs()[0].custom_title, None);
    ws.rename(9, Some("hors limites".into()));
}

#[test]
fn starts_with_one_tab_and_one_term() {
    let (ws, first) = Workspace::new();
    assert_eq!(first, TermId(1));
    assert_eq!(ws.tabs().len(), 1);
    assert_eq!(ws.active(), 0);
    assert_eq!(ws.focused_term(), Some(TermId(1)));
}

#[test]
fn new_tab_is_inserted_after_the_active_one_and_activated() {
    let (mut ws, _) = Workspace::new();
    let t2 = ws.new_tab();
    assert_eq!((t2, ws.active()), (TermId(2), 1));
    ws.go_to_tab(1);
    let t3 = ws.new_tab();
    assert_eq!(
        (t3, ws.active()),
        (TermId(3), 1),
        "inséré après l'onglet 1, avant l'ancien onglet 2"
    );
    assert_eq!(ws.tabs()[2].focused_term(), Some(TermId(2)));
}

#[test]
fn next_prev_and_go_to_wrap_and_clamp() {
    let (mut ws, _) = Workspace::new();
    ws.new_tab();
    ws.new_tab();
    ws.go_to_tab(1);
    ws.prev_tab();
    assert_eq!(ws.active(), 2, "circulaire vers la fin");
    ws.next_tab();
    assert_eq!(ws.active(), 0);
    ws.go_to_tab(9);
    assert_eq!(ws.active(), 0, "numéro hors limites ignoré");
    ws.go_to_tab(0);
    assert_eq!(ws.active(), 0);
}

#[test]
fn closing_a_tab_returns_its_terms_and_moves_the_active_index() {
    let (mut ws, t1) = Workspace::new();
    let t2 = ws.new_tab();
    let t3 = ws.split_focused(SplitAxis::Vertical).unwrap();
    assert_eq!(ws.tab_of(t3), Some(1));
    let killed = ws.close_tab(1);
    assert_eq!(killed.len(), 2);
    assert!(killed.contains(&t2) && killed.contains(&t3));
    assert_eq!((ws.tabs().len(), ws.active()), (1, 0));
    assert_eq!(ws.focused_term(), Some(t1));
    assert!(ws.close_tab(5).is_empty(), "index invalide");
    assert_eq!(ws.close_tab(0), vec![t1]);
    assert!(ws.is_empty());
    assert_eq!(ws.focused_term(), None);
}

#[test]
fn closing_the_first_tab_keeps_the_next_one_active() {
    let (mut ws, _) = Workspace::new();
    ws.new_tab();
    ws.go_to_tab(1);
    ws.close_tab(0);
    assert_eq!((ws.tabs().len(), ws.active()), (1, 0));
}

#[test]
fn closing_terms_removes_empty_tabs() {
    let (mut ws, t1) = Workspace::new();
    let t2 = ws.split_focused(SplitAxis::Horizontal).unwrap();
    assert!(ws.close_term(t2));
    assert_eq!(ws.tabs().len(), 1);
    assert!(!ws.close_term(TermId(42)));
    assert!(ws.close_term(t1));
    assert!(ws.is_empty());
}

#[test]
fn focus_resize_rotate_zoom_delegate_to_the_layout() {
    let (mut ws, t1) = Workspace::new();
    let t2 = ws.split_focused(SplitAxis::Vertical).unwrap();
    assert_eq!(ws.focused_term(), Some(t2));
    assert!(ws.focus(FocusDirection::Left));
    assert_eq!(ws.focused_term(), Some(t1));
    assert!(!ws.focus(FocusDirection::Up), "pas de voisin au-dessus");
    assert!(ws.resize(ResizeDir::Wider));
    assert!(ws.rotate());
    assert!(ws.toggle_zoom());
    assert_eq!(
        ws.active_tab().layout.zoomed(),
        ws.active_tab().layout.focused()
    );
    assert!(ws.toggle_zoom());
    assert!(ws.focus_term(t2));
    assert_eq!(ws.focused_term(), Some(t2));
}

#[test]
fn focus_term_switches_tabs() {
    let (mut ws, t1) = Workspace::new();
    ws.new_tab();
    assert!(ws.focus_term(t1));
    assert_eq!(ws.active(), 0);
    assert!(!ws.focus_term(TermId(99)));
}
