//! Glisser-déposer dans la fenêtre : appui, mouvement, relâchement, Échap,
//! survol d'onglet et choix de la cible. La machine pure est dans `drag`.

use std::time::Instant;

use rustty_render::{ChromeQuad, HoverTarget, PixelRect, TabBarLayout};
use winit::keyboard::{Key, NamedKey};

use crate::drag::{Drag, DragSource, DropTarget, PaneDrop, drop_side, tab_slot};
use crate::drag_preview::drop_preview;
use crate::model::Effect;
use crate::mouse::MouseButton;
use crate::pane_header::contains;
use crate::tab::TermId;
use crate::window_state::OsWindow;
use crate::workspace::Workspace;

/// La cible sous `(x, y)` ; `panes` porte les rectangles complets des panneaux.
/// La barre d'onglets prime : un panneau vise l'onglet survolé, un onglet
/// vise l'index d'insertion. Un panneau n'est jamais sa propre cible.
pub fn pick_target(
    source: DragSource,
    x: f64,
    y: f64,
    panes: &[(TermId, PixelRect)],
    tab_bar: Option<&TabBarLayout>,
) -> DropTarget {
    if let Some(bar) = tab_bar.filter(|b| contains(b.bar, x, y)) {
        return match source {
            DragSource::Tab(_) => DropTarget::TabSlot(tab_slot(&bar.tabs, x)),
            DragSource::Pane(_) => bar
                .tabs
                .iter()
                .position(|t| contains(t.rect, x, y))
                .map_or(DropTarget::None, DropTarget::OnTab),
        };
    }
    let DragSource::Pane(dragged) = source else {
        return DropTarget::None;
    };
    match panes.iter().find(|(_, r)| contains(*r, x, y)) {
        Some((term, rect)) if *term != dragged => DropTarget::Pane {
            term: *term,
            side: drop_side(*rect, x, y),
        },
        _ => DropTarget::None,
    }
}

/// Le déplacement qu'un dépôt demande au modèle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropAction {
    Pane(TermId, PaneDrop),
    /// Onglet `from` vers l'index d'insertion `to`.
    Tab(usize, usize),
}

/// `None` : rien à déplacer (aucune cible, ou une cible d'un autre genre).
pub fn drop_action(source: DragSource, target: DropTarget) -> Option<DropAction> {
    match (source, target) {
        (DragSource::Pane(term), DropTarget::Pane { term: target, side }) => {
            Some(DropAction::Pane(term, PaneDrop::Pane { target, side }))
        }
        (DragSource::Pane(term), DropTarget::OnTab(tab)) => {
            Some(DropAction::Pane(term, PaneDrop::Tab(tab)))
        }
        (DragSource::Tab(from), DropTarget::TabSlot(to)) => Some(DropAction::Tab(from, to)),
        _ => None,
    }
}

/// La source existe encore (shell non terminé, onglet non fermé).
pub fn source_alive(source: DragSource, workspace: &Workspace) -> bool {
    match source {
        DragSource::Pane(term) => workspace.tab_of(term).is_some(),
        DragSource::Tab(tab) => tab < workspace.tabs().len(),
    }
}

impl OsWindow {
    fn drag_active(&self) -> bool {
        self.drag.as_ref().is_some_and(Drag::is_active)
    }

    /// Appui gauche sur un titre de bandeau ou un onglet : amorce un glisser,
    /// sauf pendant un menu, une confirmation, un renommage ou un glisser de barre.
    pub(crate) fn arm_drag(&mut self, source: DragSource, x: f64, y: f64) {
        if self.menu.is_some()
            || !self.model.accepts_drag()
            || self.divider_drag.is_some()
            || self.model.workspace.is_empty()
        {
            return;
        }
        let start = self.model.workspace.active();
        self.drag = Some(Drag::press(source, start, self.tab_signature(), x, y));
    }

    /// Appui sur la barre : un onglet pressé au bouton gauche peut être tiré.
    pub(crate) fn arm_tab_drag(
        &mut self,
        on_bar: HoverTarget,
        button: MouseButton,
        x: f64,
        y: f64,
    ) {
        if let (HoverTarget::Tab(tab), MouseButton::Left) = (on_bar, button) {
            self.arm_drag(DragSource::Tab(tab), x, y);
        }
    }

    /// Autre appui pendant un glisser actif : il l'annule et s'arrête là ;
    /// un glisser seulement amorcé est oublié et l'appui suit son cours.
    pub(crate) fn drag_press(&mut self) -> Option<Vec<Effect>> {
        if self.drag_active() {
            return Some(self.cancel_drag());
        }
        self.drag = None;
        None
    }

    /// Mouvement : franchit le seuil puis suit la cible. `None` sans glisser
    /// actif ; sinon rien d'autre (sélection, rapport au terminal) ne réagit.
    pub(crate) fn drag_moved(&mut self, x: f64, y: f64) -> Option<Vec<Effect>> {
        let drag = self.drag.as_mut()?;
        if !drag.is_active() {
            if !drag.moved(x, y) {
                return None;
            }
            // Le seuil est franchi : ce n'est plus un clic sur l'onglet,
            // ni la première moitié d'un double clic sur un bandeau.
            self.header_clicks.reset();
            self.press_target = HoverTarget::None;
            self.update_cursor_icon(x, y);
        }
        let target = pick_target(
            self.drag.as_ref()?.source(),
            x,
            y,
            &self.full_pane_rects(),
            self.tab_bar.as_ref(),
        );
        let drag = self.drag.as_mut()?;
        let changed = drag.target() != target;
        drag.set_target(target, Instant::now());
        Some(if changed {
            vec![Effect::Redraw]
        } else {
            Vec::new()
        })
    }

