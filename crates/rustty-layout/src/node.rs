//! Arbre binaire des divisions. Récursif et privé : la façade `TabLayout`
//! est la seule à le manipuler.
//!
//! Pour l'instant une seule feuille : la variante `Split` arrive en tâche 2,
//! avec ses premiers usages (clippy refuse le code mort).

use crate::geometry::{Rect, WindowId};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    Leaf(WindowId),
}

impl Node {
    /// Feuilles de gauche à droite et de haut en bas.
    pub(crate) fn leaves(&self, out: &mut Vec<WindowId>) {
        match self {
            Self::Leaf(id) => out.push(*id),
        }
    }

    pub(crate) fn contains(&self, id: WindowId) -> bool {
        match self {
            Self::Leaf(leaf) => *leaf == id,
        }
    }

    /// Rectangle de chaque feuille dans `bounds`, `gap` pixels entre panneaux.
    pub(crate) fn rects(&self, bounds: Rect, _gap: u32, out: &mut Vec<(WindowId, Rect)>) {
        match self {
            Self::Leaf(id) => out.push((*id, bounds)),
        }
    }
}
