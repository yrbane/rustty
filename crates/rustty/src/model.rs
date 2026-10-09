//! L'état d'interface et ses décisions, sans fenêtre ni PTY : chaque entrée
//! (action, clic, fin de shell) rend la liste des effets à exécuter.

use std::collections::BTreeSet;

use rustty_config::Action;
use rustty_pty::ExitStatus;
use rustty_render::HoverTarget;

mod close;

use crate::banner::Banner;
use crate::font_zoom::FontChange;
use crate::mouse::MouseButton;
use crate::rename::Rename;
use crate::tab::TermId;
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
    FontSize(FontChange),
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
    /// Onglet en cours de renommage.
    pub renaming: Option<Rename>,
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
            renaming: None,
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

    /// Le menu contextuel peut s'ouvrir : ni confirmation ni renommage en cours.
    pub fn accepts_context_menu(&self) -> bool {
        self.pending_close.is_none() && self.renaming.is_none()
    }

    /// Abandonne une édition de nom en cours.
    pub fn cancel_rename(&mut self) -> Vec<Effect> {
        match self.renaming.take() {
            Some(_) => vec![Effect::Relayout],
            None => Vec::new(),
        }
    }

    pub fn apply(
        &mut self,
        action: Action,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        if action != Action::RenameTab {
            // Toute autre action peut décaler les onglets : l'édition s'arrête.
            self.renaming = None;
        }
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
            Action::RenameTab => self.start_rename(self.workspace.active()),
            Action::IncreaseFontSize => vec![Effect::FontSize(FontChange::Increase)],
            Action::DecreaseFontSize => vec![Effect::FontSize(FontChange::Decrease)],
            Action::ResetFontSize => vec![Effect::FontSize(FontChange::Reset)],
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

    pub fn tab_bar_click(
        &mut self,
        target: HoverTarget,
        button: MouseButton,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        self.renaming = None;
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
#[path = "model_tests.rs"]
mod tests;
