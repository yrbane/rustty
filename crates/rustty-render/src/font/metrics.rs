//! Géométrie d'une cellule déduite d'une police à une taille donnée.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellMetrics {
    pub width: u32,
    pub height: u32,
    /// Distance du haut de la cellule à la ligne de base.
    pub baseline: u32,
    /// Ligne du soulignement, depuis le haut de la cellule.
    pub underline_y: u32,
    pub underline_thickness: u32,
    /// Ligne du barré, depuis le haut de la cellule.
    pub strike_y: u32,
}

impl CellMetrics {
    pub fn from_font(font: &swash::FontRef<'_>, size_px: f32) -> Self {
        let m = font.metrics(&[]).scale(size_px);
        let gid = font.charmap().map('0');
        let advance = if gid != 0 {
            font.glyph_metrics(&[]).scale(size_px).advance_width(gid)
        } else {
            0.0
        };
        let advance = if advance > 0.0 {
            advance
        } else {
            m.average_width.max(m.max_width)
        };
        let width = advance.round().max(1.0) as u32;
        let height = (m.ascent + m.descent + m.leading).round().max(1.0) as u32;
        let baseline = m.ascent.round().clamp(1.0, height as f32) as u32;
        // Bornes calculées avant le clamp : sur une cellule d'une ligne, lo > hi
        // ferait paniquer f32::clamp.
        let hi = height.saturating_sub(1);
        let lo = (baseline + 1).min(hi);
        let underline_y =
            ((baseline as f32 - m.underline_offset).round().max(0.0) as u32).clamp(lo, hi);
        let strike_y = (baseline as f32 - m.strikeout_offset)
            .round()
            .clamp(1.0, (baseline.saturating_sub(1)).max(1) as f32) as u32;
        Self {
            width,
            height,
            baseline,
            underline_y,
            underline_thickness: m.stroke_size.round().max(1.0) as u32,
            strike_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::loader::EMBEDDED_FONT;

    #[test]
    fn metrics_of_the_embedded_font_are_sane() {
        let font = swash::FontRef::from_index(EMBEDDED_FONT, 0).unwrap();
        let m = CellMetrics::from_font(&font, 16.0);
        assert!(m.width >= 8 && m.width <= 12, "{m:?}");
        assert!(m.height >= 16 && m.height <= 22, "{m:?}");
        assert!(m.baseline > 0 && m.baseline < m.height, "{m:?}");
        assert!(
            m.underline_y > m.baseline && m.underline_y < m.height,
            "{m:?}"
        );
        assert!(m.underline_thickness >= 1);
        assert!(m.strike_y > 0 && m.strike_y < m.baseline, "{m:?}");
    }

    #[test]
    fn metrics_scale_with_size() {
        let font = swash::FontRef::from_index(EMBEDDED_FONT, 0).unwrap();
        let small = CellMetrics::from_font(&font, 10.0);
        let big = CellMetrics::from_font(&font, 30.0);
        assert!(big.width > small.width * 2 && big.height > small.height * 2);
    }

    #[test]
    fn tiny_size_never_yields_zero_cells() {
        let font = swash::FontRef::from_index(EMBEDDED_FONT, 0).unwrap();
        let m = CellMetrics::from_font(&font, 0.1);
        assert!(m.width >= 1 && m.height >= 1 && m.underline_y < m.height);
    }
}
