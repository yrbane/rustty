//! Arbre binaire des divisions. Récursif et privé : la façade `TabLayout`
//! est la seule à le manipuler.

use crate::geometry::{Axis, Rect, SplitId, WindowId};

/// Un panneau ne peut pas descendre sous 10 % de l'espace de sa division.
pub const MIN_RATIO: f32 = 0.1;
pub const MAX_RATIO: f32 = 0.9;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    Leaf(WindowId),
    Split {
        id: SplitId,
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
                ..
            } => {
                let (a, b) = split_rect(bounds, *axis, *ratio, gap);
                first.rects(a, gap, out);
                second.rects(b, gap, out);
            }
        }
    }

    /// L'interstice de chaque division, de la racine vers les feuilles.
    /// Les interstices de taille nulle sont omis.
    pub(crate) fn dividers(&self, bounds: Rect, gap: u32, out: &mut Vec<(SplitId, Rect)>) {
        let Self::Split {
            id,
            axis,
            ratio,
            first,
            second,
        } = self
        else {
            return;
        };
        let (a, b) = split_rect(bounds, *axis, *ratio, gap);
        let bar = match axis {
            Axis::Vertical => {
                let x = a.x + a.width;
                Rect::new(x, bounds.y, b.x - x, bounds.height)
            }
            Axis::Horizontal => {
                let y = a.y + a.height;
                Rect::new(bounds.x, y, bounds.width, b.y - y)
            }
        };
        if bar.width > 0 && bar.height > 0 {
            out.push((*id, bar));
        }
        first.dividers(a, gap, out);
        second.dividers(b, gap, out);
    }

    /// Remplace la feuille `target` par une division dont elle est le premier
    /// panneau et `new_id` le second. Vrai si `target` a été trouvée.
    pub(crate) fn split_leaf(&mut self, target: WindowId, axis: Axis, new_id: WindowId) -> bool {
        match self {
            Self::Leaf(id) if *id == target => {
                *self = Self::Split {
                    id: SplitId(new_id.0),
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

    pub(crate) fn first_leaf(&self) -> WindowId {
        match self {
            Self::Leaf(id) => *id,
            Self::Split { first, .. } => first.first_leaf(),
        }
    }

    /// Retire la feuille `id`. Rend l'arbre restant, ou `None` si ce nœud
    /// était cette feuille. Un nœud qui ne contient pas `id` est rendu intact.
    pub(crate) fn remove(self, id: WindowId) -> Option<Node> {
        match self {
            Self::Leaf(leaf) if leaf == id => None,
            Self::Leaf(_) => Some(self),
            Self::Split {
                id: split_id,
                axis,
                ratio,
                first,
                second,
            } => {
                if first.contains(id) {
                    match first.remove(id) {
                        None => Some(*second),
                        Some(kept) => Some(Self::Split {
                            id: split_id,
                            axis,
                            ratio,
                            first: Box::new(kept),
                            second,
                        }),
                    }
                } else if second.contains(id) {
                    match second.remove(id) {
                        None => Some(*first),
                        Some(kept) => Some(Self::Split {
                            id: split_id,
                            axis,
                            ratio,
                            first,
                            second: Box::new(kept),
                        }),
                    }
                } else {
                    Some(Self::Split {
                        id: split_id,
                        axis,
                        ratio,
                        first,
                        second,
                    })
                }
            }
        }
    }
    /// Ajuste la division la plus proche de `id` ayant l'axe `axis` : `delta`
    /// positif agrandit le côté qui contient `id`. Faux si aucune ne convient.
    pub(crate) fn resize(&mut self, id: WindowId, axis: Axis, delta: f32) -> bool {
        let Self::Split {
            axis: own_axis,
            ratio,
            first,
            second,
            ..
        } = self
        else {
            return false;
        };
        let in_first = first.contains(id);
        if !in_first && !second.contains(id) {
            return false;
        }
        let child = if in_first { first } else { second };
        if child.resize(id, axis, delta) {
            return true;
        }
        if *own_axis != axis {
            return false;
        }
        let signed = if in_first { delta } else { -delta };
        *ratio = (*ratio + signed).clamp(MIN_RATIO, MAX_RATIO);
        true
    }

    /// Inverse l'axe de la division la plus proche de `id`.
    pub(crate) fn rotate(&mut self, id: WindowId) -> bool {
        let Self::Split {
            axis,
            first,
            second,
            ..
        } = self
        else {
            return false;
        };
        let in_first = first.contains(id);
        if !in_first && !second.contains(id) {
            return false;
        }
        let child = if in_first { first } else { second };
        if child.rotate(id) {
            return true;
        }
        *axis = match axis {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        };
        true
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
