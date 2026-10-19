//! Souris sur les bandeaux de panneau : décision pure puis exécution.

use std::time::Instant;

use crate::model::{CloseRequest, Effect};
use crate::mouse::MouseButton;
use crate::pane_header::{HeaderHit, header_at};
use crate::tab::TermId;
use crate::window_state::OsWindow;

/// Ce qu'une pression sur un bandeau déclenche ; jamais un rapport au terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderAction {
    Close(TermId),
    Focus(TermId),
    Rename(TermId),
    /// Clic droit : absorbé, le menu du bandeau viendra s'y brancher.
    Menu(TermId),
    Ignore,
}

pub fn header_action(hit: HeaderHit, button: MouseButton, double: bool) -> HeaderAction {
    match (hit, button) {
        (HeaderHit::Close(t), MouseButton::Left) => HeaderAction::Close(t),
        (HeaderHit::Title(t), MouseButton::Left) if double => HeaderAction::Rename(t),
        (HeaderHit::Title(t), MouseButton::Left) => HeaderAction::Focus(t),
        (HeaderHit::Title(t) | HeaderHit::Close(t), MouseButton::Right) => HeaderAction::Menu(t),
        _ => HeaderAction::Ignore,
    }
}

impl OsWindow {
    /// Pression sur un bandeau ; `None` hors bandeau. Rien n'est transmis au terminal.
    pub fn header_press(
        &mut self,
        button: MouseButton,
        x: f64,
        y: f64,
        now: Instant,
    ) -> Option<Vec<Effect>> {
        let hit = header_at(&self.headers, x, y)?;
        let double = match hit {
            HeaderHit::Title(t) if button == MouseButton::Left => {
                // L'identifiant du terminal sert de cible au détecteur de double clic.
                self.header_clicks.register(t.0 as usize, now)
            }
            _ => false,
        };
        let mut effects = self.model.cancel_rename();
        match header_action(hit, button, double) {
            HeaderAction::Close(term) => {
                let confirm = self.config.window.confirm_close_with_running_children;
                let running = self
                    .terms
                    .get(&term)
                    .is_some_and(crate::term_window::TermWindow::has_running_children);
                effects.extend(
                    self.model
                        .request_close(CloseRequest::Term(term), confirm && running),
                );
            }
            HeaderAction::Focus(term) => effects.extend(self.focus_pane(term)),
            HeaderAction::Rename(term) => {
                effects.extend(self.focus_pane(term));
                effects.extend(self.model.start_pane_rename(term));
            }
            HeaderAction::Menu(_) | HeaderAction::Ignore => {}
        }
        Some(effects)
    }

    fn focus_pane(&mut self, term: TermId) -> Vec<Effect> {
        if self.model.workspace.focused_term() == Some(term) {
            return Vec::new();
        }
        self.model.workspace.focus_term(term);
        self.update_title();
        vec![Effect::Redraw]
    }
}

#[cfg(test)]
#[path = "header_input_tests.rs"]
mod tests;
