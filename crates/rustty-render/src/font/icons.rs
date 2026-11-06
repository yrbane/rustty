//! Police d'icônes de la barre de titre : Material Symbols Rounded réduite
//! aux sept glyphes utilisés (voir `scripts/material-icons.sh`). Face dédiée,
//! chargée à la taille des icônes et jamais mêlée aux replis système.

use std::sync::Arc;

use super::loader::{FaceData, glyph_in};

/// Material Symbols Rounded, poids 400, sans remplissage, sept glyphes.
pub const ICON_FONT: &[u8] = include_bytes!("../../assets/MaterialSymbolsRounded-subset.ttf");

/// Les codepoints Material des icônes, dans la zone à usage privé.
pub mod glyphs {
    pub const SETTINGS: char = '\u{e8b8}';
    pub const PALETTE: char = '\u{e40a}';
    pub const FOLDER_OPEN: char = '\u{e2c8}';
    pub const REMOVE: char = '\u{e15b}';
    pub const CROP_SQUARE: char = '\u{e3c6}';
    pub const FILTER_NONE: char = '\u{e3e0}';
    pub const CLOSE: char = '\u{e5cd}';
    pub const ALL: [char; 7] = [
        SETTINGS,
        PALETTE,
        FOLDER_OPEN,
        REMOVE,
        CROP_SQUARE,
        FILTER_NONE,
        CLOSE,
    ];
}

/// La face d'icônes à une taille donnée (pixels physiques).
pub struct IconFace {
    face: FaceData,
    size_px: f32,
}

impl IconFace {
    /// Police embarquée : toujours valide.
    pub fn new(size_px: f32) -> Self {
        Self {
            face: FaceData {
                data: Arc::new(ICON_FONT.to_vec()),
                index: 0,
            },
            size_px,
        }
    }

    pub fn size_px(&self) -> f32 {
        self.size_px
    }

    pub fn face(&self) -> &FaceData {
        &self.face
    }

    /// `None` hors des sept glyphes : aucun repli.
    pub fn glyph_id(&self, ch: char) -> Option<u16> {
        glyph_in(&self.face, ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_a_glyph_in_the_subset() {
        let face = IconFace::new(14.0);
        for ch in glyphs::ALL {
            assert!(face.glyph_id(ch).is_some(), "{:x}", u32::from(ch));
        }
    }

    #[test]
    fn the_subset_holds_nothing_else() {
        let face = IconFace::new(14.0);
        assert_eq!(face.glyph_id('a'), None);
        assert_eq!(face.glyph_id('\u{e000}'), None, "zone privée hors des sept");
    }
}
