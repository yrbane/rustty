//! Aperçu d'un dépôt pendant un glisser : moitié éclairée d'un panneau,
//! contour d'un onglet, repère vertical d'insertion. Pur : des rectangles.

use rustty_layout::Side;
use rustty_render::{ChromeQuad, PixelRect, Rgba, TabRect};

use crate::drag::DropTarget;
use crate::tab::TermId;

/// Épaisseur (px) du contour d'onglet et du repère d'insertion.
pub const MARK_WIDTH: u32 = 2;
/// Opacité de la moitié éclairée, posée par-dessus le panneau visé.
pub const HALF_ALPHA: f32 = 0.35;

/// Les quads qui annoncent `target` ; `panes` porte les rectangles complets.
pub fn drop_preview(
    target: &DropTarget,
    panes: &[(TermId, PixelRect)],
    tabs: &[TabRect],
    color: Rgba,
) -> Vec<ChromeQuad> {
    let rects = match *target {
        DropTarget::None => Vec::new(),
        DropTarget::Pane { term, side } => panes
            .iter()
            .find(|(t, _)| *t == term)
            .map(|(_, r)| half(*r, side))
            .into_iter()
            .collect(),
        DropTarget::OnTab(i) => tabs.get(i).map(|t| outline(t.rect)).unwrap_or_default(),
        DropTarget::TabSlot(i) => slot_bar(tabs, i).into_iter().collect(),
    };
    let color = match target {
        DropTarget::Pane { .. } => color.with_alpha(HALF_ALPHA),
        _ => color,
    };
    rects
        .into_iter()
        .map(|rect| ChromeQuad { rect, color })
        .collect()
}

/// La moitié de `r` du côté `side` (l'éventuel pixel impair va à droite ou en bas).
fn half(r: PixelRect, side: Side) -> PixelRect {
    let (hw, hh) = (r.width / 2, r.height / 2);
    match side {
        Side::Left => PixelRect::new(r.x, r.y, hw, r.height),
        Side::Right => PixelRect::new(r.x + hw, r.y, r.width - hw, r.height),
        Side::Top => PixelRect::new(r.x, r.y, r.width, hh),
        Side::Bottom => PixelRect::new(r.x, r.y + hh, r.width, r.height - hh),
    }
}

/// Quatre traits le long des bords intérieurs de `r`.
fn outline(r: PixelRect) -> Vec<PixelRect> {
    let t = MARK_WIDTH.min(r.width / 2).min(r.height / 2);
    vec![
        PixelRect::new(r.x, r.y, r.width, t),
        PixelRect::new(r.x, r.y + r.height - t, r.width, t),
        PixelRect::new(r.x, r.y + t, t, r.height - 2 * t),
        PixelRect::new(r.x + r.width - t, r.y + t, t, r.height - 2 * t),
    ]
}

/// Repère d'insertion avant l'onglet `i` : au bord du premier ou du dernier,
/// sinon au milieu de l'espace entre deux onglets.
fn slot_bar(tabs: &[TabRect], i: usize) -> Option<PixelRect> {
    let first = tabs.first()?.rect;
    let end = |r: PixelRect| r.x + r.width;
    let x = match i {
        0 => first.x,
        i if i == tabs.len() => end(tabs[i - 1].rect).saturating_sub(MARK_WIDTH),
        i if i < tabs.len() => {
            let mid = (end(tabs[i - 1].rect) + tabs[i].rect.x) / 2;
            mid.saturating_sub(MARK_WIDTH / 2)
        }
        _ => return None,
    };
    Some(PixelRect::new(x, first.y, MARK_WIDTH, first.height))
}

#[cfg(test)]
#[path = "drag_preview_tests.rs"]
mod tests;
