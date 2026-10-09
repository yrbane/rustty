//! Tests du modèle d'application.

use super::*;
use crate::rename::RenameKey;

#[test]
fn no_menu_while_a_confirmation_or_rename_is_pending() {
    let (mut m, _) = Model::new(0.9, false);
    assert!(m.accepts_context_menu());
    m.start_rename(0);
    assert!(!m.accepts_context_menu(), "édition de nom en cours");
    m.cancel_rename();
    m.pending_close = Some(CloseRequest::Window);
    assert!(!m.accepts_context_menu(), "confirmation en attente");
    m.pending_close = None;
    assert!(m.accepts_context_menu());
}

#[test]
fn any_other_action_or_tab_bar_click_cancels_the_edit() {
    let (mut m, _) = Model::new(0.9, false);
    m.start_rename(0);
    m.apply(Action::NewTab, false, &never_running);
    assert!(m.renaming.is_none(), "un nouvel onglet décale les index");
    m.start_rename(0);
    m.tab_bar_click(
        HoverTarget::NewTabButton,
        MouseButton::Left,
        false,
        &never_running,
    );
    assert!(m.renaming.is_none());
    m.start_rename(0);
    assert_eq!(m.cancel_rename(), vec![Effect::Relayout]);
    assert!(m.renaming.is_none());
    assert!(m.cancel_rename().is_empty(), "rien à annuler");
}

#[test]
fn font_actions_emit_font_size_effects() {
    let (mut m, _) = Model::new(0.9, false);
    assert_eq!(
        m.apply(Action::IncreaseFontSize, false, &never_running),
        vec![Effect::FontSize(FontChange::Increase)]
    );
    assert_eq!(
        m.apply(Action::DecreaseFontSize, false, &never_running),
        vec![Effect::FontSize(FontChange::Decrease)]
    );
    assert_eq!(
        m.apply(Action::ResetFontSize, false, &never_running),
        vec![Effect::FontSize(FontChange::Reset)]
    );
}

#[test]
fn rename_action_edits_the_active_tab() {
    let (mut m, _) = Model::new(0.9, false);
    m.apply(Action::NewTab, false, &never_running);
    assert_eq!(
        m.apply(Action::RenameTab, false, &never_running),
        vec![Effect::Redraw]
    );
    assert_eq!(m.renaming.as_ref().map(|r| r.tab), Some(1));
}

#[test]
fn commit_renames_and_relayouts() {
    let (mut m, _) = Model::new(0.9, false);
    m.workspace.rename(0, Some("ancien".into()));
    m.start_rename(0);
    assert_eq!(m.renaming.as_ref().unwrap().buffer, "ancien", "pré-rempli");
    m.rename_key(RenameKey::Backspace);
    assert_eq!(
        m.rename_key(RenameKey::Text("x".into())),
        vec![Effect::Relayout]
    );
    assert_eq!(m.rename_key(RenameKey::Commit), vec![Effect::Relayout]);
    assert_eq!(
        m.workspace.tabs()[0].custom_title.as_deref(),
        Some("anciex")
    );
    assert!(m.renaming.is_none());
    m.start_rename(0);
    assert_eq!(m.rename_key(RenameKey::Cancel), vec![Effect::Redraw]);
    assert_eq!(
        m.workspace.tabs()[0].custom_title.as_deref(),
        Some("anciex")
    );
    assert!(m.start_rename(7).is_empty(), "onglet inexistant");
}

#[test]
fn closing_the_renamed_tab_cancels_the_edit() {
    let (mut m, _) = Model::new(0.9, false);
    m.apply(Action::NewTab, false, &never_running);
    m.start_rename(1);
    m.apply(Action::CloseTab, false, &never_running);
    assert!(m.renaming.is_none());
    assert!(m.rename_key(RenameKey::Commit).is_empty());
}
use rustty_config::SplitAxis;

fn never_running(_: TermId) -> bool {
    false
}

fn always_running(_: TermId) -> bool {
    true
}

