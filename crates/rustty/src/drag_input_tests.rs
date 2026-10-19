//! Tests du choix de la cible d'un glisser.

use rustty_layout::Side;
use rustty_render::{PixelRect, TabBarLayout, TabRect};

use super::*;
use crate::drag::{DragSource, DropTarget, PaneDrop};
use crate::tab::TermId;
use crate::workspace::Workspace;

/// Deux panneaux côte à côte sous une barre d'onglets haute de 30 px.
fn panes() -> Vec<(TermId, PixelRect)> {
    vec![
        (TermId(1), PixelRect::new(0, 30, 200, 100)),
        (TermId(2), PixelRect::new(210, 30, 200, 100)),
    ]
}

fn bar() -> TabBarLayout {
    TabBarLayout {
        bar: PixelRect::new(0, 0, 410, 30),
        tabs: vec![
            TabRect {
                rect: PixelRect::new(0, 0, 100, 30),
                close: None,
            },
            TabRect {
                rect: PixelRect::new(104, 0, 100, 30),
                close: None,
            },
        ],
        new_tab: PixelRect::new(208, 0, 30, 30),
    }
}

fn pick(source: DragSource, x: f64, y: f64) -> DropTarget {
    pick_target(source, x, y, &panes(), Some(&bar()))
}

#[test]
fn pane_is_not_a_target_for_itself() {
    let source = DragSource::Pane(TermId(1));
    assert_eq!(pick(source, 100.0, 80.0), DropTarget::None);
    assert_eq!(
        pick(source, 220.0, 80.0),
        DropTarget::Pane {
            term: TermId(2),
            side: Side::Left,
        }
    );
    assert_eq!(
        pick(source, 310.0, 125.0),
        DropTarget::Pane {
            term: TermId(2),
            side: Side::Bottom,
        }
    );
    // Dans l'espace entre les panneaux : rien.
    assert_eq!(pick(source, 205.0, 80.0), DropTarget::None);
}

#[test]
fn tab_bar_wins_over_panes() {
    // Des panneaux qui débordent sous la barre : la barre décide.
    let under = vec![(TermId(2), PixelRect::new(0, 0, 410, 130))];
    let source = DragSource::Pane(TermId(1));
    assert_eq!(
        pick_target(source, 150.0, 10.0, &under, Some(&bar())),
        DropTarget::OnTab(1)
    );
    // Sur la barre hors de tout onglet (bouton « + ») : aucune cible.
    assert_eq!(
        pick_target(source, 220.0, 10.0, &under, Some(&bar())),
        DropTarget::None
    );
    // Sans barre, le panneau redevient la cible.
    assert!(matches!(
        pick_target(source, 150.0, 10.0, &under, None),
        DropTarget::Pane {
            term: TermId(2),
            ..
        }
    ));
}

#[test]
fn tab_drag_targets_slots_only() {
    let source = DragSource::Tab(0);
    assert_eq!(pick(source, 220.0, 80.0), DropTarget::None);
    assert_eq!(pick(source, 20.0, 10.0), DropTarget::TabSlot(0));
    assert_eq!(pick(source, 60.0, 10.0), DropTarget::TabSlot(1));
    assert_eq!(pick(source, 180.0, 10.0), DropTarget::TabSlot(2));
    assert_eq!(pick(source, 300.0, 10.0), DropTarget::TabSlot(2));
    assert_eq!(
        pick_target(source, 20.0, 10.0, &panes(), None),
        DropTarget::None
    );
}

#[test]
fn release_maps_source_and_target_to_a_move() {
    let pane = DragSource::Pane(TermId(1));
    let on_pane = DropTarget::Pane {
        term: TermId(2),
        side: Side::Top,
    };
    assert_eq!(
        drop_action(pane, on_pane),
        Some(DropAction::Pane(
            TermId(1),
            PaneDrop::Pane {
                target: TermId(2),
                side: Side::Top,
            }
        ))
    );
    assert_eq!(
        drop_action(pane, DropTarget::OnTab(3)),
        Some(DropAction::Pane(TermId(1), PaneDrop::Tab(3)))
    );
    assert_eq!(
        drop_action(DragSource::Tab(0), DropTarget::TabSlot(2)),
        Some(DropAction::Tab(0, 2))
    );
}

#[test]
fn drop_on_nothing_changes_nothing() {
    assert_eq!(
        drop_action(DragSource::Pane(TermId(1)), DropTarget::None),
        None
    );
    assert_eq!(drop_action(DragSource::Tab(0), DropTarget::None), None);
    // Combinaisons que `pick_target` ne produit pas : ignorées.
    assert_eq!(
        drop_action(DragSource::Pane(TermId(1)), DropTarget::TabSlot(0)),
        None
    );
    assert_eq!(drop_action(DragSource::Tab(0), DropTarget::OnTab(1)), None);
}

#[test]
fn drag_of_a_vanished_source_is_dropped() {
    let (mut ws, first) = Workspace::new();
    let second = ws.new_tab();
    assert!(source_alive(DragSource::Pane(first), &ws));
    assert!(source_alive(DragSource::Tab(1), &ws));
    assert!(!source_alive(DragSource::Pane(TermId(99)), &ws));
    assert!(!source_alive(DragSource::Tab(2), &ws));
    ws.close_term(second);
    assert!(!source_alive(DragSource::Pane(second), &ws));
    assert!(!source_alive(DragSource::Tab(1), &ws));
}
