//! Taille de police de chaque panneau : celle de la configuration, sauf pour
//! les panneaux zoomés. Pur ; le binaire en déduit un renderer par taille.

use std::collections::{BTreeSet, HashMap};

use crate::font_zoom::{FontChange, next_size};
use crate::tab::TermId;

/// Clé de cache d'une taille de police : dixièmes de point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SizeKey(u32);

impl SizeKey {
    pub fn of(size: f32) -> Self {
        Self((size * 10.0).round() as u32)
    }

    pub fn size(self) -> f32 {
        self.0 as f32 / 10.0
    }
}

#[derive(Clone, Debug)]
pub struct PaneFonts {
    base: f32,
    sizes: HashMap<TermId, f32>,
}

impl PaneFonts {
    pub fn new(base: f32) -> Self {
        Self {
            base,
            sizes: HashMap::new(),
        }
    }

    pub fn base(&self) -> f32 {
        self.base
    }

    pub fn size_of(&self, term: TermId) -> f32 {
        self.sizes.get(&term).copied().unwrap_or(self.base)
    }

    /// Zoome le panneau `term` ; vrai si sa taille a changé.
    pub fn apply(&mut self, term: TermId, change: FontChange) -> bool {
        let old = self.size_of(term);
        let new = next_size(old, self.base, change);
        if SizeKey::of(new) == SizeKey::of(self.base) {
            self.sizes.remove(&term);
        } else {
            self.sizes.insert(term, new);
        }
        SizeKey::of(new) != SizeKey::of(old)
    }

    pub fn forget(&mut self, term: TermId) {
        self.sizes.remove(&term);
    }

    /// Nouvelle taille de la configuration : tous les zooms sont oubliés.
    pub fn rebase(&mut self, base: f32) {
        self.base = base;
        self.sizes.clear();
    }

    /// Les tailles utilisées par les panneaux `live`, base comprise.
    pub fn keys_in_use(&self, live: impl IntoIterator<Item = TermId>) -> BTreeSet<SizeKey> {
        let mut keys = BTreeSet::from([SizeKey::of(self.base)]);
        keys.extend(live.into_iter().map(|t| SizeKey::of(self.size_of(t))));
        keys
    }
}

/// Le panneau à zoomer : celui sous la souris (si elle est dans la fenêtre,
/// sinon la dernière position est périmée), sinon le focalisé.
pub fn zoom_target(
    hovered: Option<TermId>,
    inside: bool,
    focused: Option<TermId>,
) -> Option<TermId> {
    hovered.filter(|_| inside).or(focused)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_panes_use_the_base() {
        let f = PaneFonts::new(11.0);
        assert_eq!(f.size_of(TermId(1)), 11.0);
        assert_eq!(f.base(), 11.0);
    }

    #[test]
    fn zoom_changes_only_the_target() {
        let mut f = PaneFonts::new(11.0);
        assert!(f.apply(TermId(1), FontChange::Increase));
        assert_eq!(f.size_of(TermId(1)), 12.0);
        assert_eq!(
            f.size_of(TermId(2)),
            11.0,
            "les autres panneaux gardent leur taille"
        );
    }

    #[test]
    fn reset_removes_the_override() {
        let mut f = PaneFonts::new(11.0);
        f.apply(TermId(1), FontChange::Increase);
        assert!(f.apply(TermId(1), FontChange::Reset));
        assert_eq!(f.size_of(TermId(1)), 11.0);
        assert!(
            !f.apply(TermId(1), FontChange::Reset),
            "déjà à la base : rien ne change"
        );
        f.apply(TermId(2), FontChange::Increase);
        f.apply(TermId(2), FontChange::Decrease);
        assert_eq!(
            f.keys_in_use([TermId(2)]),
            BTreeSet::from([SizeKey::of(11.0)]),
            "revenu à la base : plus de surcharge"
        );
    }

    #[test]
    fn bounds_apply_per_pane() {
        let mut f = PaneFonts::new(71.0);
        assert!(f.apply(TermId(1), FontChange::Increase));
        assert!(
            !f.apply(TermId(1), FontChange::Increase),
            "72 points au plus"
        );
        assert_eq!(f.size_of(TermId(1)), 72.0);
    }

    #[test]
    fn forget_and_rebase() {
        let mut f = PaneFonts::new(11.0);
        f.apply(TermId(1), FontChange::Increase);
        f.apply(TermId(2), FontChange::Increase);
        f.forget(TermId(1));
        assert_eq!(f.size_of(TermId(1)), 11.0);
        f.rebase(14.0);
        assert_eq!(
            f.size_of(TermId(2)),
            14.0,
            "un rechargement remet tout à la nouvelle taille"
        );
        assert_eq!(f.base(), 14.0);
    }

    #[test]
    fn keys_in_use_lists_distinct_sizes_of_live_panes() {
        let mut f = PaneFonts::new(11.0);
        f.apply(TermId(1), FontChange::Increase);
        f.apply(TermId(2), FontChange::Increase);
        f.apply(TermId(3), FontChange::Decrease);
        let keys = f.keys_in_use([TermId(1), TermId(2)]);
        assert_eq!(
            keys,
            BTreeSet::from([SizeKey::of(11.0), SizeKey::of(12.0)]),
            "le panneau 3 n'est plus vivant ; la base est toujours là"
        );
        assert_eq!(SizeKey::of(12.0).size(), 12.0);
    }

    #[test]
    fn zoom_target_prefers_the_hovered_pane() {
        assert_eq!(
            zoom_target(Some(TermId(2)), true, Some(TermId(1))),
            Some(TermId(2))
        );
        assert_eq!(zoom_target(None, true, Some(TermId(1))), Some(TermId(1)));
        assert_eq!(zoom_target(None, true, None), None);
    }

    #[test]
    fn zoom_target_ignores_a_stale_hover() {
        assert_eq!(
            zoom_target(Some(TermId(2)), false, Some(TermId(1))),
            Some(TermId(1)),
            "souris hors fenêtre : le panneau focalisé"
        );
    }
}
