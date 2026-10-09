//! L'état d'interface et ses décisions, sans fenêtre ni PTY : chaque entrée
//! (action, clic, fin de shell) rend la liste des effets à exécuter.

use std::collections::BTreeSet;

use rustty_config::Action;
use rustty_pty::ExitStatus;
use rustty_render::HoverTarget;

use crate::banner::Banner;
use crate::mouse::MouseButton;
use crate::tab::{Tab, TermId};
use crate::workspace::Workspace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollRequest {
    Lines(i32),
    Pages(i32),
    ToBottom,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    SpawnTerm(TermId),
    CloseTerm(TermId),
    Scroll(TermId, ScrollRequest),
    Copy,
    Paste,
    SetOpacity(f32),
    ReloadConfig,
    Relayout,
    Redraw,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseRequest {
    Tab(usize),
    Term(TermId),
    Window,
}

const CONFIRM_TEXT: &str =
    "Des programmes tournent encore — Entrée pour fermer, Échap pour annuler";

pub struct Model {
    pub workspace: Workspace,
    pub opacity: f32,
    pub hover: HoverTarget,
    pub pending_close: Option<CloseRequest>,
    pub hold: bool,
    notice: Option<Banner>,
    dead: BTreeSet<TermId>,
}

impl Model {
    pub fn new(opacity: f32, hold: bool) -> (Self, TermId) {
        let (workspace, first) = Workspace::new();
        let model = Self {
            workspace,
            opacity,
            hover: HoverTarget::None,
            pending_close: None,
            hold,
            notice: None,
            dead: BTreeSet::new(),
        };
        (model, first)
    }

    pub fn set_notice(&mut self, notice: Option<Banner>) {
        self.notice = notice;
    }

    pub fn banner(&self) -> Option<Banner> {
        if self.pending_close.is_some() {
            return Some(Banner::confirm(CONFIRM_TEXT));
        }
        self.notice.clone()
    }

    pub fn is_dead(&self, term: TermId) -> bool {
        self.dead.contains(&term)
    }

    pub fn hover_changed(&mut self, target: HoverTarget) -> bool {
        let changed = self.hover != target;
        self.hover = target;
        changed
    }

    pub fn apply(
        &mut self,
        action: Action,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        match action {
            Action::NewTab => Self::spawn(Some(self.workspace.new_tab())),
            Action::Split(axis) => {
                let term = self.workspace.split_focused(axis);
                Self::spawn(term)
            }
            Action::CloseTab => {
                let req = CloseRequest::Tab(self.workspace.active());
                let needs = confirm_close && self.terms_of(req).iter().any(|t| running(*t));
                self.request_close(req, needs)
            }
            Action::CloseWindow => match self.workspace.focused_term() {
                Some(term) => {
                    self.request_close(CloseRequest::Term(term), confirm_close && running(term))
                }
                None => Vec::new(),
            },
            Action::NextTab => {
                self.workspace.next_tab();
                vec![Effect::Relayout]
            }
            Action::PrevTab => {
                self.workspace.prev_tab();
                vec![Effect::Relayout]
            }
            Action::GoToTab(n) => {
                self.workspace.go_to_tab(n);
                vec![Effect::Relayout]
            }
            Action::Focus(dir) => {
                self.workspace.focus(dir);
                vec![Effect::Relayout]
            }
            Action::Resize(dir) => {
                self.workspace.resize(dir);
                vec![Effect::Relayout]
            }
            Action::Rotate => {
                self.workspace.rotate();
                vec![Effect::Relayout]
            }
            Action::ToggleZoom => {
                self.workspace.toggle_zoom();
                vec![Effect::Relayout]
            }
            Action::Opacity(delta) => {
                self.opacity = (self.opacity + delta).clamp(0.0, 1.0);
                vec![Effect::SetOpacity(self.opacity)]
            }
            Action::Copy => vec![Effect::Copy],
            Action::Paste => vec![Effect::Paste],
            Action::ReloadConfig => vec![Effect::ReloadConfig],
            Action::ScrollLines(n) => self.scroll(ScrollRequest::Lines(n)),
            Action::ScrollPages(n) => self.scroll(ScrollRequest::Pages(n)),
            Action::ScrollToBottom => self.scroll(ScrollRequest::ToBottom),
            Action::Unbind => Vec::new(),
        }
    }

    fn spawn(term: Option<TermId>) -> Vec<Effect> {
        match term {
            Some(t) => vec![Effect::SpawnTerm(t), Effect::Relayout],
            None => Vec::new(),
        }
    }

    fn scroll(&self, request: ScrollRequest) -> Vec<Effect> {
        self.workspace
            .focused_term()
            .map(|t| vec![Effect::Scroll(t, request)])
            .unwrap_or_default()
    }

    fn terms_of(&self, req: CloseRequest) -> Vec<TermId> {
        match req {
            CloseRequest::Tab(i) => self
                .workspace
                .tabs()
                .get(i)
                .map(|t| t.terms())
                .unwrap_or_default(),
            CloseRequest::Term(t) => vec![t],
            CloseRequest::Window => self.workspace.tabs().iter().flat_map(Tab::terms).collect(),
        }
    }

    pub fn request_close_window(
        &mut self,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        let needs = confirm_close
            && self
                .terms_of(CloseRequest::Window)
                .iter()
                .any(|t| running(*t));
        self.request_close(CloseRequest::Window, needs)
    }

    pub fn request_close(&mut self, req: CloseRequest, needs_confirm: bool) -> Vec<Effect> {
        if needs_confirm {
            self.pending_close = Some(req);
            return vec![Effect::Redraw];
        }
        self.execute_close(req)
    }

    fn execute_close(&mut self, req: CloseRequest) -> Vec<Effect> {
        let killed = match req {
            CloseRequest::Tab(i) => self.workspace.close_tab(i),
            CloseRequest::Term(t) => self
                .workspace
                .close_term(t)
                .then_some(t)
                .into_iter()
                .collect(),
            CloseRequest::Window => {
                let mut all = Vec::new();
                while !self.workspace.is_empty() {
                    all.extend(self.workspace.close_tab(0));
                }
                all
            }
        };
        let mut effects: Vec<Effect> = killed.iter().map(|t| Effect::CloseTerm(*t)).collect();
        for t in &killed {
            self.dead.remove(t);
        }
        effects.push(if self.workspace.is_empty() {
            Effect::Exit
        } else {
            Effect::Relayout
        });
        effects
    }

    pub fn confirm_pending(&mut self) -> Vec<Effect> {
        match self.pending_close.take() {
            Some(req) => self.execute_close(req),
            None => Vec::new(),
        }
    }

    pub fn cancel_pending(&mut self) -> Vec<Effect> {
        match self.pending_close.take() {
            Some(_) => vec![Effect::Redraw],
            None => Vec::new(),
        }
    }

    pub fn tab_bar_click(
        &mut self,
        target: HoverTarget,
        button: MouseButton,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        let close_tab = |model: &mut Self, i: usize| {
            let req = CloseRequest::Tab(i);
            let needs = confirm_close && model.terms_of(req).iter().any(|t| running(*t));
            model.request_close(req, needs)
        };
        match (target, button) {
            (HoverTarget::Tab(i), MouseButton::Left) => {
                self.workspace
                    .go_to_tab(u8::try_from(i + 1).unwrap_or(u8::MAX));
                vec![Effect::Relayout]
            }
            (HoverTarget::CloseButton(i), MouseButton::Left)
            | (HoverTarget::Tab(i), MouseButton::Middle) => close_tab(self, i),
            (HoverTarget::NewTabButton, MouseButton::Left) => {
                Self::spawn(Some(self.workspace.new_tab()))
            }
            _ => Vec::new(),
        }
    }

    pub fn term_exited(&mut self, term: TermId, status: ExitStatus) -> Vec<Effect> {
        if !self.hold {
            return self.execute_close(CloseRequest::Term(term));
        }
        self.dead.insert(term);
        let status_text = match status {
            ExitStatus::Exited(code) => format!("code {code}"),
            ExitStatus::Signaled => "tué par un signal".to_string(),
        };
        self.notice = Some(Banner::info(format!(
            "[processus terminé : {status_text}] — une touche pour fermer"
        )));
        vec![Effect::Redraw]
    }

    pub fn key_on_dead_term(&mut self, term: TermId) -> Vec<Effect> {
        if !self.dead.remove(&term) {
            return Vec::new();
        }
        self.notice = None;
        self.execute_close(CloseRequest::Term(term))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
