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

use crate::tab::TermId;

pub use client::{LookupError, LookupKey, Suggestion, spawn_lookup};
pub use popup::{PopupLayout, SpellPopup, chrome as popup_chrome, layout as popup_layout};
pub use run::{UnderlinedRun, apply_token, occurrence_index, underlined_run};

/// Après un serveur injoignable, pas de nouvelle requête pendant ce délai.
const RETRY_AFTER: Duration = Duration::from_secs(30);

/// L'état du survol : le mot en cours de consultation, l'infobulle ouverte,
/// et le dernier échec du serveur (quand, pourquoi).
#[derive(Default)]
pub struct SpellState {
    /// Le mot sous la souris, requête partie ou infobulle ouverte.
    pub hover: Option<LookupKey>,
    pub popup: Option<SpellPopup>,
    /// Dernière cellule examinée : la souris qui bouge dans une même cellule
    /// ne relit pas la grille.
    pub last_cell: Option<(TermId, usize, usize)>,
    failure: Option<(Instant, LookupError)>,
}

impl SpellState {
    /// Le serveur peut être interrogé : pas d'échec récent.
    pub fn may_query(&self, now: Instant) -> bool {
        self.failure
            .as_ref()
            .is_none_or(|(since, _)| now.duration_since(*since) >= RETRY_AFTER)
    }

    /// Le dernier échec, récent, est un serveur muet : le survol le dit sans
    /// réinterroger (ce qui coûterait le délai entier à chaque mot).
    pub fn silent_recently(&self, now: Instant) -> bool {
        !self.may_query(now)
            && self
                .failure
                .as_ref()
                .is_some_and(|(_, e)| *e == LookupError::Silent)
    }

    pub fn mark_failed(&mut self, now: Instant, error: LookupError) {
        self.failure = Some((now, error));
    }

    pub fn mark_reachable(&mut self) {
        self.failure = None;
    }
}

/// Le message de l'infobulle quand le serveur écoute sans répondre.
pub fn silent_message(url: &str) -> String {
    let host = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/');
    format!("LanguageTool ne répond pas sur {host} : serveur gelé ?")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreachable_server_is_not_queried_again_for_a_while() {
        let mut s = SpellState::default();
        let t0 = Instant::now();
        assert!(s.may_query(t0));
        s.mark_failed(t0, LookupError::Unreachable("refus".into()));
        assert!(!s.may_query(t0 + Duration::from_secs(10)));
        assert!(!s.silent_recently(t0 + Duration::from_secs(10)));
        assert!(s.may_query(t0 + RETRY_AFTER));
        s.mark_reachable();
        assert!(s.may_query(t0));
    }

    #[test]
    fn a_silent_server_is_reported_without_a_new_query_until_the_retry() {
        let mut s = SpellState::default();
        let t0 = Instant::now();
        s.mark_failed(t0, LookupError::Silent);
        assert!(!s.may_query(t0 + Duration::from_secs(10)));
        assert!(s.silent_recently(t0 + Duration::from_secs(10)));
        assert!(!s.silent_recently(t0 + RETRY_AFTER), "on réinterroge");
        s.mark_reachable();
        assert!(!s.silent_recently(t0));
    }

    #[test]
    fn silent_message_names_the_host_without_scheme() {
        assert_eq!(
            silent_message("http://127.0.0.1:8099/"),
            "LanguageTool ne répond pas sur 127.0.0.1:8099 : serveur gelé ?"
        );
    }
}
