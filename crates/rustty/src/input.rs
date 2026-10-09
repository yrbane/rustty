//! Réactions aux entrées winit : clavier (raccourcis puis encodage), souris
//! (barre d'onglets, sélection, rapports aux applications, molette), focus.

use std::time::Instant;

use rustty_config::Mods;
use rustty_render::HoverTarget;
use rustty_vt::MouseMode;
use winit::event::{ElementState, KeyEvent, MouseButton as WinitButton};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::geometry;
use crate::keyboard::{encode_key, key_combo, mods_from_winit};
use crate::model::{Effect, ScrollRequest};
use crate::mouse::{CellPos, MouseButton, MouseKind, Selection, encode_mouse};
use crate::rename::RenameKey;
use crate::tab::TermId;
use crate::window_state::OsWindow;

impl OsWindow {
    pub fn on_modifiers(&mut self, state: ModifiersState) {
        self.modifiers = mods_from_winit(state);
    }

    pub fn on_key(&mut self, event: &KeyEvent) -> Vec<Effect> {
        if event.state != ElementState::Pressed {
            return Vec::new();
        }
        if self.model.renaming.is_some() {
            let key = match &event.logical_key {
                Key::Named(NamedKey::Enter) => RenameKey::Commit,
                Key::Named(NamedKey::Escape) => RenameKey::Cancel,
                Key::Named(NamedKey::Backspace) => RenameKey::Backspace,
                _ => match event.text.as_deref() {
                    Some(text) => RenameKey::Text(text.to_string()),
                    None => return Vec::new(),
                },
            };
            return self.model.rename_key(key);
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
    pub(crate) fn pane_under(&self, x: f64, y: f64) -> Option<(TermId, rustty_render::PixelRect)> {
        let wid = geometry::pane_at(&self.pane_rects, x, y)?;
        let rect = self.pane_rects.iter().find(|(w, _)| *w == wid)?.1;
        let term = self.model.workspace.active_tab().term_at(wid)?;
        Some((term, rect))
    }

    pub(crate) fn cell_under(
        &self,
        term: TermId,
        rect: rustty_render::PixelRect,
        x: f64,
        y: f64,
    ) -> Option<(usize, usize)> {
        geometry::cell_at(
            rect,
            self.metrics_of(term),
            self.config.window.padding,
            x,
            y,
        )
    }

    pub fn on_cursor_moved(&mut self, x: f64, y: f64) -> Vec<Effect> {
        self.cursor = (x, y);
        self.update_cursor_icon(x, y);
        if let Some(effects) = self.drag_divider_to(x, y) {
            return effects;
        }
        let mut effects = Vec::new();
        if self.model.hover_changed(self.bar_target(x, y)) {
            effects.push(Effect::Redraw);
        }
        let Some((term, rect)) = self.pane_under(x, y) else {
            return effects;
        };
        let cell = self.cell_under(term, rect, x, y);
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
        if button == MouseButton::Left && self.start_divider_drag(x, y) {
            return Vec::new();
        }
        let Some((term, rect)) = self.pane_under(x, y) else {
            return Vec::new();
        };
        // Un clic dans un panneau ramène au terminal : l'édition de nom s'arrête.
        let mut effects = self.model.cancel_rename();
        if self.model.workspace.focused_term() != Some(term) {
            self.model.workspace.focus_term(term);
            self.update_title();
            effects.push(Effect::Redraw);
        }
        let cell = self.cell_under(term, rect, x, y);
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
        if self.end_divider_drag() {
            return Vec::new();
        }
        if self.press_target != HoverTarget::None {
            let target = std::mem::replace(&mut self.press_target, HoverTarget::None);
            if target != self.bar_target(x, y) {
                return Vec::new();
            }
            if let (HoverTarget::Tab(i), MouseButton::Left) = (target, button)
                && self.tab_clicks.register(i, Instant::now())
            {
                return self.model.start_rename(i);
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
                    self.cell_under(term, rect, x, y),
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
}
