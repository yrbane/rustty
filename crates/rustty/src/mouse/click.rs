//! Détection du double clic sur une même cible. Pur : l'horloge est passée.

use std::time::{Duration, Instant};

/// Délai maximal entre les deux clics d'un double clic.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(400);

#[derive(Debug, Default)]
pub struct DoubleClick {
    last: Option<(usize, Instant)>,
}

impl DoubleClick {
    /// Enregistre un clic sur `target` ; vrai s'il complète un double clic.
    pub fn register(&mut self, target: usize, now: Instant) -> bool {
        let double = self.last.is_some_and(|(t, at)| {
            t == target && now.saturating_duration_since(at) <= DOUBLE_CLICK
        });
        self.last = if double { None } else { Some((target, now)) };
        double
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_quick_clicks_on_the_same_tab() {
        let t0 = Instant::now();
        let mut d = DoubleClick::default();
        assert!(!d.register(1, t0));
        assert!(d.register(1, t0 + Duration::from_millis(250)));
    }

    #[test]
    fn slow_or_different_clicks_are_single() {
        let t0 = Instant::now();
        let mut d = DoubleClick::default();
        d.register(1, t0);
        assert!(!d.register(1, t0 + Duration::from_millis(500)), "trop lent");
        assert!(
            !d.register(2, t0 + Duration::from_millis(600)),
            "autre onglet"
        );
    }

    #[test]
    fn a_triple_click_is_one_double_then_single() {
        let t0 = Instant::now();
        let mut d = DoubleClick::default();
        d.register(1, t0);
        assert!(d.register(1, t0 + Duration::from_millis(100)));
        assert!(
            !d.register(1, t0 + Duration::from_millis(200)),
            "le double clic consomme le premier"
        );
    }
}
