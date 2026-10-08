//! Rastérisation d'un glyphe en bitmap RGBA8 via swash : contours, contours
//! couleur (COLR) et bitmaps couleur (emoji) sont tous ramenés au même format.

use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;

use super::FaceData;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlyphBitmap {
    pub width: u32,
    pub height: u32,
    /// Décalage horizontal du bitmap par rapport à l'origine du glyphe.
    pub left: i32,
    /// Distance de la ligne de base au haut du bitmap, positive vers le haut.
    pub top: i32,
    /// RGBA8, `width × height × 4`.
    pub data: Vec<u8>,
    pub is_color: bool,
}

pub struct Rasterizer {
    ctx: ScaleContext,
}

impl Default for Rasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rasterizer {
    pub fn new() -> Self {
        Self {
            ctx: ScaleContext::new(),
        }
    }

    pub fn rasterize(
        &mut self,
        face: &FaceData,
        size_px: f32,
        glyph_id: u16,
    ) -> Option<GlyphBitmap> {
        let font = face.font_ref()?;
        let mut scaler = self.ctx.builder(font).size(size_px).hint(true).build();
        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .render(&mut scaler, glyph_id)?;
        let (width, height) = (image.placement.width, image.placement.height);
        if width == 0 || height == 0 {
            return None;
        }
        let (data, is_color) = match image.content {
            Content::Mask => (
                image
                    .data
                    .iter()
                    .flat_map(|&a| [255, 255, 255, a])
                    .collect(),
                false,
            ),
            Content::Color => (image.data.clone(), true),
            Content::SubpixelMask => (
                image
                    .data
                    .chunks(3)
                    .flat_map(|rgb| [255, 255, 255, rgb.iter().copied().max().unwrap_or(0)])
                    .collect(),
                false,
            ),
        };
        Some(GlyphBitmap {
            width,
            height,
            left: image.placement.left,
            top: image.placement.top,
            data,
            is_color,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{FontSet, Variant};

    fn glyph_of(set: &mut FontSet, ch: char) -> (FaceData, u16) {
        let g = set
            .glyph(ch, Variant::Regular)
            .unwrap_or_else(|| panic!("{ch:?} introuvable"));
        (set.face_by_slot(g.slot).clone(), g.glyph_id)
    }

    #[test]
    fn letter_a_has_ink_inside_the_cell() {
        let mut set = FontSet::embedded(16.0);
        let (face, gid) = glyph_of(&mut set, 'A');
        let bmp = Rasterizer::new().rasterize(&face, 16.0, gid).unwrap();
        assert!(bmp.width > 0 && bmp.height > 0);
        assert_eq!(bmp.data.len() as u32, bmp.width * bmp.height * 4);
        assert!(!bmp.is_color);
        let ink: u32 = bmp.data.chunks(4).map(|px| u32::from(px[3])).sum();
        assert!(ink > 255 * 20, "trop peu d'encre : {ink}");
        assert!(
            bmp.data
                .chunks(4)
                .all(|px| px[0] == 255 && px[1] == 255 && px[2] == 255),
            "monochrome = blanc + alpha"
        );
        let m = set.metrics();
        assert!(
            bmp.width <= m.width + 2 && bmp.height <= m.height + 2,
            "le A tient dans la cellule : {bmp:?} vs {m:?}"
        );
        assert!(
            bmp.top > 0 && bmp.top as u32 <= m.baseline,
            "au-dessus de la ligne de base"
        );
    }

    #[test]
    fn space_has_no_image() {
        let mut set = FontSet::embedded(16.0);
        let (face, gid) = glyph_of(&mut set, ' ');
        assert!(Rasterizer::new().rasterize(&face, 16.0, gid).is_none());
    }

    #[test]
    fn missing_glyph_is_none_not_panic() {
        let mut set = FontSet::embedded(16.0);
        // Un caractère de contrôle peut mapper vers .notdef ou vers rien : ce qui
        // compte est l'absence de panique et un bitmap exploitable ou None.
        let _ = set.glyph('\u{1}', Variant::Regular);
        let r = set.glyph('\u{10FFFF}', Variant::Bold);
        if let Some(g) = r {
            let face = set.face_by_slot(g.slot).clone();
            let _ = Rasterizer::new().rasterize(&face, 16.0, g.glyph_id);
        }
    }

    #[test]
    fn replacement_character_comes_from_the_embedded_font() {
        let mut set = FontSet::embedded(16.0);
        let g = set
            .glyph('\u{FFFD}', Variant::Regular)
            .expect("DejaVu couvre U+FFFD");
        assert!(g.slot < 4);
    }

    #[test]
    fn coverage_fallback_finds_another_face_or_none() {
        // Sur un système avec une police CJK ou emoji, le slot est ≥ 4 ; sinon None. Jamais de panique.
        let mut set = FontSet::load("Police-Inexistante-Rustty", 16.0).unwrap();
        for ch in ['漢', '😀'] {
            match set.glyph(ch, Variant::Regular) {
                Some(g) => {
                    let face = set.face_by_slot(g.slot).clone();
                    let bmp = Rasterizer::new().rasterize(&face, 16.0, g.glyph_id);
                    assert!(bmp.is_none() || bmp.unwrap().width > 0);
                }
                None => eprintln!("aucune police système ne couvre {ch:?}"),
            }
        }
        let first = set.glyph('漢', Variant::Regular);
        let second = set.glyph('漢', Variant::Regular);
        assert_eq!(
            first.map(|g| g.slot),
            second.map(|g| g.slot),
            "résultat mémorisé"
        );
    }
}
