//! Relâchement de souris adressé au terminal qui a reçu l'appui.

use rustty_render::PixelRect;

use crate::geometry;
use crate::mouse::{MouseButton, MouseKind, encode_mouse};
use crate::tab::TermId;
use crate::window_state::OsWindow;

/// Où envoyer un relâchement : le terminal tenu et son rectangle, même si la
/// souris est ailleurs. `None` sans bouton tenu ou si le panneau a disparu.
pub fn release_target(
    held: Option<(TermId, MouseButton)>,
    rect_of: impl Fn(TermId) -> Option<PixelRect>,
) -> Option<(TermId, MouseButton, PixelRect)> {
    let (term, button) = held?;
    Some((term, button, rect_of(term)?))
}

impl OsWindow {
    /// Appui rapporté à l'application : retient le terminal et le bouton.
    pub(crate) fn hold(&mut self, term: TermId, button: MouseButton) {
        self.held = Some((term, button));
        self.motion.reset();
    }

    /// Mouvement rapporté à l'application, une fois par cellule traversée.
    pub(crate) fn report_motion(&mut self, term: TermId, cell: Option<(usize, usize)>) {
        let Some(cell) = cell else { return };
        if !self.motion.should_report(term, cell) {
            return;
        }
        if let Some(tw) = self.terms.get(&term)
            && let Some(bytes) = encode_mouse(
                MouseKind::Motion,
                self.held.map(|(_, b)| b),
                Some(cell),
                self.modifiers,
                &tw.modes(),
            )
        {
            tw.write(bytes);
        }
    }

    /// Relâchement d'un bouton tenu : va au terminal de l'appui, cellule
    /// bornée à son panneau. Vrai si un bouton était tenu.
    pub(crate) fn release_held(&mut self, x: f64, y: f64) -> bool {
        self.motion.reset();
        let held = self.held.take();
        let tab = self.model.workspace.active_tab();
        let rect_of = |term: TermId| {
            self.pane_rects
                .iter()
                .find(|(wid, _)| tab.term_at(*wid) == Some(term))
                .map(|(_, r)| *r)
        };
        let Some((term, button, rect)) = release_target(held, rect_of) else {
            return held.is_some();
        };
        let (col, row) = geometry::cell_at_clamped(
            rect,
            self.metrics_of(term),
            self.config.window.padding,
            x,
            y,
        );
        if let Some(tw) = self.terms.get(&term)
            && let Some(bytes) = encode_mouse(
                MouseKind::Release,
                Some(button),
                Some((col, row)),
                self.modifiers,
                &tw.modes(),
            )
        {
            tw.write(bytes);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_goes_to_the_term_that_got_the_press() {
        let rect = PixelRect::new(0, 0, 100, 100);
        let rect_of = |t: TermId| (t == TermId(1)).then_some(rect);
        assert_eq!(
            release_target(Some((TermId(1), MouseButton::Left)), rect_of),
            Some((TermId(1), MouseButton::Left, rect))
        );
        assert_eq!(
            release_target(Some((TermId(2), MouseButton::Left)), rect_of),
            None,
            "panneau disparu"
        );
        assert_eq!(release_target(None, rect_of), None);
    }
}
