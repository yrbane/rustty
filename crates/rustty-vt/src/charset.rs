//! Jeux de caractères G0/G1. Seul le jeu graphique DEC (lignes de boîte) est
//! pris en charge, c'est celui qu'utilisent encore les applications.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Charset {
    #[default]
    Ascii,
    DecSpecialGraphics,
}

impl Charset {
    pub fn map(self, c: char) -> char {
        match self {
            Self::Ascii => c,
            Self::DecSpecialGraphics => match c {
                '`' => '◆',
                'a' => '▒',
                'b' => '␉',
                'c' => '␌',
                'd' => '␍',
                'e' => '␊',
                'f' => '°',
                'g' => '±',
                'h' => '␤',
                'i' => '␋',
                'j' => '┘',
                'k' => '┐',
                'l' => '┌',
                'm' => '└',
                'n' => '┼',
                'o' => '⎺',
                'p' => '⎻',
                'q' => '─',
                'r' => '⎼',
                's' => '⎽',
                't' => '├',
                'u' => '┤',
                'v' => '┴',
                'w' => '┬',
                'x' => '│',
                'y' => '≤',
                'z' => '≥',
                '{' => 'π',
                '|' => '≠',
                '}' => '£',
                '~' => '·',
                _ => c,
            },
        }
    }
}

/// Les deux slots G0/G1 et celui qui est actif (SI = G0, SO = G1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Charsets {
    slots: [Charset; 2],
    active: usize,
}

impl Charsets {
    /// `ESC ( x` désigne le slot 0, `ESC ) x` le slot 1. Autre slot : ignoré.
    pub fn designate(&mut self, slot: usize, cs: Charset) {
        if let Some(s) = self.slots.get_mut(slot) {
            *s = cs;
        }
    }

    pub fn shift_in(&mut self) {
        self.active = 0;
    }

    pub fn shift_out(&mut self) {
        self.active = 1;
    }

    pub fn map(&self, c: char) -> char {
        self.slots[self.active].map(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_identity() {
        assert_eq!(Charset::Ascii.map('q'), 'q');
    }

    #[test]
    fn dec_special_graphics_maps_line_drawing() {
        let g = Charset::DecSpecialGraphics;
        assert_eq!(g.map('q'), '─');
        assert_eq!(g.map('x'), '│');
        assert_eq!(g.map('l'), '┌');
        assert_eq!(g.map('k'), '┐');
        assert_eq!(g.map('m'), '└');
        assert_eq!(g.map('j'), '┘');
        assert_eq!(g.map('n'), '┼');
        assert_eq!(g.map('a'), '▒');
        assert_eq!(g.map('A'), 'A', "les lettres hors table sont inchangées");
    }

    #[test]
    fn charsets_default_to_ascii_g0() {
        let cs = Charsets::default();
        assert_eq!(cs.map('q'), 'q');
    }

    #[test]
    fn designate_and_shift_between_g0_and_g1() {
        let mut cs = Charsets::default();
        cs.designate(1, Charset::DecSpecialGraphics);
        assert_eq!(cs.map('q'), 'q', "G1 désigné mais G0 actif");
        cs.shift_out();
        assert_eq!(cs.map('q'), '─');
        cs.shift_in();
        assert_eq!(cs.map('q'), 'q');
        cs.designate(0, Charset::DecSpecialGraphics);
        assert_eq!(cs.map('x'), '│');
        cs.designate(7, Charset::Ascii);
        assert_eq!(cs.map('x'), '│', "un slot inconnu est ignoré");
    }
}
