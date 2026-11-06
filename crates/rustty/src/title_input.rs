//! La souris sur la barre de titre maison et les bords de la fenêtre :
//! machine à états pure `TitleDrag` (clic ou déplacement), puis le câblage
//! dans la fenêtre (boutons, déplacement, redimensionnement, fermeture).

use std::time::Instant;

use rustty_config::TitleButton;
use rustty_render::{TitleBarLayout, TitleTarget};
use winit::window::ResizeDirection;

use crate::context_menu::ContextMenu;
use crate::model::Effect;
use crate::mouse::MouseButton;
use crate::tab::TermId;
use crate::window_edges::resize_direction;
use crate::window_state::OsWindow;

/// Distance (pixels logiques) à partir de laquelle un appui devient un
/// déplacement de fenêtre plutôt qu'un clic ; la fenêtre la met à l'échelle.
pub const MOVE_THRESHOLD: f64 = 4.0;

/// Une fois le déplacement confié au système, celui-ci prend le pointeur et
/// le relâchement ne nous parvient pas : la machine revient aussitôt au repos.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TitleDrag {
    Idle,
    /// Bouton gauche enfoncé à cette position, pas encore de déplacement.
    Armed {
        x: f64,
        y: f64,
    },
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

    /// Armé et déplacé d'au moins `threshold` pixels : demande le déplacement,
    /// une seule fois, et revient au repos.
    pub fn moved(&mut self, x: f64, y: f64, threshold: f64) -> TitleDragEvent {
        match *self {
            TitleDrag::Armed { x: x0, y: y0 } if (x - x0).hypot(y - y0) >= threshold => {
                *self = TitleDrag::Idle;
                TitleDragEvent::StartMove
            }
            _ => TitleDragEvent::None,
        }
    }

    /// Relâché sans avoir bougé : un clic. Toujours au repos ensuite.
    pub fn release(&mut self) -> TitleDragEvent {
        let event = match *self {
            TitleDrag::Armed { .. } => TitleDragEvent::Click,
            TitleDrag::Idle => TitleDragEvent::None,
        };
        *self = TitleDrag::Idle;
        event
    }
}

/// Les effets d'un bouton de la barre. Fermer passe par
/// `OsWindow::request_close_window`, couleur par `open_title_bar_palette`.
pub fn button_effects(button: TitleButton) -> Vec<Effect> {
    match button {
        TitleButton::Minimize => vec![Effect::Minimize],
        TitleButton::Maximize => vec![Effect::ToggleMaximize],
        TitleButton::Settings => vec![Effect::OpenConfig],
        TitleButton::Folder => vec![Effect::OpenCwd],
        TitleButton::Close | TitleButton::Color => Vec::new(),
    }
}

/// Le coin haut-gauche du nuancier de la barre : sous le bouton couleur,
/// ou sous le début du titre si le bouton est masqué.
pub fn palette_anchor(layout: &TitleBarLayout) -> (u32, u32) {
    layout
        .buttons
        .iter()
        .find(|(b, _)| *b == TitleButton::Color)
        .map_or((layout.title.x, layout.bar.height), |(_, r)| {
            (r.x, r.y + r.height)
        })
}

/// Un clic (sans déplacement) relâché sur le titre compte pour le double
/// clic qui renomme la fenêtre.
pub fn title_click_renames(event: TitleDragEvent, target: TitleTarget) -> bool {
    event == TitleDragEvent::Click && target == TitleTarget::Title
}

/// La position de la souris en pixels entiers, ou `None` hors de la fenêtre
/// (coordonnées négatives, comme pendant un relâchement à l'extérieur).
pub fn bar_point(x: f64, y: f64) -> Option<(u32, u32)> {
    (x >= 0.0 && y >= 0.0).then_some((x as u32, y as u32))
}

