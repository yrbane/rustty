//! La souris sur la barre de titre maison et les bords de la fenêtre :
//! machine à états pure `TitleDrag` (clic ou déplacement), puis le câblage
//! dans la fenêtre (boutons, déplacement, redimensionnement, fermeture).

use rustty_config::TitleButton;
use rustty_render::TitleTarget;
use winit::window::ResizeDirection;

use crate::model::Effect;
use crate::mouse::MouseButton;
use crate::tab::TermId;
use crate::window_edges::resize_direction;
use crate::window_state::OsWindow;

/// Distance (pixels physiques) à partir de laquelle un appui devient un
/// déplacement de fenêtre plutôt qu'un clic.
pub const MOVE_THRESHOLD: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TitleDrag {
    Idle,
    /// Bouton gauche enfoncé à cette position, pas encore de déplacement.
    Armed {
        x: f64,
        y: f64,
    },
    /// Le déplacement de la fenêtre a été demandé au système.
    Moving,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleDragEvent {
    None,
    StartMove,
    Click,
}

impl TitleDrag {
    /// Seul le bouton gauche arme un déplacement.
    pub fn press(&mut self, button: MouseButton, x: f64, y: f64) {
        if button == MouseButton::Left {
            *self = TitleDrag::Armed { x, y };
        }
    }

    /// Armé et déplacé d'au moins `MOVE_THRESHOLD` : passe en mouvement et
    /// demande le déplacement, une seule fois.
    pub fn moved(&mut self, x: f64, y: f64) -> TitleDragEvent {
        match *self {
            TitleDrag::Armed { x: x0, y: y0 } if (x - x0).hypot(y - y0) >= MOVE_THRESHOLD => {
                *self = TitleDrag::Moving;
                TitleDragEvent::StartMove
            }
            _ => TitleDragEvent::None,
        }
    }

    /// Relâché sans avoir bougé : un clic ; après un déplacement : rien.
    /// Toujours au repos ensuite.
    pub fn release(&mut self) -> TitleDragEvent {
        let event = match *self {
            TitleDrag::Armed { .. } => TitleDragEvent::Click,
            TitleDrag::Idle | TitleDrag::Moving => TitleDragEvent::None,
        };
        *self = TitleDrag::Idle;
        event
    }
}

/// Les effets d'un bouton de fenêtre. Fermer passe par
/// `OsWindow::request_close_window` ; les boutons d'outils viennent au jalon 2.
pub fn button_effects(button: TitleButton) -> Vec<Effect> {
    match button {
        TitleButton::Minimize => vec![Effect::Minimize],
        TitleButton::Maximize => vec![Effect::ToggleMaximize],
        TitleButton::Close | TitleButton::Settings | TitleButton::Color | TitleButton::Folder => {
            Vec::new()
        }
    }
}

impl OsWindow {
    /// Ce que la barre de titre a sous `(x, y)` ; `None` sans barre maison.
    fn title_target(&self, x: f64, y: f64) -> TitleTarget {
        self.title_bar.as_ref().map_or(TitleTarget::None, |bar| {
            bar.hit_test(x.max(0.0) as u32, y.max(0.0) as u32)
        })
    }

    /// Le bord ou coin de redimensionnement sous `(x, y)` ; `None` avec les
    /// décorations système, hors de la marge ou fenêtre agrandie.
    pub(crate) fn edge_direction(&self, x: f64, y: f64) -> Option<ResizeDirection> {
        self.title_bar.as_ref()?;
        let size = self.window.inner_size();
        let border = (f64::from(self.config.titlebar.resize_border) * self.window.scale_factor())
            .round() as u32;
        resize_direction(x, y, size.width, size.height, border, self.maximized)
    }

    /// Appui gauche dans la marge de redimensionnement : demande au système
    /// de redimensionner la fenêtre. Vrai si l'appui est pris.
    pub fn edge_press(&mut self, button: MouseButton, x: f64, y: f64) -> bool {
        if button != MouseButton::Left {
            return false;
        }
        let Some(direction) = self.edge_direction(x, y) else {
            return false;
        };
        if let Err(e) = self.window.drag_resize_window(direction) {
            tracing::debug!("redimensionnement refusé par le système : {e}");
        }
        true
    }

    /// Appui sur la barre de titre : ferme le menu, annule le renommage,
    /// retient le bouton visé ou arme un déplacement. `None` hors de la barre.
    pub fn title_press(&mut self, button: MouseButton, x: f64, y: f64) -> Option<Vec<Effect>> {
        let target = self.title_target(x, y);
        if target == TitleTarget::None {
            return None;
        }
        let mut effects = self.model.cancel_rename();
        if self.menu.take().is_some() {
            effects.push(Effect::Redraw);
        }
        match target {
            TitleTarget::Button(b) => self.title_press = Some(b),
            TitleTarget::Title | TitleTarget::Background => {
                self.title_drag.press(button, x, y);
            }
            TitleTarget::None => {}
        }
        Some(effects)
    }

    /// Mouvement : démarre le déplacement de la fenêtre au-delà du seuil,
    /// sinon suit le bouton survolé. `None` si rien n'a changé.
    pub fn title_moved(&mut self, x: f64, y: f64) -> Option<Vec<Effect>> {
        if self.title_drag.moved(x, y) == TitleDragEvent::StartMove {
            if let Err(e) = self.window.drag_window() {
                tracing::debug!("déplacement refusé par le système : {e}");
            }
            return Some(Vec::new());
        }
        let hover = match self.title_target(x, y) {
            TitleTarget::Button(b) => Some(b),
            _ => None,
        };
        if hover == self.title_hover {
            return None;
        }
        self.title_hover = hover;
        Some(vec![Effect::Redraw])
    }

    /// Relâchement : déclenche le bouton si la souris y est encore, termine
    /// un clic ou un déplacement. `None` si la barre n'était pas concernée.
    pub fn title_release(&mut self, button: MouseButton, x: f64, y: f64) -> Option<Vec<Effect>> {
        let pressed = self.title_press.take();
        let dragging = self.title_drag != TitleDrag::Idle;
        if pressed.is_none() && !dragging {
            return None;
        }
        // `Click` servira au double clic sur le titre (jalon 3).
        let _ = self.title_drag.release();
        match pressed {
            Some(b)
                if button == MouseButton::Left
                    && self.title_target(x, y) == TitleTarget::Button(b) =>
            {
                Some(self.title_button_action(b))
            }
            _ => Some(Vec::new()),
        }
    }

    /// Les effets d'un bouton de fenêtre, fermer compris.
    pub fn title_button_action(&mut self, button: TitleButton) -> Vec<Effect> {
        match button {
            TitleButton::Close => self.request_close_window(),
            other => button_effects(other),
        }
    }

    /// Demande de fermeture de la fenêtre (croix du système ou bouton de la
    /// barre) : confirmation si un programme tourne encore et que la
    /// configuration le demande.
    pub fn request_close_window(&mut self) -> Vec<Effect> {
        let confirm = self.config.window.confirm_close_with_running_children;
        let terms = &self.terms;
        let running = |t: TermId| {
            terms
                .get(&t)
                .is_some_and(crate::term_window::TermWindow::has_running_children)
        };
        self.model.request_close_window(confirm, &running)
    }
}

#[cfg(test)]
#[path = "title_input_tests.rs"]
mod tests;
