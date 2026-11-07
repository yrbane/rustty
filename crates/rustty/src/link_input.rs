//! Réactions de la fenêtre pour les hyperliens : survol (soulignement,
//! curseur, infobulle après un délai), clic d'ouverture. La logique pure
//! (plage, geste, machine de clic, texte) est dans le module `links`.

use std::time::{Duration, Instant};

use rustty_config::LinkClick as ClickMode;
use rustty_render::{CellRange, Chrome, PixelRect, grid_geometry};
use rustty_vt::{LinkId, MouseMode, Snapshot};
use unicode_width::UnicodeWidthStr;

use crate::links::{
    LinkGesture, LinkUnderMouse, gesture_armed, link_run, tooltip_hint, tooltip_text,
};
use crate::model::Effect;
use crate::tab::TermId;
use crate::tooltip::{self, Tooltip};
use crate::window_state::OsWindow;

/// Temps de survol avant l'infobulle.
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(400);

/// Le lien sous la souris.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkHover {
    pub term: TermId,
    pub id: LinkId,
    pub range: CellRange,
    pub uri: String,
    /// Début du survol : l'infobulle s'ouvre `TOOLTIP_DELAY` plus tard.
    pub since: Instant,
    pub tooltip: bool,
}

impl LinkHover {
    pub fn tooltip_due(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.since) >= TOOLTIP_DELAY
    }
}

impl OsWindow {
    /// Le programme du panneau suit la souris (modes 1000 à 1003).
    fn mouse_captured(&self, term: TermId) -> bool {
        self.terms
            .get(&term)
            .is_some_and(|tw| tw.modes().mouse != MouseMode::None)
    }

    pub(crate) fn link_gesture(&self, term: TermId) -> LinkGesture {
        LinkGesture {
            click: self.config.links.click,
            captured: self.mouse_captured(term),
        }
    }

    /// Le geste d'ouverture est-il tenu pour le lien survolé ?
    pub(crate) fn link_armed(&self) -> bool {
        self.link_hover
            .as_ref()
            .is_some_and(|h| gesture_armed(self.link_gesture(h.term), self.modifiers))
    }

    /// Le lien d'un schéma autorisé sous la cellule `cell` du panneau `term`.
    fn link_under(
        &self,
        term: TermId,
        cell: Option<(usize, usize)>,
    ) -> Option<(LinkId, CellRange, String)> {
        let snapshot = self.terms.get(&term)?.snapshot();
        self.link_in(&snapshot, cell)
    }

    /// Le lien d'un schéma autorisé sous `cell` dans `snapshot`.
    fn link_in(
        &self,
        snapshot: &Snapshot,
        cell: Option<(usize, usize)>,
    ) -> Option<(LinkId, CellRange, String)> {
        let (col, row) = cell?;
        let (id, range) = link_run(&snapshot.lines, row, col)?;
        let uri = snapshot.link_uri(id)?;
        self.config
            .links
            .scheme_allowed(uri)
            .then(|| (id, range, uri.to_string()))
    }

    /// Mouvement de souris sur la cellule `cell` du panneau `term` : entretient
    /// le lien survolé. Sous capture, seul un geste armé (shift) survole, et
    /// l'instantané (`cache`, partagé avec le correcteur) n'est pris qu'alors.
    /// Vrai s'il faut redessiner.
    pub(crate) fn link_hover(
        &mut self,
        term: TermId,
        cell: Option<(usize, usize)>,
        cache: &mut Option<Snapshot>,
    ) -> bool {
        let gesture = self.link_gesture(term);
        if gesture.captured && !gesture_armed(gesture, self.modifiers) {
            return self.leave_link();
        }
        let Some(snapshot) = self.cached_snapshot(term, cache) else {
            return self.leave_link();
        };
        let Some((id, range, uri)) = self.link_in(snapshot, cell) else {
            return self.leave_link();
        };
        if self
            .link_hover
            .as_ref()
            .is_some_and(|h| h.term == term && h.id == id && h.range == range)
        {
            return false;
        }
        self.link_hover = Some(LinkHover {
            term,
            id,
            range,
            uri,
            since: Instant::now(),
            tooltip: false,
        });
        true
    }

    /// Plus de lien survolé ni d'infobulle. Vrai s'il y en avait.
    pub(crate) fn leave_link(&mut self) -> bool {
        self.link_hover.take().is_some()
    }

    /// Quand l'infobulle doit s'ouvrir, tant qu'elle ne l'est pas.
    pub(crate) fn link_deadline(&self) -> Option<Instant> {
        self.link_hover
            .as_ref()
            .filter(|h| !h.tooltip)
            .map(|h| h.since + TOOLTIP_DELAY)
    }

