//! Le menu du clic droit dans un panneau : ouverture, survol, exécution de
//! l'entrée choisie, fermeture. La logique (entrées, placement, dessin) est
//! dans le module `context_menu`.

use rustty_render::Chrome;

use crate::context_menu::{
    ContextMenu, ENTRIES, MenuClick, MenuEntry, MenuLayout, layout, menu_chrome, shortcuts,
};
use crate::model::Effect;
use crate::mouse::MouseButton;
use crate::tab::TermId;
use crate::term_window::TermWindow;
use crate::window_state::OsWindow;

impl OsWindow {
    /// Ouvre le menu au point `(x, y)` pour le panneau de `term`.
    pub(crate) fn open_context_menu(&mut self, term: TermId, x: f64, y: f64) -> Vec<Effect> {
        let can_copy = self
            .selection
            .as_ref()
            .is_some_and(|(t, s)| *t == term && !s.is_empty());
        let zoomed = !self.model.workspace.is_empty()
            && self.model.workspace.active_tab().layout.zoomed().is_some();
        self.menu = Some(ContextMenu {
            term,
            x: x.max(0.0) as u32,
            y: y.max(0.0) as u32,
            can_copy,
            zoomed,
            hovered: None,
        });
        vec![Effect::Redraw]
    }

    pub(crate) fn close_context_menu(&mut self) -> Vec<Effect> {
        match self.menu.take() {
            Some(_) => vec![Effect::Redraw],
            None => Vec::new(),
        }
    }

    fn menu_layout(&self, menu: &ContextMenu) -> MenuLayout {
        layout(
            menu,
            self.surface.size(),
            self.metrics(),
            &shortcuts(&self.config.keys),
        )
    }

    /// Mouvement de souris menu ouvert : met à jour l'entrée survolée.
    /// `None` si aucun menu n'est ouvert.
    pub(crate) fn menu_hover(&mut self, x: f64, y: f64) -> Option<Vec<Effect>> {
        let menu = self.menu.as_ref()?;
        let hovered = self.menu_layout(menu).hit(menu, x, y);
        let menu = self.menu.as_mut()?;
        if menu.hovered == hovered {
            return Some(Vec::new());
        }
        menu.hovered = hovered;
        Some(vec![Effect::Redraw])
    }

    /// Appui de souris menu ouvert : exécute l'entrée cliquée, ignore un
    /// clic dans le menu hors des entrées actives, ferme sinon.
    /// `None` si aucun menu n'est ouvert.
    pub(crate) fn menu_click(
        &mut self,
        button: MouseButton,
        x: f64,
        y: f64,
    ) -> Option<Vec<Effect>> {
        let menu = self.menu.as_ref()?;
        let decision = self
            .menu_layout(menu)
            .click(menu, button == MouseButton::Left, x, y);
        let index = match decision {
            MenuClick::Keep => return Some(Vec::new()),
            MenuClick::Close => return Some(self.close_context_menu()),
            MenuClick::Run(index) => index,
        };
        let menu = self.menu.take()?;
        let mut effects = vec![Effect::Redraw];
        if let MenuEntry::Item(item) = ENTRIES[index] {
            // L'action vise le panneau du menu : il prend le focus d'abord.
            self.model.workspace.focus_term(menu.term);
            self.update_title();
            let confirm = self.config.window.confirm_close_with_running_children;
            let terms = &self.terms;
            let running = |t: TermId| terms.get(&t).is_some_and(TermWindow::has_running_children);
            effects.extend(self.model.apply(item.action(), confirm, &running));
        }
        Some(effects)
    }

    /// Le menu ouvert en chrome, pour la passe finale du rendu.
    pub(crate) fn context_menu_chrome(&self) -> Option<Chrome> {
        let menu = self.menu.as_ref()?;
        let keys = shortcuts(&self.config.keys);
        let layout = layout(menu, self.surface.size(), self.metrics(), &keys);
        Some(menu_chrome(
            menu,
            &layout,
            &keys,
            &self.palette,
            self.metrics(),
        ))
    }
}
