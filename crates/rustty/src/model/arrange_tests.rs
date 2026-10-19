//! Tests de l'exécution des dépôts de panneaux et d'onglets.

use rustty_config::SplitAxis;
use rustty_layout::Side;

use super::super::*;
use crate::drag::PaneDrop;
use crate::rename::{Rename, RenameTarget};

fn idle(_: TermId) -> bool {
    false
}

/// Deux onglets : le premier porte deux panneaux (1, 2), le second le panneau 3.
fn model() -> Model {
    let (mut m, _) = Model::new(0.9, false);
    m.workspace.split_focused(SplitAxis::Vertical).unwrap();
    m.apply(Action::NewTab, false, &idle);
    m.workspace.go_to_tab(0);
    m
}

fn spawns_or_closes(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(|e| matches!(e, Effect::SpawnTerm { .. } | Effect::CloseTerm(_)))
}

#[test]
fn pane_drop_moves_without_spawning() {
    let mut m = model();
    let moved = m.workspace.tabs()[0].terms()[1];
    let target = m.workspace.tabs()[1].terms()[0];
    let effects = m.move_pane(
        moved,
        PaneDrop::Pane {
            target,
            side: Side::Left,
        },
    );
    assert_eq!(effects, vec![Effect::Relayout]);
    assert!(!spawns_or_closes(&effects));
    assert_eq!(m.workspace.tab_of(moved), Some(1));

    let effects = m.move_pane(moved, PaneDrop::Tab(0));
    assert_eq!(effects, vec![Effect::Relayout]);
    assert_eq!(m.workspace.tab_of(moved), Some(0));
}

#[test]
fn pane_to_new_tab_spawns_nothing() {
    let mut m = model();
    let moved = m.workspace.tabs()[0].terms()[1];
    let effects = m.move_pane(moved, PaneDrop::NewTab);
    assert_eq!(effects, vec![Effect::Relayout]);
    assert_eq!(m.workspace.tabs().len(), 3);
}

#[test]
fn drop_on_itself_changes_nothing() {
    let mut m = model();
    let term = m.workspace.tabs()[0].terms()[0];
    let effects = m.move_pane(
        term,
        PaneDrop::Pane {
            target: term,
            side: Side::Top,
        },
    );
    assert!(effects.is_empty());
    assert!(m.move_pane(TermId(99), PaneDrop::NewTab).is_empty());
    let lone = m.workspace.tabs()[1].terms()[0];
    assert!(m.move_pane(lone, PaneDrop::NewTab).is_empty());
}

#[test]
fn drag_refused_while_confirming_or_renaming() {
    let mut m = model();
    assert!(m.accepts_drag());
    m.renaming = Some(Rename::new(RenameTarget::Tab(0), "x"));
    assert!(!m.accepts_drag());
    m.renaming = None;
    m.pending_close = Some(CloseRequest::Tab(0));
    assert!(!m.accepts_drag());
}

#[test]
fn drag_of_a_vanished_term_is_cancelled() {
    let mut m = model();
    let target = m.workspace.tabs()[0].terms()[0];
    let effects = m.move_pane(
        TermId(999),
        PaneDrop::Pane {
            target,
            side: Side::Right,
        },
    );
    assert!(effects.is_empty());
    let moved = m.workspace.tabs()[0].terms()[1];
    let gone = TermId(999);
    let effects = m.move_pane(
        moved,
        PaneDrop::Pane {
            target: gone,
            side: Side::Right,
        },
    );
    assert!(effects.is_empty());
    assert_eq!(m.workspace.tab_of(moved), Some(0));
    assert!(m.move_pane(moved, PaneDrop::Tab(9)).is_empty());
}

#[test]
fn move_tab_effects() {
    let mut m = model();
    assert_eq!(m.move_tab(0, 2), vec![Effect::Relayout]);
    assert!(m.move_tab(0, 0).is_empty(), "sans changement");
    assert!(m.move_tab(0, 1).is_empty(), "sans changement");
    assert!(m.move_tab(7, 0).is_empty());
}

#[test]
fn a_drop_abandons_the_rename() {
    let mut m = model();
    m.renaming = Some(Rename::new(RenameTarget::Tab(0), "x"));
    m.move_tab(0, 2);
    assert!(m.renaming.is_none());
}
