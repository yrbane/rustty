//! Hyperliens : logique pure (plage d'un lien, geste d'ouverture, texte
//! d'infobulle), sans fenêtre ni horloge.
// Câblé dans la fenêtre à la tâche suivante.
#![allow(dead_code)]

use std::time::Instant;

use rustty_config::LinkClick as ClickMode;
use rustty_config::keys::Mods;
use rustty_render::CellRange;
use rustty_vt::{Line, LinkId};
use unicode_width::UnicodeWidthChar;

use crate::mouse::click::DOUBLE_CLICK;
use crate::tab::TermId;

/// Largeur maximale de la cible dans l'infobulle, en cellules.
const TOOLTIP_CELLS: usize = 60;

/// Les cellules contiguës portant le même lien que `(row, col)`, en
/// traversant les lignes repliées (`wrapped`) dans les deux sens.
pub fn link_run(lines: &[Line], row: usize, col: usize) -> Option<(LinkId, CellRange)> {
    let id = lines.get(row)?.cells().get(col)?.link?;
    let same = |r: usize, c: usize| {
        lines[r]
            .cells()
            .get(c)
            .is_some_and(|cell| cell.link == Some(id))
    };
    let (mut sr, mut sc) = (row, col);
    loop {
        if sc > 0 && same(sr, sc - 1) {
            sc -= 1;
        } else if sc == 0
            && sr > 0
            && lines[sr - 1].wrapped
            && same(sr - 1, lines[sr - 1].len() - 1)
        {
            sr -= 1;
            sc = lines[sr].len() - 1;
        } else {
            break;
        }
    }
    let (mut er, mut ec) = (row, col);
    loop {
        if ec + 1 < lines[er].len() && same(er, ec + 1) {
            ec += 1;
        } else if ec + 1 >= lines[er].len()
            && lines[er].wrapped
            && er + 1 < lines.len()
            && same(er + 1, 0)
        {
            er += 1;
            ec = 0;
        } else {
            break;
        }
    }
    Some((
        id,
        CellRange {
            start: (sr, sc),
            end: (er, ec),
        },
    ))
}

/// Ce qu'il faut tenir enfoncé pour qu'un clic ouvre un lien.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkGesture {
    pub click: ClickMode,
    /// L'application capture la souris : shift en plus.
    pub captured: bool,
}

/// Le geste est-il armé avec ces modificateurs ?
pub fn gesture_armed(g: LinkGesture, mods: Mods) -> bool {
    let shift_ok = !g.captured || mods.contains(Mods::SHIFT);
    let ctrl_ok = g.click == ClickMode::Plain || mods.contains(Mods::CTRL);
    shift_ok && ctrl_ok
}

/// Un lien sous la souris : panneau, identifiant et plage.
pub type LinkUnderMouse = (TermId, LinkId, CellRange);

/// Appui puis relâche sur un même lien, sans glisser, une seule fois par
/// double clic.
#[derive(Debug, Default)]
pub struct LinkClick {
    armed: Option<LinkUnderMouse>,
    last_open: Option<(TermId, LinkId, Instant)>,
}

impl LinkClick {
    /// Appui gauche sur `run` ; `armed` = geste tenu ; `ctrl_mode` = le clic
    /// configuré exige ctrl. Vrai si l'appui est capté (pas de sélection) :
    /// en mode ctrl, un appui armé sur un lien ne pose pas le curseur.
    pub fn press(&mut self, run: Option<LinkUnderMouse>, armed: bool, ctrl_mode: bool) -> bool {
        self.armed = run.filter(|_| armed);
        self.armed.is_some() && ctrl_mode
    }

    /// Relâche gauche sur `run` : l'identifiant à ouvrir si c'est le lien
    /// de l'appui armé, sans sélection, et pas une seconde relâche sur ce
    /// lien dans le délai de double clic.
    pub fn release(
        &mut self,
        run: Option<LinkUnderMouse>,
        selection_empty: bool,
        now: Instant,
    ) -> Option<LinkId> {
        let armed = self.armed.take()?;
        let run = run?;
        if run != armed || !selection_empty {
            return None;
        }
        let (term, id, _) = run;
        let repeat = self.last_open.is_some_and(|(t, i, at)| {
            t == term && i == id && now.saturating_duration_since(at) <= DOUBLE_CLICK
        });
        if repeat {
            return None;
        }
        self.last_open = Some((term, id, now));
        Some(id)
    }

    pub fn reset(&mut self) {
        self.armed = None;
    }
}

/// Ce que l'infobulle ajoute au lien pour dire comment l'ouvrir.
pub fn tooltip_hint(click: ClickMode, captured: bool) -> Option<&'static str> {
    match (click, captured) {
        (ClickMode::Plain, false) => None,
        (ClickMode::Ctrl, false) => Some("ctrl+clic"),
        (ClickMode::Plain, true) => Some("shift+clic"),
        (ClickMode::Ctrl, true) => Some("shift+ctrl+clic"),
    }
}

/// La cible tronquée au milieu à `TOOLTIP_CELLS` cellules (le domaine et
/// la fin restent lisibles), puis ` · ` et l'indication du geste.
pub fn tooltip_text(uri: &str, hint: Option<&str>) -> String {
    let width: usize = uri.chars().map(|c| c.width().unwrap_or(0)).sum();
    let mut text = if width <= TOOLTIP_CELLS {
        uri.to_string()
    } else {
        let keep = TOOLTIP_CELLS - 1;
        let head: String = uri.chars().take(keep / 2).collect();
        let tail: String = uri
            .chars()
            .rev()
            .take(keep - keep / 2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{head}…{tail}")
    };
    if let Some(hint) = hint {
        text.push_str(" · ");
        text.push_str(hint);
    }
    text
}

#[cfg(test)]
#[path = "links_tests.rs"]
mod tests;
