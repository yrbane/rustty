//! Machine à états pure de la souris sur la zone de titre : un clic gauche
//! arme, un petit relâchement est un clic, un déplacement au-delà du seuil
//! démarre le déplacement de la fenêtre.

// Câblé dans la fenêtre à la tâche 8 : retirer cette autorisation alors.
#![allow(dead_code)]

use winit::event::MouseButton;

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

#[cfg(test)]
#[path = "title_input_tests.rs"]
mod tests;
