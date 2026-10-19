//! Exécution des entrées de menu contextuel selon leur cible.

use super::{CloseRequest, Effect, Model};
use crate::context_menu::{MenuItem, MenuTarget};
use crate::drag::PaneDrop;
use crate::tab::TermId;

impl Model {
    /// Exécute `item` sur la cible du menu ; la cible, pas l'état courant,
    /// décide de l'onglet ou du panneau visé.
    pub fn menu_command(
        &mut self,
        target: MenuTarget,
        item: MenuItem,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        match target {
            MenuTarget::Pane(term) => self.focused_action(term, item, confirm_close, running),
            MenuTarget::Header(term) => self.header_command(term, item, confirm_close, running),
            MenuTarget::Tab(tab) => self.tab_command(tab, item, confirm_close, running),
            MenuTarget::Palette(tab) => self.palette_command(tab, item),
        }
    }

    /// Donne le focus à `term` puis applique l'action de l'entrée.
    fn focused_action(
        &mut self,
        term: TermId,
        item: MenuItem,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        let Some(action) = item.action() else {
            return Vec::new();
        };
        self.workspace.focus_term(term);
        self.apply(action, confirm_close, running)
    }

    fn header_command(
        &mut self,
        term: TermId,
        item: MenuItem,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        match item {
            MenuItem::MoveToNewTab => self.move_pane(term, PaneDrop::NewTab),
            MenuItem::RenamePane => {
                self.workspace.focus_term(term);
                self.start_pane_rename(term)
            }
            // `ClosePane` passe par l'action de fermeture du panneau focalisé.
            _ => self.focused_action(term, item, confirm_close, running),
        }
    }

    fn tab_command(
        &mut self,
        tab: usize,
        item: MenuItem,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        if tab >= self.workspace.tabs().len() {
            return Vec::new();
        }
        match item {
            MenuItem::RenameTab => self.start_rename(tab),
            MenuItem::NewTab => {
                // Le nouvel onglet s'insère après l'actif : on active d'abord le cliqué.
                self.workspace.activate(tab);
                self.apply(rustty_config::Action::NewTab, confirm_close, running)
            }
            MenuItem::CloseTab => {
                self.close_with_check(CloseRequest::Tab(tab), confirm_close, running)
            }
            MenuItem::CloseOtherTabs => {
                self.close_with_check(CloseRequest::OtherTabs(tab), confirm_close, running)
            }
            _ => Vec::new(),
        }
    }

    fn palette_command(&mut self, tab: usize, item: MenuItem) -> Vec<Effect> {
        let color = match item {
            MenuItem::Color(slot) => Some(slot),
            MenuItem::DefaultColor => None,
            _ => return Vec::new(),
        };
        self.workspace.set_tab_color(tab, color);
        vec![Effect::Relayout]
    }

    /// Demande la fermeture, avec confirmation si un programme tourne dans les terminaux visés.
    fn close_with_check(
        &mut self,
        req: CloseRequest,
        confirm_close: bool,
        running: &dyn Fn(TermId) -> bool,
    ) -> Vec<Effect> {
        let needs = confirm_close && self.terms_of(req).iter().any(|t| running(*t));
        self.request_close(req, needs)
    }
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