    /// Ouvre l'infobulle quand son heure est venue. Vrai s'il faut redessiner.
    pub(crate) fn link_tick(&mut self, now: Instant) -> bool {
        let Some(h) = self.link_hover.as_mut() else {
            return false;
        };
        if h.tooltip || !h.tooltip_due(now) {
            return false;
        }
        h.tooltip = true;
        true
    }

    /// Les cellules du lien survolé dans le panneau `term`, à souligner.
    pub(crate) fn link_range_of(&self, term: TermId) -> Option<CellRange> {
        self.link_hover
            .as_ref()
            .filter(|h| h.term == term)
            .map(|h| h.range)
    }

    /// Rectangle de la première ligne du lien à l'écran, en pixels.
    fn link_anchor(&self, h: &LinkHover) -> Option<PixelRect> {
        let tab = self.model.workspace.active_tab();
        let rect = self
            .pane_rects
            .iter()
            .find(|(wid, _)| tab.term_at(*wid) == Some(h.term))
            .map(|(_, r)| *r)?;
        let metrics = self.metrics_of(h.term);
        let g = grid_geometry(rect, metrics, self.config.window.padding);
        let (row, start) = h.range.start;
        let end = if h.range.end.0 == row {
            h.range.end.1 + 1
        } else {
            g.cols
        };
        Some(PixelRect::new(
            g.origin_x + start as u32 * metrics.width,
            g.origin_y + row as u32 * metrics.height,
            end.saturating_sub(start) as u32 * metrics.width,
            metrics.height,
        ))
    }

    /// L'infobulle du lien, ouverte et sans infobulle de correction devant.
    pub(crate) fn link_chrome(&self) -> Option<Chrome> {
        let h = self.link_hover.as_ref()?;
        if !h.tooltip || self.spell.popup.is_some() {
            return None;
        }
        let anchor = self.link_anchor(h)?;
        let gesture = self.link_gesture(h.term);
        let tip = Tooltip {
            anchor,
            message: tooltip_text(&h.uri, tooltip_hint(gesture.click, gesture.captured)),
        };
        let metrics = self.metrics();
        let layout = tooltip::layout(&tip, 0, tip.message.width(), self.surface.size(), metrics);
        Some(tooltip::chrome(&tip, &layout, &self.palette, metrics))
    }

    /// Appui gauche sur la cellule `cell` du panneau `term`. Vrai si le lien
    /// capte l'appui (mode ctrl armé) : ni sélection ni envoi au programme.
    pub(crate) fn link_press(
        &mut self,
        term: TermId,
        cell: Option<(usize, usize)>,
        captured: bool,
    ) -> bool {
        let run: Option<LinkUnderMouse> = self
            .link_under(term, cell)
            .map(|(id, range, _)| (term, id, range));
        let gesture = LinkGesture {
            click: self.config.links.click,
            captured,
        };
        let armed = gesture_armed(gesture, self.modifiers);
        self.link_click
            .press(run, armed, gesture.click == ClickMode::Ctrl)
    }

    /// Relâche gauche : le lien à ouvrir si c'est celui de l'appui, sans
    /// glisser. `term` est le panneau sous la souris, s'il y en a un.
    pub(crate) fn link_release(
        &mut self,
        term: Option<TermId>,
        cell: Option<(usize, usize)>,
    ) -> Option<Effect> {
        let under = term.and_then(|t| self.link_under(t, cell).map(|l| (t, l)));
        let run = under.as_ref().map(|(t, (id, range, _))| (*t, *id, *range));
        let selection_empty = self.selection.as_ref().is_none_or(|(_, s)| s.is_empty());
        self.link_click
            .release(run, selection_empty, Instant::now())?;
        under.map(|(_, (_, _, uri))| Effect::OpenLink(uri))
    }

    /// Les modificateurs ont changé : le survol dépend d'eux sous capture,
    /// le curseur partout.
    pub(crate) fn link_modifiers_changed(&mut self) {
        if !self.cursor_inside {
            return;
        }
        let (x, y) = self.cursor;
        if let Some((term, rect)) = self.pane_under(x, y) {
            let cell = self.cell_under(term, rect, x, y);
            if self.link_hover(term, cell, &mut None) {
                self.window.request_redraw();
            }
        }
        self.update_cursor_icon(x, y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tooltip_is_due_after_the_delay() {
        let t0 = Instant::now();
        let h = LinkHover {
            term: TermId(1),
            id: LinkId::new(1).unwrap(),
            range: CellRange {
                start: (0, 0),
                end: (0, 1),
            },
            uri: "https://x".into(),
            since: t0,
            tooltip: false,
        };
        assert!(!h.tooltip_due(t0 + TOOLTIP_DELAY / 2));
        assert!(h.tooltip_due(t0 + TOOLTIP_DELAY));
    }
}
