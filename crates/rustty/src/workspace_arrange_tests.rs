//! Tests des déplacements de panneaux et d'onglets.

use crate::tab::TermId;
use crate::workspace::Workspace;
use rustty_config::SplitAxis;
use rustty_layout::{Axis, Side};

#[test]
fn move_within_a_tab_keeps_every_term() {
    let (mut ws, t1) = Workspace::new();
    let t2 = ws.split_focused(SplitAxis::Vertical).unwrap();
    assert!(ws.move_term_to_pane(t1, t2, Side::Bottom));
    assert_eq!(ws.active_tab().terms(), vec![t2, t1]);
    assert_eq!(ws.focused_term(), Some(t1));
}

#[test]
fn move_to_another_tab_activates_it() {
    let (mut ws, t1) = Workspace::new();
    let t2 = ws.split_focused(SplitAxis::Vertical).unwrap();
    let t3 = ws.new_tab();
    assert!(ws.move_term_to_pane(t2, t3, Side::Right));
    assert_eq!(ws.tabs()[0].terms(), vec![t1]);
    assert_eq!(ws.tabs()[1].terms(), vec![t3, t2]);
    assert_eq!(ws.active(), 1);
    assert_eq!(ws.focused_term(), Some(t2));
}

#[test]
fn emptied_source_tab_is_removed_and_indices_follow() {
    let (mut ws, a) = Workspace::new();
    let b = ws.new_tab();
    let c = ws.split_focused(SplitAxis::Vertical).unwrap();
    assert!(ws.move_term_to_pane(a, c, Side::Left));
    assert_eq!(ws.tabs().len(), 1);
    assert_eq!(ws.active(), 0);
    assert_eq!(ws.active_tab().terms(), vec![b, a, c]);
    assert_eq!(ws.tab_of(a), Some(0));
}

#[test]
fn move_onto_itself_is_refused() {
    let (mut ws, t1) = Workspace::new();
    ws.split_focused(SplitAxis::Vertical).unwrap();
    assert!(!ws.move_term_to_pane(t1, t1, Side::Left));
    assert!(!ws.move_term_to_pane(t1, TermId(99), Side::Left));
    assert!(!ws.move_term_to_pane(TermId(99), t1, Side::Left));
    assert_eq!(ws.active_tab().terms().len(), 2);
}

#[test]
fn move_to_own_tab_goes_to_the_right_edge() {
    let (mut ws, t1) = Workspace::new();
    let t2 = ws.split_focused(SplitAxis::Horizontal).unwrap();
    assert!(ws.move_term_to_tab(t1, 0));
    assert_eq!(ws.active_tab().terms(), vec![t2, t1]);
    let root = ws.active_tab().layout.windows()[1];
    let split = ws.active_tab().parent_split_of(t1).unwrap();
    assert_eq!(
        split.0, root.0,
        "la division racine porte l'id du nouveau panneau"
    );
    assert_eq!(Side::Right.axis(), Axis::Vertical);
}

#[test]
fn move_to_tab_refuses_a_lone_term_in_its_own_tab_and_bad_indices() {
    let (mut ws, t1) = Workspace::new();
    assert!(!ws.move_term_to_tab(t1, 0));
    assert!(!ws.move_term_to_tab(t1, 5));
    assert!(!ws.move_term_to_new_tab(t1));
    assert_eq!(ws.tabs().len(), 1);
}

#[test]
fn move_to_new_tab_inserts_after_the_source() {
    let (mut ws, a) = Workspace::new();
    let b = ws.split_focused(SplitAxis::Vertical).unwrap();
    let c = ws.new_tab();
    ws.go_to_tab(1);
    assert!(ws.move_term_to_new_tab(b));
    assert_eq!(ws.tabs().len(), 3);
    assert_eq!(ws.tabs()[0].terms(), vec![a]);
    assert_eq!(ws.tabs()[1].terms(), vec![b]);
    assert_eq!(ws.tabs()[2].terms(), vec![c]);
    assert_eq!(ws.active(), 1);
    assert_eq!(ws.focused_term(), Some(b));
}

#[test]
fn move_tab_reorders_and_active_follows() {
    let (mut ws, a) = Workspace::new();
    let b = ws.new_tab();
    let c = ws.new_tab();
    ws.focus_term(b);
    assert!(ws.move_tab(0, 3));
    let order: Vec<_> = ws.tabs().iter().flat_map(|t| t.terms()).collect();
    assert_eq!(order, vec![b, c, a]);
    assert_eq!(ws.active(), 0);
    assert!(ws.move_tab(2, 0));
    let order: Vec<_> = ws.tabs().iter().flat_map(|t| t.terms()).collect();
    assert_eq!(order, vec![a, b, c]);
    assert_eq!(ws.active(), 1);
    assert!(!ws.move_tab(1, 1));
    assert!(!ws.move_tab(1, 2));
    assert!(!ws.move_tab(5, 0));
    assert!(!ws.move_tab(0, 4));
}

#[test]
fn close_other_tabs_returns_their_terms() {
    let (mut ws, a) = Workspace::new();
    let b = ws.new_tab();
    let c = ws.split_focused(SplitAxis::Vertical).unwrap();
    let d = ws.new_tab();
    ws.rename_pane(a, Some("x".into()));
    let mut killed = ws.close_other_tabs(1);
    killed.sort();
    assert_eq!(killed, vec![a, d]);
    assert_eq!(ws.tabs().len(), 1);
    assert_eq!(ws.active(), 0);
    assert_eq!(ws.active_tab().terms(), vec![b, c]);
    assert_eq!(ws.pane_name(a), None);
    assert!(ws.close_other_tabs(7).is_empty());
}

#[test]
fn pane_name_follows_the_term_and_is_dropped_on_close() {
    let (mut ws, _t1) = Workspace::new();
    let t2 = ws.split_focused(SplitAxis::Vertical).unwrap();
    ws.new_tab();
    ws.rename_pane(t2, Some("logs".into()));
    assert!(ws.move_term_to_tab(t2, 1));
    assert_eq!(ws.pane_name(t2), Some("logs"));
    ws.close_term(t2);
    assert_eq!(ws.pane_name(t2), None);
    let (mut ws, t) = Workspace::new();
    ws.rename_pane(t, Some("a".into()));
    ws.rename_pane(t, Some(String::new()));
    assert_eq!(ws.pane_name(t), None);
}

#[test]
fn pane_name_is_dropped_on_close_tab() {
    let (mut ws, a) = Workspace::new();
    ws.new_tab();
    ws.rename_pane(a, Some("n".into()));
    ws.close_tab(0);
    assert_eq!(ws.pane_name(a), None);
}

#[test]
fn tab_color_is_set_and_cleared() {
    let (mut ws, _) = Workspace::new();
    ws.set_tab_color(0, Some(4));
    assert_eq!(ws.tabs()[0].color, Some(4));
    ws.set_tab_color(0, None);
    assert_eq!(ws.tabs()[0].color, None);
    ws.set_tab_color(9, Some(1));
}
