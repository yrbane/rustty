//! Tests de l'exécution des entrées de menu selon leur cible.

use rustty_config::SplitAxis;

use super::super::*;
use crate::context_menu::{MenuItem, MenuTarget};
use crate::rename::RenameTarget;

fn idle(_: TermId) -> bool {
    false
}

fn busy(_: TermId) -> bool {
    true
}

/// Trois onglets (termes 1, 2, 3), le premier actif.
fn three_tabs() -> Model {
    let (mut m, _) = Model::new(0.9, false);
    m.apply(Action::NewTab, false, &idle);
    m.apply(Action::NewTab, false, &idle);
    m.workspace.go_to_tab(1);
    m
}

#[test]
fn tab_menu_targets_the_clicked_tab_not_the_active_one() {
    let mut m = three_tabs();
    m.menu_command(MenuTarget::Tab(2), MenuItem::RenameTab, false, &idle);
    assert_eq!(
        m.renaming.as_ref().map(|r| r.target),
        Some(RenameTarget::Tab(2))
    );
    assert_eq!(m.workspace.active(), 0, "l'onglet actif ne change pas");
}

#[test]
fn new_tab_from_tab_menu_follows_the_clicked_tab() {
    let mut m = three_tabs();
    let effects = m.menu_command(MenuTarget::Tab(1), MenuItem::NewTab, false, &idle);
    assert_eq!(m.workspace.active(), 2, "juste après l'onglet cliqué");
    assert_eq!(m.workspace.tabs().len(), 4);
    assert_eq!(
        effects,
        vec![
            Effect::SpawnTerm {
                term: TermId(4),
                cwd_of: Some(TermId(2))
            },
            Effect::Relayout
        ]
    );
}

#[test]
fn close_tab_from_menu_closes_the_clicked_tab() {
    let mut m = three_tabs();
    let effects = m.menu_command(MenuTarget::Tab(2), MenuItem::CloseTab, false, &idle);
    assert_eq!(
        effects,
        vec![Effect::CloseTerm(TermId(3)), Effect::Relayout]
    );
    assert_eq!(m.workspace.tabs().len(), 2);
}

#[test]
fn close_other_tabs_asks_when_something_runs() {
    let mut m = three_tabs();
    let effects = m.menu_command(MenuTarget::Tab(1), MenuItem::CloseOtherTabs, true, &busy);
    assert_eq!(effects, vec![Effect::Redraw]);
    assert_eq!(m.pending_close, Some(CloseRequest::OtherTabs(1)));
    assert_eq!(
        m.workspace.tabs().len(),
        3,
        "rien n'est fermé avant confirmation"
    );
    let effects = m.confirm_pending();
    assert!(effects.contains(&Effect::CloseTerm(TermId(1))));
    assert!(effects.contains(&Effect::CloseTerm(TermId(3))));
    assert_eq!(m.workspace.tabs().len(), 1);
    assert_eq!(m.workspace.focused_term(), Some(TermId(2)));
}

#[test]
fn close_other_tabs_closes_directly_otherwise() {
    let mut m = three_tabs();
    let effects = m.menu_command(MenuTarget::Tab(1), MenuItem::CloseOtherTabs, true, &idle);
    assert_eq!(m.pending_close, None);
    assert_eq!(
        effects,
        vec![
            Effect::CloseTerm(TermId(1)),
            Effect::CloseTerm(TermId(3)),
            Effect::Relayout
        ]
    );
    assert_eq!(m.workspace.tabs().len(), 1);
}

#[test]
fn header_split_targets_its_pane() {
    let (mut m, first) = Model::new(0.9, false);
    let second = m.workspace.split_focused(SplitAxis::Vertical).unwrap();
    assert_eq!(m.workspace.focused_term(), Some(second));
    let effects = m.menu_command(
        MenuTarget::Header(first),
        MenuItem::SplitHorizontal,
        false,
        &idle,
    );
    assert_eq!(
        effects,
        vec![
            Effect::SpawnTerm {
                term: TermId(3),
                cwd_of: Some(first)
            },
            Effect::Relayout
        ],
        "le dossier est celui du bandeau, pas du panneau focalisé avant"
    );
}

#[test]
fn header_move_to_new_tab_creates_a_tab_after() {
    let (mut m, first) = Model::new(0.9, false);
    let second = m.workspace.split_focused(SplitAxis::Vertical).unwrap();
    let effects = m.menu_command(
        MenuTarget::Header(first),
        MenuItem::MoveToNewTab,
        false,
        &idle,
    );
    assert_eq!(effects, vec![Effect::Relayout]);
    assert_eq!(m.workspace.tabs().len(), 2);
    assert_eq!(m.workspace.tab_of(first), Some(1));
    assert_eq!(m.workspace.tab_of(second), Some(0));
}

#[test]
fn header_rename_and_close_target_its_pane() {
    let (mut m, first) = Model::new(0.9, false);
    let second = m.workspace.split_focused(SplitAxis::Vertical).unwrap();
    m.menu_command(
        MenuTarget::Header(first),
        MenuItem::RenamePane,
        false,
        &idle,
    );
    assert_eq!(
        m.renaming.as_ref().map(|r| r.target),
        Some(RenameTarget::Pane(first))
    );
    m.renaming = None;
    let effects = m.menu_command(MenuTarget::Header(second), MenuItem::ClosePane, true, &busy);
    assert_eq!(m.pending_close, Some(CloseRequest::Term(second)));
    assert_eq!(effects, vec![Effect::Redraw]);
}

#[test]
fn palette_sets_and_resets_the_tab_color() {
    let mut m = three_tabs();
    let effects = m.menu_command(MenuTarget::Palette(1), MenuItem::Color(4), false, &idle);
    assert_eq!(effects, vec![Effect::Relayout]);
    assert_eq!(m.workspace.tabs()[1].color, Some(4));
    assert_eq!(m.workspace.tabs()[0].color, None);
    let effects = m.menu_command(MenuTarget::Palette(1), MenuItem::DefaultColor, false, &idle);
    assert_eq!(effects, vec![Effect::Relayout]);
    assert_eq!(m.workspace.tabs()[1].color, None);
}

#[test]
fn pane_target_keeps_the_current_behavior() {
    let (mut m, first) = Model::new(0.9, false);
    let effects = m.menu_command(
        MenuTarget::Pane(first),
        MenuItem::SplitVertical,
        false,
        &idle,
    );
    assert_eq!(
        effects,
        vec![
            Effect::SpawnTerm {
                term: TermId(2),
                cwd_of: Some(first)
            },
            Effect::Relayout
        ]
    );
}
