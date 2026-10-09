//! Molette (défilement, zoom de police avec ctrl, rapports aux applications,
//! flèches sur l'écran alternatif) et changement de focus de la fenêtre.

use rustty_config::{Key as ConfigKey, Mods, NamedKey as ConfigNamed};
use rustty_vt::MouseMode;
use winit::event::MouseScrollDelta;

use crate::font_zoom::wheel_change;
use crate::keyboard::encode_key;
use crate::model::{Effect, ScrollRequest};
use crate::mouse::{MouseButton, MouseKind, encode_mouse};
use crate::window_state::OsWindow;

/// Lignes défilées par cran de molette.
const WHEEL_LINES: f64 = 3.0;

/// Lignes défilées par un événement ; le pavé tactile (pixels) se mesure
/// à la hauteur de cellule du panneau concerné.
pub fn wheel_lines(delta: MouseScrollDelta, cell_height: u32) -> f64 {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => f64::from(y) * WHEEL_LINES,
        MouseScrollDelta::PixelDelta(p) => p.y / f64::from(cell_height.max(1)),
    }
}

impl OsWindow {
    pub fn on_wheel(&mut self, delta: MouseScrollDelta) -> Vec<Effect> {
        let (x, y) = self.cursor;
        let target = self
            .pane_under(x, y)
            .map(|(term, _)| term)
            .or_else(|| self.model.workspace.focused_term());
        let cell_height =
            target.map_or_else(|| self.metrics().height, |t| self.metrics_of(t).height);
        let lines = wheel_lines(delta, cell_height);
        let lines = self.wheel.lines(lines);
        if lines == 0 {
            return Vec::new();
        }
        if self.modifiers.contains(Mods::CTRL) {
            return wheel_change(lines)
                .map(Effect::FontSize)
                .into_iter()
                .collect();
        }
        let Some((term, rect)) = self.pane_under(x, y).or_else(|| {
            self.model
                .workspace
                .focused_term()
                .map(|t| (t, rustty_render::PixelRect::default()))
        }) else {
            return Vec::new();
        };
        let Some(tw) = self.terms.get(&term) else {
            return Vec::new();
        };
        let modes = tw.modes();
        let steps = lines.unsigned_abs() as usize;
        if modes.mouse != MouseMode::None && !self.modifiers.contains(Mods::SHIFT) {
            let button = if lines > 0 {
                MouseButton::WheelUp
            } else {
                MouseButton::WheelDown
            };
            let cell = self.cell_under(term, rect, x, y);
            for _ in 0..steps {
                if let Some(bytes) =
                    encode_mouse(MouseKind::Press, Some(button), cell, self.modifiers, &modes)
                {
                    tw.write(bytes);
                }
            }
            return Vec::new();
        }
        if modes.alt_screen {
            // Pas d'historique sur l'écran alternatif : la molette devient des flèches.
            let named = if lines > 0 {
                ConfigNamed::Up
            } else {
                ConfigNamed::Down
            };
            let combo = Some(rustty_config::KeyCombo {
                mods: Mods::empty(),
                key: ConfigKey::Named(named),
            });
            let bytes = encode_key(combo, None, Mods::empty(), &modes);
            for _ in 0..steps {
                tw.write(bytes.clone());
            }
            return Vec::new();
        }
        vec![Effect::Scroll(term, ScrollRequest::Lines(lines))]
    }

    pub fn on_focus(&mut self, focused: bool) -> Vec<Effect> {
        self.focused = focused;
        if !focused {
            self.menu = None;
        }
        if let Some(term) = self.model.workspace.focused_term()
            && let Some(tw) = self.terms.get(&term)
            && tw.modes().focus_events
        {
            tw.write(if focused {
                b"\x1b[I".to_vec()
            } else {
                b"\x1b[O".to_vec()
            });
        }
        vec![Effect::Redraw]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalPosition;

    #[test]
    fn pixel_delta_uses_the_pane_cell_height() {
        let px = |y| MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, y));
        assert_eq!(wheel_lines(px(40.0), 20), 2.0);
        assert_eq!(wheel_lines(px(40.0), 40), 1.0);
        assert_eq!(wheel_lines(MouseScrollDelta::LineDelta(0.0, 1.0), 20), 3.0);
    }
}