impl OsWindow {
    /// Ce que la barre de titre a sous `(x, y)` ; `None` sans barre maison.
    fn title_target(&self, x: f64, y: f64) -> TitleTarget {
        match (self.title_bar.as_ref(), bar_point(x, y)) {
            (Some(bar), Some((px, py))) => bar.hit_test(px, py),
            _ => TitleTarget::None,
        }
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

    /// Appui sur la barre de titre : annule le renommage (le menu est déjà
    /// fermé par `menu_click`), retient le bouton visé ou arme un
    /// déplacement. `None` hors de la barre.
    pub fn title_press(&mut self, button: MouseButton, x: f64, y: f64) -> Option<Vec<Effect>> {
        let target = self.title_target(x, y);
        if target == TitleTarget::None {
            return None;
        }
        let effects = self.model.cancel_rename();
        match target {
            TitleTarget::Button(b) => self.title_press = Some(b),
            TitleTarget::Title | TitleTarget::Background => {
                self.title_drag.press(button, x, y);
            }
            TitleTarget::None => {}
        }
        Some(effects)
    }

    /// Mouvement : démarre le déplacement de la fenêtre au-delà du seuil
    /// (mis à l'échelle de l'écran). Vrai si le système a pris la main.
    pub fn title_drag_moved(&mut self, x: f64, y: f64) -> bool {
        let threshold = MOVE_THRESHOLD * self.window.scale_factor();
        if self.title_drag.moved(x, y, threshold) != TitleDragEvent::StartMove {
            return false;
        }
        // Un déplacement n'est pas la moitié d'un double clic.
        self.title_clicks.reset();
        if let Err(e) = self.window.drag_window() {
            tracing::debug!("déplacement refusé par le système : {e}");
        }
        true
    }

    /// Suit le bouton de la barre survolé. Vrai s'il a changé.
    pub fn update_title_hover(&mut self, x: f64, y: f64) -> bool {
        let hover = match self.title_target(x, y) {
            TitleTarget::Button(b) => Some(b),
            _ => None,
        };
        let changed = hover != self.title_hover;
        self.title_hover = hover;
        changed
    }

    /// Relâchement : déclenche le bouton si la souris y est encore, termine
    /// un clic ou un déplacement. `None` si la barre n'était pas concernée.
    pub fn title_release(&mut self, button: MouseButton, x: f64, y: f64) -> Option<Vec<Effect>> {
        let pressed = self.title_press.take();
        let dragging = self.title_drag != TitleDrag::Idle;
        if pressed.is_none() && !dragging {
            return None;
        }
        let event = self.title_drag.release();
        let target = self.title_target(x, y);
        if title_click_renames(event, target) && self.title_clicks.register(0, Instant::now()) {
            return Some(self.model.start_window_rename());
        }
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

    /// La souris est partie ou la fenêtre a perdu le focus : plus de survol,
    /// plus d'appui ni de déplacement en cours. Vrai si l'affichage change.
    pub fn leave_title_bar(&mut self) -> bool {
        self.title_press = None;
        self.title_drag = TitleDrag::Idle;
        self.title_clicks.reset();
        self.title_hover.take().is_some()
    }

    /// Les effets d'un bouton de la barre, fermer et couleur compris.
    pub fn title_button_action(&mut self, button: TitleButton) -> Vec<Effect> {
        match button {
            TitleButton::Close => self.request_close_window(),
            TitleButton::Color => self.open_title_bar_palette(),
            other => button_effects(other),
        }
    }

    /// Ouvre le nuancier de la barre de titre (bouton couleur ou action
    /// `pick_titlebar_color`) ; rien sans barre maison, ni pendant une
    /// confirmation ou un renommage.
    pub fn open_title_bar_palette(&mut self) -> Vec<Effect> {
        let Some(bar) = self.title_bar.as_ref() else {
            return Vec::new();
        };
        if !self.model.accepts_context_menu() {
            return Vec::new();
        }
        let (x, y) = palette_anchor(bar);
        self.menu = Some(ContextMenu::title_bar(x, y));
        // Un menu qui s'ouvre met fin à tout glisser en cours.
        let mut effects = self.cancel_drag();
        effects.push(Effect::Redraw);
        effects
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
