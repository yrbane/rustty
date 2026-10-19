//! Insertion d'une feuille d'un côté donné d'un nœud, et recherche de la
//! division parente : les opérations de l'arbre qui servent au déplacement
//! de panneaux.

use crate::geometry::{Side, SplitId, WindowId};
use crate::node::Node;

impl Node {
    /// Enveloppe ce nœud dans une division de ratio 0.5 portant `SplitId(new_id.0)` ;
    /// la nouvelle feuille se place du côté `side`.
    pub(crate) fn wrap(self, side: Side, new_id: WindowId) -> Node {
        let new = Box::new(Node::Leaf(new_id));
        let old = Box::new(self);
        let (first, second) = if side.new_is_first() {
            (new, old)
        } else {
            (old, new)
        };
        Node::Split {
            id: SplitId(new_id.0),
            axis: side.axis(),
            ratio: 0.5,
            first,
            second,
        }
    }

    /// Insère `new_id` du côté `side` de la feuille `target`. Faux si absente.
    pub(crate) fn insert_leaf(&mut self, target: WindowId, side: Side, new_id: WindowId) -> bool {
        match self {
            Self::Leaf(id) if *id == target => {
                let leaf = std::mem::replace(self, Self::Leaf(new_id));
                *self = leaf.wrap(side, new_id);
                true
            }
            Self::Leaf(_) => false,
            Self::Split { first, second, .. } => {
                first.insert_leaf(target, side, new_id) || second.insert_leaf(target, side, new_id)
            }
        }
    }

    /// La division qui contient directement la feuille `id`.
    pub(crate) fn parent_split(&self, id: WindowId) -> Option<SplitId> {
        let Self::Split {
            id: split_id,
            first,
            second,
            ..
        } = self
        else {
            return None;
        };
        let direct = |n: &Node| matches!(n, Node::Leaf(leaf) if *leaf == id);
        if direct(first) || direct(second) {
            Some(*split_id)
        } else {
            first.parent_split(id).or_else(|| second.parent_split(id))
        }
    }
}

#[cfg(test)]
#[path = "insert_tests.rs"]
mod tests;
