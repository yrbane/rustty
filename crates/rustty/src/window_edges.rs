//! Bords de redimensionnement de la fenêtre sans décorations : quelle
//! direction correspond à une position de souris, et quel curseur l'annonce.

use winit::window::{CursorIcon, ResizeDirection};

/// La direction de redimensionnement sous `(x, y)` (pixels physiques), si la
/// souris est dans la marge de `border` pixels le long du bord ; `None` au
/// centre, sans marge, ou quand la fenêtre est maximisée.
pub fn resize_direction(
    x: f64,
    y: f64,
    width: u32,
    height: u32,
    border: u32,
    maximized: bool,
) -> Option<ResizeDirection> {
    if maximized || border == 0 {
        return None;
    }
    let border = f64::from(border);
    let west = x < border;
    let east = x >= f64::from(width) - border;
    let north = y < border;
    let south = y >= f64::from(height) - border;
    match (north, south, west, east) {
        (true, _, true, _) => Some(ResizeDirection::NorthWest),
        (true, _, _, true) => Some(ResizeDirection::NorthEast),
        (_, true, true, _) => Some(ResizeDirection::SouthWest),
        (_, true, _, true) => Some(ResizeDirection::SouthEast),
        (true, _, _, _) => Some(ResizeDirection::North),
        (_, true, _, _) => Some(ResizeDirection::South),
        (_, _, true, _) => Some(ResizeDirection::West),
        (_, _, _, true) => Some(ResizeDirection::East),
        _ => None,
    }
}

/// Le curseur à afficher au-dessus d'un bord de redimensionnement.
pub fn resize_cursor(direction: ResizeDirection) -> CursorIcon {
    match direction {
        ResizeDirection::West | ResizeDirection::East => CursorIcon::EwResize,
        ResizeDirection::North | ResizeDirection::South => CursorIcon::NsResize,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::NwseResize,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::NeswResize,
    }
}

#[cfg(test)]
#[path = "window_edges_tests.rs"]
mod tests;
