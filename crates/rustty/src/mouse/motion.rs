//! Filtre des rapports de mouvement : une application qui suit la souris veut
//! un rapport par cellule traversée, pas un par pixel. Pur.

use crate::tab::TermId;

/// Dernière cellule rapportée. Remis à zéro à chaque appui et relâchement :
/// le premier mouvement après un clic est toujours rapporté.
#[derive(Debug, Default)]
pub struct MotionFilter {
    last: Option<(TermId, (usize, usize))>,
}

impl MotionFilter {
    /// Vrai si `cell` de `term` diffère de la dernière cellule rapportée.
    pub fn should_report(&mut self, term: TermId, cell: (usize, usize)) -> bool {
        let key = (term, cell);
        let changed = self.last != Some(key);
        self.last = Some(key);
        changed
    }

    pub fn reset(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_is_reported_once_per_cell() {
        let mut f = MotionFilter::default();
        assert!(f.should_report(TermId(1), (3, 4)), "premier mouvement");
        assert!(!f.should_report(TermId(1), (3, 4)), "même cellule");
        assert!(f.should_report(TermId(1), (4, 4)), "autre cellule");
        assert!(f.should_report(TermId(2), (4, 4)), "autre terminal");
        f.reset();
        assert!(f.should_report(TermId(2), (4, 4)), "après remise à zéro");
    }
}