#[test]
fn new_tab_and_split_spawn_then_relayout() {
    let (mut m, first) = Model::new(0.9, false);
    assert_eq!(first, TermId(1));
    assert_eq!(
        m.apply(Action::NewTab, true, &never_running),
        vec![Effect::SpawnTerm(TermId(2)), Effect::Relayout]
    );
    assert_eq!(
        m.apply(Action::Split(SplitAxis::Vertical), true, &never_running),
        vec![Effect::SpawnTerm(TermId(3)), Effect::Relayout]
    );
    assert_eq!(m.workspace.focused_term(), Some(TermId(3)));
}

#[test]
fn closing_the_last_term_exits() {
    let (mut m, first) = Model::new(0.9, false);
    assert_eq!(
        m.apply(Action::CloseWindow, true, &never_running),
        vec![Effect::CloseTerm(first), Effect::Exit]
    );
    assert!(m.workspace.is_empty());
}

#[test]
fn closing_a_tab_with_running_children_asks_first() {
    let (mut m, first) = Model::new(0.9, false);
    let effects = m.apply(Action::CloseTab, true, &always_running);
    assert_eq!(effects, vec![Effect::Redraw]);
    assert_eq!(m.pending_close, Some(CloseRequest::Tab(0)));
    assert_eq!(
        m.banner().map(|b| b.kind),
        Some(crate::banner::BannerKind::Confirm)
    );
    assert_eq!(m.cancel_pending(), vec![Effect::Redraw]);
    assert!(m.pending_close.is_none() && m.banner().is_none());
    m.apply(Action::CloseTab, true, &always_running);
    assert_eq!(
        m.confirm_pending(),
        vec![Effect::CloseTerm(first), Effect::Exit]
    );
    assert_eq!(m.confirm_pending(), Vec::<Effect>::new(), "rien en attente");
}

#[test]
fn confirmation_is_skipped_when_disabled_or_idle() {
    let (mut m, first) = Model::new(0.9, false);
    m.apply(Action::NewTab, false, &always_running);
    assert_eq!(
        m.apply(Action::CloseTab, false, &always_running),
        vec![Effect::CloseTerm(TermId(2)), Effect::Relayout]
    );
    assert_eq!(
        m.apply(Action::CloseTab, true, &never_running),
        vec![Effect::CloseTerm(first), Effect::Exit]
    );
}

#[test]
fn tab_navigation_and_layout_actions_relayout() {
    let (mut m, _) = Model::new(0.9, false);
    m.apply(Action::NewTab, false, &never_running);
    assert_eq!(
        m.apply(Action::PrevTab, false, &never_running),
        vec![Effect::Relayout]
    );
    assert_eq!(m.workspace.active(), 0);
    assert_eq!(
        m.apply(Action::GoToTab(2), false, &never_running),
        vec![Effect::Relayout]
    );
    assert_eq!(m.workspace.active(), 1);
    assert_eq!(
        m.apply(Action::Rotate, false, &never_running),
        vec![Effect::Relayout]
    );
    assert_eq!(
        m.apply(Action::ToggleZoom, false, &never_running),
        vec![Effect::Relayout]
    );
}

#[test]
fn opacity_is_clamped() {
    let (mut m, _) = Model::new(0.95, false);
    assert_eq!(
        m.apply(Action::Opacity(0.1), false, &never_running),
        vec![Effect::SetOpacity(1.0)]
    );
    assert_eq!(
        m.apply(Action::Opacity(-2.0), false, &never_running),
        vec![Effect::SetOpacity(0.0)]
    );
    assert_eq!(m.opacity, 0.0);
}

#[test]
fn scroll_copy_paste_reload_and_unbind() {
    let (mut m, first) = Model::new(0.9, false);
    assert_eq!(
        m.apply(Action::ScrollLines(-3), false, &never_running),
        vec![Effect::Scroll(first, ScrollRequest::Lines(-3))]
    );
    assert_eq!(
        m.apply(Action::ScrollPages(1), false, &never_running),
        vec![Effect::Scroll(first, ScrollRequest::Pages(1))]
    );
    assert_eq!(
        m.apply(Action::ScrollToBottom, false, &never_running),
        vec![Effect::Scroll(first, ScrollRequest::ToBottom)]
    );
    assert_eq!(
        m.apply(Action::Copy, false, &never_running),
        vec![Effect::Copy]
    );
    assert_eq!(
        m.apply(Action::Paste, false, &never_running),
        vec![Effect::Paste]
    );
    assert_eq!(
        m.apply(Action::ReloadConfig, false, &never_running),
        vec![Effect::ReloadConfig]
    );
    assert!(m.apply(Action::Unbind, false, &never_running).is_empty());
}