    /// Relâchement gauche : exécute le dépôt. `None` si aucun glisser n'a
    /// franchi le seuil (le clic garde son effet habituel).
    pub(crate) fn drag_released(&mut self, button: MouseButton) -> Option<Vec<Effect>> {
        if button != MouseButton::Left {
            return self.drag_active().then(Vec::new);
        }
        let drag = self.drag.take()?;
        let start = drag.start_tab();
        let (source, target) = drag.finish()?;
        let mut effects = match drop_action(source, target) {
            Some(DropAction::Pane(term, drop)) => self.model.move_pane(term, drop),
            Some(DropAction::Tab(from, to)) => self.model.move_tab(from, to),
            None => Vec::new(),
        };
        if effects.is_empty() {
            effects = self.restore_start_tab(start);
        }
        effects.push(Effect::Redraw);
        self.update_cursor_icon(self.cursor.0, self.cursor.1);
        Some(effects)
    }

    /// Échap pendant un glisser actif l'annule ; la touche n'atteint pas le terminal.
    pub(crate) fn drag_key(&mut self, key: &Key) -> Option<Vec<Effect>> {
        (self.drag_active() && *key == Key::Named(NamedKey::Escape)).then(|| self.cancel_drag())
    }

    /// Abandon sans dépôt : l'onglet de départ revient si un survol l'a changé.
    pub(crate) fn cancel_drag(&mut self) -> Vec<Effect> {
        let Some(drag) = self.drag.take() else {
            return Vec::new();
        };
        if !drag.is_active() {
            return Vec::new();
        }
        let mut effects = self.restore_start_tab(drag.start_tab());
        effects.push(Effect::Redraw);
        self.update_cursor_icon(self.cursor.0, self.cursor.1);
        effects
    }

    fn restore_start_tab(&mut self, start: usize) -> Vec<Effect> {
        let ws = &mut self.model.workspace;
        if start < ws.tabs().len() && ws.active() != start {
            ws.activate(start);
            vec![Effect::Relayout]
        } else {
            Vec::new()
        }
    }

    /// Réveil de la boucle : le survol d'un onglet a assez duré, on l'affiche.
    pub(crate) fn drag_tick(&mut self, now: Instant) -> Vec<Effect> {
        let Some(drag) = self.drag.as_mut() else {
            return Vec::new();
        };
        let Some(tab) = drag.due_tab_switch(now) else {
            return Vec::new();
        };
        drag.mark_switched();
        if tab >= self.model.workspace.tabs().len() || tab == self.model.workspace.active() {
            return Vec::new();
        }
        self.model.workspace.activate(tab);
        vec![Effect::Relayout]
    }

    /// Mise en page : un glisser dont la source a disparu s'arrête sans dépôt.
    pub(crate) fn drop_vanished_drag(&mut self) {
        let Some(drag) = &self.drag else { return };
        if drag.tabs_changed(&self.tab_signature()) {
            // Les indices ne désignent plus les mêmes onglets : ni dépôt ni retour à l'onglet de départ.
            self.drag = None;
            self.update_cursor_icon(self.cursor.0, self.cursor.1);
            return;
        }
        if source_alive(drag.source(), &self.model.workspace) {
            return;
        }
        // La mise en page qui nous appelle prend en compte l'onglet rendu.
        let _ = self.cancel_drag();
    }

    /// Un terminal par onglet (le plus ancien) : change dès qu'un onglet apparaît, disparaît ou bouge.
    fn tab_signature(&self) -> Vec<TermId> {
        self.model
            .workspace
            .tabs()
            .iter()
            .filter_map(|t| t.terms().into_iter().min())
            .collect()
    }

    /// Rectangles complets (bandeau compris) des panneaux de l'onglet affiché.
    fn full_pane_rects(&self) -> Vec<(TermId, PixelRect)> {
        if self.model.workspace.is_empty() {
            return Vec::new();
        }
        let tab = self.model.workspace.active_tab();
        self.pane_rects
            .iter()
            .filter_map(|(wid, _)| tab.term_at(*wid))
            .filter_map(|term| Some((term, self.full_pane_rect(term)?)))
            .collect()
    }

    /// Aperçu du dépôt visé, dessiné au-dessus des panneaux et sous le menu.
    pub(crate) fn drag_preview_quads(&self) -> Vec<ChromeQuad> {
        let Some(drag) = self.drag.as_ref().filter(|d| d.is_active()) else {
            return Vec::new();
        };
        let tabs = self.tab_bar.as_ref().map_or(&[][..], |b| &b.tabs[..]);
        drop_preview(
            &drag.target(),
            &self.full_pane_rects(),
            tabs,
            self.palette.cursor,
        )
    }
}

#[cfg(test)]
#[path = "drag_input_tests.rs"]
mod tests;
