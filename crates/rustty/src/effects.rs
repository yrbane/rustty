//! Exécution des effets décidés par le modèle : terminaux à lancer ou
//! tuer, défilement, presse-papiers, rechargement, mise en page, sortie.

use winit::event_loop::ActiveEventLoop;

use crate::banner::Banner;
use crate::config_watch::{self, ReloadOutcome};
use crate::keyboard::paste_bytes;
use crate::model::Effect;
use crate::pane_fonts::zoom_target;
use crate::tab::TermId;
use crate::window_state::OsWindow;

impl OsWindow {
    pub fn run_effects(&mut self, effects: Vec<Effect>, event_loop: &ActiveEventLoop) {
        for effect in effects {
            match effect {
                Effect::SpawnTerm(id) => {
                    if let Err(e) = self.spawn_term(id) {
                        tracing::error!("{e:#}");
                        self.model.set_notice(Some(Banner::error(format!("{e:#}"))));
                        self.model.workspace.close_term(id);
                    }
                }
                Effect::CloseTerm(id) => self.close_term(id),
                Effect::Scroll(id, request) => {
                    if let Some(tw) = self.terms.get(&id) {
                        tw.scroll(request);
                    }
                    self.window.request_redraw();
                }
                Effect::Copy => self.copy_selection(),
                Effect::Paste => self.paste(),
                Effect::SetOpacity(_) | Effect::Redraw => self.window.request_redraw(),
                Effect::FontSize(change) => {
                    // Seul le panneau survolé (sinon le focalisé) change de taille.
                    let (x, y) = self.cursor;
                    let hovered = self.pane_under(x, y).map(|(term, _)| term);
                    if let Some(term) = zoom_target(
                        hovered,
                        self.cursor_inside,
                        self.model.workspace.focused_term(),
                    ) && self.pane_fonts.apply(term, change)
                    {
                        self.relayout();
                    }
                    self.window.request_redraw();
                }
                Effect::ReloadConfig => self.reload_config(),
                Effect::Relayout => {
                    self.relayout();
                    self.window.request_redraw();
                }
                Effect::Exit => event_loop.exit(),
            }
        }
    }

    fn close_term(&mut self, id: TermId) {
        // Le Drop du Pty tue et moissonne le shell dans un thread détaché.
        self.terms.remove(&id);
        self.titles.remove(&id);
        self.pane_fonts.forget(id);
        if self.menu.as_ref().is_some_and(|m| m.term == id) {
            self.menu = None;
        }
        if self.selection.as_ref().is_some_and(|(t, _)| *t == id) {
            self.selection = None;
        }
    }

    fn copy_selection(&mut self) {
        let Some((term, sel)) = &self.selection else {
            return;
        };
        let Some(tw) = self.terms.get(term) else {
            return;
        };
        let text = sel.text(&tw.snapshot());
        if text.is_empty() {
            return;
        }
        if let Some(cb) = self.clipboard.as_mut()
            && let Err(e) = cb.set_text(text)
        {
            tracing::warn!("copie : {e}");
        }
    }

    fn paste(&mut self) {
        let Some(term) = self.model.workspace.focused_term() else {
            return;
        };
        let Some(tw) = self.terms.get(&term) else {
            return;
        };
        let Some(cb) = self.clipboard.as_mut() else {
            return;
        };
        match cb.get_text() {
            Ok(text) => {
                tw.write(paste_bytes(&text, tw.modes().bracketed_paste));
                tw.scroll(crate::model::ScrollRequest::ToBottom);
                self.window.request_redraw();
            }
            Err(e) => tracing::debug!("collage : {e}"),
        }
    }

    /// Le shell de `id` s'est-il terminé ? Si oui, le modèle décide de la suite.
    pub fn check_exit(&mut self, id: TermId) -> Vec<Effect> {
        let Some(tw) = self.terms.get_mut(&id) else {
            return Vec::new();
        };
        match tw.poll_exit() {
            Some(status) => {
                // Avec --hold le panneau mort reste : le menu ne doit pas le viser.
                if self.menu.as_ref().is_some_and(|m| m.term == id) {
                    self.menu = None;
                }
                self.model.term_exited(id, status)
            }
            None => Vec::new(),
        }
    }

    pub fn reload_config(&mut self) {
        match config_watch::reload(self.config_path.as_deref()) {
            ReloadOutcome::Applied(c) => {
                self.model.set_notice(None);
                self.apply_config(*c);
            }
            ReloadOutcome::Rejected(e) => {
                self.model
                    .set_notice(Some(Banner::error(format!("configuration : {e}"))));
                self.window.request_redraw();
            }
        }
    }
}
