//! Infobulle de correction au survol d'un mot souligné : ce qu'un programme a
//! souligné (le mod ortho de Claude Code), LanguageTool l'explique et propose,
//! un clic renvoie la suggestion au programme. Logique pure ; les réactions
//! aux entrées sont dans `spell_input.rs`.
//!
//! Contrat avec le mod : `docs/superpowers/specs/2026-10-30-spell-popup-design.md`.

mod client;
mod popup;
mod run;

use std::time::{Duration, Instant};

pub use client::{LookupKey, Suggestion, spawn_lookup};
pub use popup::{PopupLayout, SpellPopup, chrome as popup_chrome, layout as popup_layout};
pub use run::{UnderlinedRun, apply_token, occurrence_index, underlined_run};

/// Après un serveur injoignable, pas de nouvelle requête pendant ce délai.
const RETRY_AFTER: Duration = Duration::from_secs(30);

/// L'état du survol : le mot en cours de consultation, l'infobulle ouverte,
/// et depuis quand le serveur ne répond pas.
#[derive(Default)]
pub struct SpellState {
    /// Le mot sous la souris, requête partie ou infobulle ouverte.
    pub hover: Option<LookupKey>,
    pub popup: Option<SpellPopup>,
    unreachable_since: Option<Instant>,
}

impl SpellState {
    /// Le serveur peut être interrogé : pas d'échec récent.
    pub fn may_query(&self, now: Instant) -> bool {
        self.unreachable_since
            .is_none_or(|since| now.duration_since(since) >= RETRY_AFTER)
    }

    pub fn mark_unreachable(&mut self, now: Instant) {
        self.unreachable_since = Some(now);
    }

    pub fn mark_reachable(&mut self) {
        self.unreachable_since = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreachable_server_is_not_queried_again_for_a_while() {
        let mut s = SpellState::default();
        let t0 = Instant::now();
        assert!(s.may_query(t0));
        s.mark_unreachable(t0);
        assert!(!s.may_query(t0 + Duration::from_secs(10)));
        assert!(s.may_query(t0 + RETRY_AFTER));
        s.mark_reachable();
        assert!(s.may_query(t0));
    }
}
