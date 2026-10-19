//! Machine de glisser pure : seuil, cible de dépôt et survol d'onglet,
//! sans fenêtre ni modèle (le câblage est dans la fenêtre).
#![allow(dead_code)] // câblé par la tâche 10

use std::time::{Duration, Instant};

use rustty_layout::Side;
use rustty_render::{PixelRect, TabRect};

use crate::tab::TermId;

/// Distance (px) à partir de laquelle un appui devient un glisser.
pub const DRAG_THRESHOLD: f64 = 4.0;
/// Durée de survol d'un onglet avant de l'afficher.
pub const TAB_HOVER_DELAY: Duration = Duration::from_millis(500);

/// Ce que l'on tire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragSource {
    Pane(TermId),
    Tab(usize),
}

/// Où l'on déposerait en ce moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropTarget {
    None,
    Pane { term: TermId, side: Side },
    OnTab(usize),
    TabSlot(usize),
}

/// Dépôt d'un panneau, tel que le modèle l'exécute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneDrop {
    Pane { target: TermId, side: Side },
    Tab(usize),
    NewTab,
}

/// Côté du rectangle visé : les deux diagonales le découpent en quatre zones.
pub fn drop_side(rect: PixelRect, x: f64, y: f64) -> Side {
    let dx = (x - f64::from(rect.x)) / f64::from(rect.width.max(1)) - 0.5;
    let dy = (y - f64::from(rect.y)) / f64::from(rect.height.max(1)) - 0.5;
    match (dx.abs() > dy.abs(), dx < 0.0, dy < 0.0) {
        (true, true, _) => Side::Left,
        (true, false, _) => Side::Right,
        (false, _, true) => Side::Top,
        (false, _, false) => Side::Bottom,
    }
}

/// Index d'insertion d'un onglet lâché en `x`, d'après le milieu de chaque onglet.
pub fn tab_slot(tabs: &[TabRect], x: f64) -> usize {
    tabs.iter()
        .take_while(|t| x >= f64::from(t.rect.x) + f64::from(t.rect.width) / 2.0)
        .count()
}

/// Un glisser en cours : « amorcé » après l'appui, actif après le seuil.
#[derive(Clone, Debug)]
pub struct Drag {
    source: DragSource,
    start_tab: usize,
    origin: (f64, f64),
    active: bool,
    target: DropTarget,
    /// Onglet survolé et début du survol.
    hover: Option<(usize, Instant)>,
    switched: bool,
}

impl Drag {
    pub fn press(source: DragSource, start_tab: usize, x: f64, y: f64) -> Self {
        Self {
            source,
            start_tab,
            origin: (x, y),
            active: false,
            target: DropTarget::None,
            hover: None,
            switched: false,
        }
    }

    /// Vrai quand le seuil vient d'être franchi.
    pub fn moved(&mut self, x: f64, y: f64) -> bool {
        if self.active {
            return false;
        }
        let (dx, dy) = (x - self.origin.0, y - self.origin.1);
        self.active = dx.hypot(dy) >= DRAG_THRESHOLD;
        self.active
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn target(&self) -> DropTarget {
        self.target
    }

    /// Un `OnTab(i)` démarre (ou poursuit) le survol ; toute autre cible le remet à zéro.
    pub fn set_target(&mut self, target: DropTarget, now: Instant) {
        self.target = target;
        match target {
            DropTarget::OnTab(i) => {
                if self.hover.map(|(tab, _)| tab) != Some(i) {
                    self.hover = Some((i, now));
                    self.switched = false;
                }
            }
            _ => {
                self.hover = None;
                self.switched = false;
            }
        }
    }

    /// Onglet à afficher quand le survol a duré `TAB_HOVER_DELAY` (une fois par survol).
    pub fn due_tab_switch(&self, now: Instant) -> Option<usize> {
        match self.hover {
            Some((tab, since)) if !self.switched && now >= since + TAB_HOVER_DELAY => Some(tab),
            _ => None,
        }
    }

    /// Note que l'onglet survolé a été affiché : plus d'échéance pour ce survol.
    pub fn mark_switched(&mut self) {
        self.switched = true;
    }

    /// Échéance du prochain changement d'onglet, pour `about_to_wait`.
    pub fn deadline(&self) -> Option<Instant> {
        match self.hover {
            Some((_, since)) if !self.switched => Some(since + TAB_HOVER_DELAY),
            _ => None,
        }
    }

    /// Source et cible du dépôt ; `None` si le seuil n'a jamais été franchi.
    pub fn finish(self) -> Option<(DragSource, DropTarget)> {
        self.active.then_some((self.source, self.target))
    }

    pub fn start_tab(&self) -> usize {
        self.start_tab
    }
}

#[cfg(test)]
#[path = "drag_tests.rs"]
mod tests;
