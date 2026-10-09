//! Glisser une barre de split à la souris, et la forme du curseur qui
//! l'annonce (↔ entre deux panneaux côte à côte, ↕ entre deux empilés).

use rustty_layout::Axis;
use winit::window::CursorIcon;

use crate::geometry::{self, GRAB_WIDTH};
use crate::model::Effect;
use crate::window_state::OsWindow;

impl OsWindow {
    /// Appui gauche : commence un glisser si une barre est sous la souris.
    pub(crate) fn start_divider_drag(&mut self, x: f64, y: f64) -> bool {
        self.drag = geometry::divider_at(&self.dividers, x, y, GRAB_WIDTH);
        self.drag.is_some()
    }

    /// Mouvement : déplace la barre saisie ; `None` si aucun glisser en cours.
    pub(crate) fn drag_divider_to(&mut self, x: f64, y: f64) -> Option<Vec<Effect>> {
        let id = self.drag?;
        let gap = geometry::split_gap(&self.config.splits);
        let content = self.content;
        let (px, py) = (x.max(0.0) as u32, y.max(0.0) as u32);
        let moved = !self.model.workspace.is_empty()
            && self
                .model
                .workspace
                .active_tab_mut()
                .layout
                .drag_divider(id, content, gap, px, py);
        Some(if moved {
            vec![Effect::Relayout]
        } else {
            Vec::new()
        })
    }

    /// Relâchement : vrai si un glisser vient de se terminer.
    pub(crate) fn end_divider_drag(&mut self) -> bool {
        self.drag.take().is_some()
    }

    /// Met à jour la forme du curseur selon ce qu'il survole.
    pub(crate) fn update_cursor_icon(&mut self, x: f64, y: f64) {
        let over = self
            .drag
            .or_else(|| geometry::divider_at(&self.dividers, x, y, GRAB_WIDTH));
        let axis = over.and_then(|id| {
            (!self.model.workspace.is_empty())
                .then(|| self.model.workspace.active_tab().layout.split_axis(id))
                .flatten()
        });
        let icon = match axis {
            Some(Axis::Vertical) => CursorIcon::ColResize,
            Some(Axis::Horizontal) => CursorIcon::RowResize,
            None if geometry::pane_at(&self.pane_rects, x, y).is_some() => CursorIcon::Text,
            None => CursorIcon::Default,
        };
        if icon != self.cursor_icon {
            self.cursor_icon = icon;
            self.window.set_cursor(icon);
        }
    }
}
