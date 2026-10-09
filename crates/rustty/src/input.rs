//! Réactions aux entrées winit : clavier (raccourcis puis encodage), souris
//! (barre d'onglets, sélection, rapports aux applications, molette), focus.

use rustty_config::{Key as ConfigKey, Mods, NamedKey as ConfigNamed};
use rustty_render::HoverTarget;
use rustty_vt::MouseMode;
use winit::event::{ElementState, KeyEvent, MouseButton as WinitButton, MouseScrollDelta};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::geometry;
use crate::keyboard::{encode_key, key_combo, mods_from_winit};
use crate::model::{Effect, ScrollRequest};
use crate::mouse::{CellPos, MouseButton, MouseKind, Selection, encode_mouse};
use crate::tab::TermId;
use crate::window_state::OsWindow;

/// Lignes défilées par cran de molette.
const WHEEL_LINES: f64 = 3.0;

impl OsWindow {
    pub fn on_modifiers(&mut self, state: ModifiersState) {
        self.modifiers = mods_from_winit(state);
    }

    pub fn on_key(&mut self, event: &KeyEvent) -> Vec<Effect> {
        if event.state != ElementState::Pressed {
            return Vec::new();
        }
        if self.model.pending_close.is_some() {
            return match &event.logical_key {
                Key::Named(NamedKey::Enter) => self.model.confirm_pending(),
                Key::Named(NamedKey::Escape) => self.model.cancel_pending(),
                _ => Vec::new(),
            };
        }
        let Some(term) = self.model.workspace.focused_term() else {
            return Vec::new();
        };
        if self.model.is_dead(term) {
            return self.model.key_on_dead_term(term);
        }
        let mods = self.modifiers;
        let combo = key_combo(&event.logical_key, mods);
        if let Some(c) = combo
            && let Some(action) = self.config.keys.resolve(c)
        {
            let confirm = self.config.window.confirm_close_with_running_children;
            let terms = &self.terms;
            let running = |t: TermId| {
                terms
                    .get(&t)
                    .is_some_and(crate::term_window::TermWindow::has_running_children)
            };
            return self.model.apply(action, confirm, &running);
        }
        let Some(tw) = self.terms.get(&term) else {
            return Vec::new();
        };
        let bytes = encode_key(combo, event.text.as_deref(), mods, &tw.modes());
        if bytes.is_empty() {
            return Vec::new();
        }
        tw.write(bytes);
        tw.scroll(ScrollRequest::ToBottom);
        self.selection = None;
        vec![Effect::Redraw]
    }

    fn bar_target(&self, x: f64, y: f64) -> HoverTarget {
        if x < 0.0 || y < 0.0 {
            return HoverTarget::None;
        }
        self.tab_bar
            .as_ref()
            .map(|b| b.hit_test(x as u32, y as u32))
            .unwrap_or(HoverTarget::None)
    }

    /// Le terminal et le rectangle du panneau sous le point, dans l'onglet actif.
    fn pane_under(&self, x: f64, y: f64) -> Option<(TermId, rustty_render::PixelRect)> {
        let wid = geometry::pane_at(&self.pane_rects, x, y)?;
        let rect = self.pane_rects.iter().find(|(w, _)| *w == wid)?.1;
        let term = self.model.workspace.active_tab().term_at(wid)?;
        Some((term, rect))
    }

    fn cell_under(&self, rect: rustty_render::PixelRect, x: f64, y: f64) -> Option<(usize, usize)> {
        geometry::cell_at(rect, self.metrics(), self.config.window.padding, x, y)
    }

    pub fn on_cursor_moved(&mut self, x: f64, y: f64) -> Vec<Effect> {
        self.cursor = (x, y);
        let mut effects = Vec::new();
        if self.model.hover_changed(self.bar_target(x, y)) {
            effects.push(Effect::Redraw);
        }
        let Some((term, rect)) = self.pane_under(x, y) else {
            return effects;
        };
        let cell = self.cell_under(rect, x, y);
        if self.dragging
            && let Some((sel_term, sel)) = &mut self.selection
            && *sel_term == term
            && let Some((col, row)) = cell
        {
            sel.extend(CellPos { col, row });
            effects.push(Effect::Redraw);
        }
        if let Some(tw) = self.terms.get(&term)
            && let Some(bytes) = encode_mouse(
                MouseKind::Motion,
                self.held,
                cell,
                self.modifiers,
                &tw.modes(),
            )
        {
            tw.write(bytes);
        }
        effects
    }

