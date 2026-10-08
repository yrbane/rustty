//! Cellule de la grille : un caractère de base et son style. Les caractères
//! combinants sont stockés à part, dans la ligne (voir `line.rs`).

use bitflags::bitflags;

use crate::color::Color;

bitflags! {
    /// Attributs de rendu et drapeaux de largeur d'une cellule.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Attrs: u16 {
        const BOLD = 1 << 0;
        const DIM = 1 << 1;
        const ITALIC = 1 << 2;
        const UNDERLINE = 1 << 3;
        const BLINK = 1 << 4;
        const INVERSE = 1 << 5;
        const HIDDEN = 1 << 6;
        const STRIKETHROUGH = 1 << 7;
        /// Première moitié d'un caractère large.
        const WIDE = 1 << 8;
        /// Seconde moitié d'un caractère large : ne se dessine pas.
        const WIDE_CONTINUATION = 1 << 9;
    }
}

/// Style courant (état SGR) ou style d'une cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

/// Une cellule de la grille.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cell {
    pub c: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self::new(' ', Style::default())
    }
}

impl Cell {
    pub const fn new(c: char, style: Style) -> Self {
        Self { c, style }
    }

    /// Cellule effacée : conserve les couleurs courantes (comportement « bce »
    /// de xterm) mais aucun attribut.
    pub const fn erased(style: Style) -> Self {
        Self::new(
            ' ',
            Style {
                fg: style.fg,
                bg: style.bg,
                attrs: Attrs::empty(),
            },
        )
    }

    pub const fn is_wide(&self) -> bool {
        self.style.attrs.contains(Attrs::WIDE)
    }

    pub const fn is_wide_continuation(&self) -> bool {
        self.style.attrs.contains(Attrs::WIDE_CONTINUATION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    #[test]
    fn default_cell_is_a_blank_space_with_default_style() {
        let c = Cell::default();
        assert_eq!(c.c, ' ');
        assert_eq!(c.style, Style::default());
        assert_eq!(c.style.fg, Color::Default);
        assert_eq!(c.style.bg, Color::Default);
        assert!(c.style.attrs.is_empty());
    }

    #[test]
    fn erased_cell_keeps_colors_but_drops_attributes() {
        let style = Style {
            fg: Color::Indexed(1),
            bg: Color::Rgb(1, 2, 3),
            attrs: Attrs::BOLD | Attrs::WIDE,
        };
        let c = Cell::erased(style);
        assert_eq!(c.c, ' ');
        assert_eq!(c.style.fg, Color::Indexed(1));
        assert_eq!(c.style.bg, Color::Rgb(1, 2, 3));
        assert!(c.style.attrs.is_empty());
    }

    #[test]
    fn wide_flags_are_reported() {
        let wide = Cell::new(
            '漢',
            Style {
                attrs: Attrs::WIDE,
                ..Style::default()
            },
        );
        let cont = Cell::new(
            ' ',
            Style {
                attrs: Attrs::WIDE_CONTINUATION,
                ..Style::default()
            },
        );
        assert!(wide.is_wide() && !wide.is_wide_continuation());
        assert!(cont.is_wide_continuation() && !cont.is_wide());
        assert!(!Cell::default().is_wide());
    }

    #[test]
    fn cell_stays_compact() {
        // Garde-fou : la grille peut contenir des millions de cellules.
        assert!(
            std::mem::size_of::<Cell>() <= 16,
            "Cell = {} octets",
            std::mem::size_of::<Cell>()
        );
    }
}
