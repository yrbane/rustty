//! Arbre binaire des divisions. Récursif et privé : la façade `TabLayout`
//! est la seule à le manipuler.

use crate::geometry::{Axis, Rect, WindowId};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    Leaf(WindowId),
    Split {
        axis: Axis,
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

impl Node {
    /// Feuilles de gauche à droite et de haut en bas.
    pub(crate) fn leaves(&self, out: &mut Vec<WindowId>) {
        match self {
            Self::Leaf(id) => out.push(*id),
            Self::Split { first, second, .. } => {
                first.leaves(out);
                second.leaves(out);
            }
        }
    }

    pub(crate) fn contains(&self, id: WindowId) -> bool {
        match self {
            Self::Leaf(leaf) => *leaf == id,
            Self::Split { first, second, .. } => first.contains(id) || second.contains(id),
        }
    }

    /// Rectangle de chaque feuille dans `bounds`, `gap` pixels entre panneaux.
    /// Le pixel impair va au second panneau (troncature) ; des bornes plus petites que `gap`
    /// donnent des rectangles de taille nulle, jamais un dépassement.
    pub(crate) fn rects(&self, bounds: Rect, gap: u32, out: &mut Vec<(WindowId, Rect)>) {
        match self {
            Self::Leaf(id) => out.push((*id, bounds)),
            Self::Split {
                axis,
                ratio,
                first,
                second,
            } => {
                let (a, b) = split_rect(bounds, *axis, *ratio, gap);
                first.rects(a, gap, out);
                second.rects(b, gap, out);
            }
        }
    }

    /// Remplace la feuille `target` par une division dont elle est le premier
    /// panneau et `new_id` le second. Vrai si `target` a été trouvée.
    pub(crate) fn split_leaf(&mut self, target: WindowId, axis: Axis, new_id: WindowId) -> bool {
        match self {
            Self::Leaf(id) if *id == target => {
                *self = Self::Split {
                    axis,
                    ratio: 0.5,
                    first: Box::new(Self::Leaf(target)),
                    second: Box::new(Self::Leaf(new_id)),
                };
                true
            }
            Self::Leaf(_) => false,
            Self::Split { first, second, .. } => {
                first.split_leaf(target, axis, new_id) || second.split_leaf(target, axis, new_id)
            }
        }
    }
}

/// Coupe `bounds` en deux selon `axis` : le premier panneau reçoit `ratio` de
/// l'espace restant une fois `gap` retiré.
fn split_rect(bounds: Rect, axis: Axis, ratio: f32, gap: u32) -> (Rect, Rect) {
    let total = match axis {
        Axis::Vertical => bounds.width,
        Axis::Horizontal => bounds.height,
    };
    let available = total.saturating_sub(gap);
    // `ratio` est dans [0, 1] et `available` tient dans un f32 sans perte
    // visible à l'échelle d'un écran : la conversion est sûre.
    let first_size = ((available as f32) * ratio).floor().min(available as f32) as u32;
    let second_size = available - first_size;
    let gap = if available == 0 { total } else { gap };
    match axis {
        Axis::Vertical => (
            Rect::new(bounds.x, bounds.y, first_size, bounds.height),
            Rect::new(
                bounds.x + first_size + gap,
                bounds.y,
                second_size,
                bounds.height,
            ),
        ),
        Axis::Horizontal => (
            Rect::new(bounds.x, bounds.y, bounds.width, first_size),
            Rect::new(
                bounds.x,
                bounds.y + first_size + gap,
                bounds.width,
                second_size,
            ),
        ),
    }
}