    pub fn on_cursor_left(&mut self) -> Vec<Effect> {
        if self.model.hover_changed(HoverTarget::None) {
            vec![Effect::Redraw]
        } else {
            Vec::new()
        }
    }

    pub fn on_mouse_input(&mut self, state: ElementState, button: WinitButton) -> Vec<Effect> {
        let button = match button {
            WinitButton::Left => MouseButton::Left,
            WinitButton::Middle => MouseButton::Middle,
            WinitButton::Right => MouseButton::Right,
            _ => return Vec::new(),
        };
        let (x, y) = self.cursor;
        match state {
            ElementState::Pressed => self.on_press(button, x, y),
            ElementState::Released => self.on_release(button, x, y),
        }
    }

    fn on_press(&mut self, button: MouseButton, x: f64, y: f64) -> Vec<Effect> {
        let on_bar = self.bar_target(x, y);
        if on_bar != HoverTarget::None {
            self.press_target = on_bar;
            return Vec::new();
        }
        let Some((term, rect)) = self.pane_under(x, y) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        if self.model.workspace.focused_term() != Some(term) {
            self.model.workspace.focus_term(term);
            self.update_title();
            effects.push(Effect::Redraw);
        }
        let cell = self.cell_under(rect, x, y);
        let Some(tw) = self.terms.get(&term) else {
            return effects;
        };
        let modes = tw.modes();
        if modes.mouse != MouseMode::None && !self.modifiers.contains(Mods::SHIFT) {
            if let Some(bytes) =
                encode_mouse(MouseKind::Press, Some(button), cell, self.modifiers, &modes)
            {
                tw.write(bytes);
            }
            self.held = Some(button);
            return effects;
        }
        match button {
            MouseButton::Left => {
                self.selection =
                    cell.map(|(col, row)| (term, Selection::start(CellPos { col, row })));
                self.dragging = self.selection.is_some();
                effects.push(Effect::Redraw);
            }
            MouseButton::Middle => effects.push(Effect::Paste),
            _ => {}
        }
        effects
    }

    fn on_release(&mut self, button: MouseButton, x: f64, y: f64) -> Vec<Effect> {
        if self.press_target != HoverTarget::None {
            let target = std::mem::replace(&mut self.press_target, HoverTarget::None);
            if target != self.bar_target(x, y) {
                return Vec::new();
            }
            let confirm = self.config.window.confirm_close_with_running_children;
            let terms = &self.terms;
            let running = |t: TermId| {
                terms
                    .get(&t)
                    .is_some_and(crate::term_window::TermWindow::has_running_children)
            };
            return self.model.tab_bar_click(target, button, confirm, &running);
        }
        if let Some(held) = self.held.take() {
            if let Some((term, rect)) = self.pane_under(x, y)
                && let Some(tw) = self.terms.get(&term)
                && let Some(bytes) = encode_mouse(
                    MouseKind::Release,
                    Some(held),
                    self.cell_under(rect, x, y),
                    self.modifiers,
                    &tw.modes(),
                )
            {
                tw.write(bytes);
            }
            return Vec::new();
        }
        if self.dragging && button == MouseButton::Left {
            self.dragging = false;
            if self.selection.as_ref().is_some_and(|(_, s)| !s.is_empty()) {
                return vec![Effect::Copy];
            }
        }
        Vec::new()
    }

    pub fn on_wheel(&mut self, delta: MouseScrollDelta) -> Vec<Effect> {
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(y) * WHEEL_LINES,
            MouseScrollDelta::PixelDelta(p) => p.y / f64::from(self.metrics().height.max(1)),
        };
        let lines = self.wheel.lines(lines);
        if lines == 0 {
            return Vec::new();
        }
        let (x, y) = self.cursor;
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
            let cell = self.cell_under(rect, x, y);
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
