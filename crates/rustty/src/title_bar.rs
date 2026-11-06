//! La barre de titre maison dans la fenêtre : décorations système retirées
//! ou rendues, style à l'échelle de l'écran, disposition et chrome.

use rustty_config::TitleBar;
use rustty_render::{
    Chrome, TitleBarState, TitleBarStyle, layout_title_bar, title_bar_chrome, title_bar_height,
};

use crate::rename::RenameTarget;
use crate::title;
use crate::window_state::OsWindow;

/// Les décorations de winit ne servent que sans barre maison.
pub fn decorations_wanted(cfg: &TitleBar) -> bool {
    !cfg.enabled
}

/// Ce que `set_decorations` doit recevoir si la configuration rechargée
/// change de barre, `None` sinon.
pub fn decorations_change(old: &TitleBar, new: &TitleBar) -> Option<bool> {
    (old.enabled != new.enabled).then(|| decorations_wanted(new))
}

impl OsWindow {
    /// Hauteur réservée en haut de la fenêtre, 0 sans barre maison.
    pub fn title_bar_height(&self) -> u32 {
        if self.config.titlebar.enabled {
            title_bar_height(&self.title_style)
        } else {
            0
        }
    }

    /// Style recalculé pour la configuration et l'échelle d'écran courantes.
    pub fn rebuild_title_style(&mut self) {
        self.title_style =
            TitleBarStyle::from_config(&self.config.titlebar, self.window.scale_factor());
    }

    /// Dispose la barre pour la largeur donnée, ou rien si elle est désactivée.
    pub fn relayout_title_bar(&mut self, width: u32, enabled: bool) {
        self.title_bar = enabled.then(|| layout_title_bar(width, &self.title_style));
    }

    /// Le texte donné à winit et dessiné dans la barre : nom fixe de la
    /// fenêtre, sinon nom de l'onglet actif, sinon titre du shell, puis la
    /// version ; pendant le renommage de la fenêtre, le tampon et un curseur.
    pub fn window_title_text(&self) -> String {
        let workspace = &self.model.workspace;
        let raw = workspace
            .focused_term()
            .and_then(|t| self.titles.get(&t))
            .map(String::as_str)
            .unwrap_or("");
        let custom = workspace
            .tabs()
            .get(workspace.active())
            .and_then(|tab| tab.custom_title.as_deref());
        let editing = self
            .model
            .renaming
            .as_ref()
            .filter(|r| r.target == RenameTarget::Window)
            .map(|r| r.buffer.as_str());
        title::bar_title(
            self.model.window_name.as_deref(),
            custom,
            raw,
            editing,
            crate::VERSION,
        )
    }

    pub fn title_bar_chrome(&self) -> Option<Chrome> {
        let bar = self.title_bar.as_ref()?;
        let title = self.window_title_text();
        Some(title_bar_chrome(
            bar,
            &self.title_style,
            TitleBarState {
                title: &title,
                hover: self.title_hover,
                maximized: self.maximized,
                background: self.model.titlebar_color.map(|s| self.palette.ansi[s]),
            },
            self.metrics(),
        ))
    }
}

#[cfg(test)]
#[path = "title_bar_tests.rs"]
mod tests;