#[test]
fn hover_changed_only_reports_changes() {
    let (mut m, _) = Model::new(0.9, false);
    assert!(m.hover_changed(HoverTarget::Tab(0)));
    assert!(!m.hover_changed(HoverTarget::Tab(0)));
    assert!(m.hover_changed(HoverTarget::None));
}

#[test]
fn tab_bar_clicks() {
    let (mut m, first) = Model::new(0.9, false);
    assert_eq!(
        m.tab_bar_click(
            HoverTarget::NewTabButton,
            MouseButton::Left,
            false,
            &never_running
        ),
        vec![Effect::SpawnTerm(TermId(2)), Effect::Relayout]
    );
    assert_eq!(
        m.tab_bar_click(
            HoverTarget::Tab(0),
            MouseButton::Left,
            false,
            &never_running
        ),
        vec![Effect::Relayout]
    );
    assert_eq!(m.workspace.active(), 0);
    assert_eq!(
        m.tab_bar_click(
            HoverTarget::CloseButton(1),
            MouseButton::Left,
            false,
            &never_running
        ),
        vec![Effect::CloseTerm(TermId(2)), Effect::Relayout]
    );
    assert!(
        m.tab_bar_click(
            HoverTarget::Tab(0),
            MouseButton::Right,
            false,
            &never_running
        )
        .is_empty()
    );
    assert!(
        m.tab_bar_click(HoverTarget::None, MouseButton::Left, false, &never_running)
            .is_empty()
    );
    assert_eq!(
        m.tab_bar_click(
            HoverTarget::Tab(0),
            MouseButton::Middle,
            false,
            &never_running
        ),
        vec![Effect::CloseTerm(first), Effect::Exit]
    );
}

#[test]
fn exited_shell_closes_its_pane() {
    let (mut m, first) = Model::new(0.9, false);
    m.apply(Action::NewTab, false, &never_running);
    assert_eq!(
        m.term_exited(TermId(2), ExitStatus::Exited(0)),
        vec![Effect::CloseTerm(TermId(2)), Effect::Relayout]
    );
    assert_eq!(
        m.term_exited(first, ExitStatus::Signaled),
        vec![Effect::CloseTerm(first), Effect::Exit]
    );
}

#[test]
fn hold_keeps_a_dead_term_until_a_key() {
    let (mut m, first) = Model::new(0.9, true);
    assert_eq!(
        m.term_exited(first, ExitStatus::Exited(3)),
        vec![Effect::Redraw]
    );
    assert!(m.is_dead(first));
    assert!(m.banner().unwrap().text.contains("3"));
    assert!(m.key_on_dead_term(TermId(9)).is_empty(), "pas mort : rien");
    assert_eq!(
        m.key_on_dead_term(first),
        vec![Effect::CloseTerm(first), Effect::Exit]
    );
    assert!(m.banner().is_none());
}

#[test]
fn window_close_request_closes_everything() {
    let (mut m, first) = Model::new(0.9, false);
    m.apply(Action::NewTab, false, &never_running);
    assert_eq!(
        m.request_close_window(true, &always_running),
        vec![Effect::Redraw]
    );
    assert_eq!(m.pending_close, Some(CloseRequest::Window));
    let effects = m.confirm_pending();
    assert!(
        effects.contains(&Effect::CloseTerm(first))
            && effects.contains(&Effect::CloseTerm(TermId(2)))
    );
    assert_eq!(effects.last(), Some(&Effect::Exit));
    assert!(m.workspace.is_empty());
}

#[test]
fn pending_confirmation_wins_over_a_notice() {
    let (mut m, _) = Model::new(0.9, false);
    m.set_notice(Some(Banner::error("config")));
    assert_eq!(m.banner().unwrap().text, "config");
    m.apply(Action::CloseTab, true, &always_running);
    assert_eq!(m.banner().unwrap().kind, crate::banner::BannerKind::Confirm);
    m.cancel_pending();
    assert_eq!(m.banner().unwrap().text, "config");
}
