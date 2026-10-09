//! Couleurs d'accent tirées au sort parmi les couleurs vives de la palette,
//! pour les onglets et les barres de split. Pur et reproductible par graine.

use rustty_render::{Palette, Rgba};

/// Les couleurs vives de la palette ANSI : 1–6 et 9–14 (ni noir, ni gris, ni blanc).
pub const ACCENT_SLOTS: [usize; 12] = [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14];

/// Tirage xorshift64 : déterministe pour une graine, sans dépendance.
#[derive(Clone, Debug)]
pub struct AccentPicker {
    state: u64,
    last: Option<usize>,
}

impl AccentPicker {
    pub fn new(seed: u64) -> Self {
        // Mélange de la graine ; xorshift exige un état non nul.
        let state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        Self { state, last: None }
    }

    /// Graine tirée de l'horloge : une session ne ressemble pas à la précédente.
    pub fn from_clock() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Self::new(nanos)
    }

    fn step(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Un accent dans `0..ACCENT_SLOTS.len()`, jamais deux fois de suite le même.
    pub fn next(&mut self) -> usize {
        let len = ACCENT_SLOTS.len() as u64;
        let mut pick = (self.step() % len) as usize;
        if Some(pick) == self.last {
            pick = (pick + 1 + (self.step() % (len - 1)) as usize) % ACCENT_SLOTS.len();
        }
        self.last = Some(pick);
        pick
    }
}

pub fn accent_color(palette: &Palette, accent: usize) -> Rgba {
    palette.ansi[ACCENT_SLOTS[accent % ACCENT_SLOTS.len()]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustty_config::Colors;

    #[test]
    fn never_twice_in_a_row() {
        let mut p = AccentPicker::new(42);
        let mut last = p.next();
        for _ in 0..1000 {
            let next = p.next();
            assert_ne!(next, last);
            assert!(next < ACCENT_SLOTS.len());
            last = next;
        }
    }

    #[test]
    fn same_seed_same_sequence() {
        let (mut a, mut b) = (AccentPicker::new(7), AccentPicker::new(7));
        let sa: Vec<_> = (0..20).map(|_| a.next()).collect();
        let sb: Vec<_> = (0..20).map(|_| b.next()).collect();
        assert_eq!(sa, sb);
        let mut c = AccentPicker::new(8);
        let sc: Vec<_> = (0..20).map(|_| c.next()).collect();
        assert_ne!(sa, sc);
    }

    #[test]
    fn uses_many_slots() {
        let mut p = AccentPicker::new(1);
        let distinct: std::collections::BTreeSet<_> = (0..100).map(|_| p.next()).collect();
        assert!(distinct.len() >= 10, "{distinct:?}");
    }

    #[test]
    fn zero_seed_is_valid() {
        let mut p = AccentPicker::new(0);
        let a = p.next();
        assert_ne!(a, p.next());
    }

    #[test]
    fn accent_color_maps_slots_to_vivid_palette_entries() {
        let palette = Palette::from_config(&Colors::default(), false);
        assert_eq!(accent_color(&palette, 0), palette.ansi[1]);
        assert_eq!(accent_color(&palette, 6), palette.ansi[9]);
        assert_eq!(accent_color(&palette, 11), palette.ansi[14]);
        assert_eq!(accent_color(&palette, 12), palette.ansi[1], "modulo");
    }
}
