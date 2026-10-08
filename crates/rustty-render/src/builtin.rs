//! Glyphes dessinés par le renderer lui-même, indépendamment de la police :
//! lignes de boîte, blocs et symboles powerline, pour des bordures et une
//! barre d'onglets nettes même sans police patchée.

use crate::font::{CellMetrics, GlyphBitmap};

pub fn is_builtin(ch: char) -> bool {
    matches!(
        ch,
        '\u{2500}'
            | '\u{2502}'
            | '\u{250C}'
            | '\u{2510}'
            | '\u{2514}'
            | '\u{2518}'
            | '\u{251C}'
            | '\u{2524}'
            | '\u{252C}'
            | '\u{2534}'
            | '\u{253C}'
    ) || matches!(
        ch,
        '\u{2588}'
            | '\u{2580}'
            | '\u{2584}'
            | '\u{258C}'
            | '\u{2590}'
            | '\u{2591}'
            | '\u{2592}'
            | '\u{2593}'
    ) || ('\u{E0B0}'..='\u{E0B7}').contains(&ch)
}

/// Canevas alpha de la taille d'une cellule.
struct Canvas {
    w: u32,
    h: u32,
    alpha: Vec<u8>,
}

impl Canvas {
    fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            alpha: vec![0; (w * h) as usize],
        }
    }

    fn fill_rect(&mut self, x0: u32, y0: u32, x1: u32, y1: u32) {
        for y in y0.min(self.h)..y1.min(self.h) {
            for x in x0.min(self.w)..x1.min(self.w) {
                self.alpha[(y * self.w + x) as usize] = 255;
            }
        }
    }

    /// Remplit chaque pixel dont le centre satisfait `inside`.
    fn fill_where(&mut self, value: u8, inside: impl Fn(f32, f32) -> bool) {
        for y in 0..self.h {
            for x in 0..self.w {
                if inside(x as f32 + 0.5, y as f32 + 0.5) {
                    self.alpha[(y * self.w + x) as usize] = value;
                }
            }
        }
    }

    fn into_bitmap(self, baseline: u32) -> GlyphBitmap {
        GlyphBitmap {
            width: self.w,
            height: self.h,
            left: 0,
            top: baseline as i32,
            data: self
                .alpha
                .iter()
                .flat_map(|&a| [255, 255, 255, a])
                .collect(),
            is_color: false,
        }
    }
}

/// Bras d'une ligne de boîte, depuis le centre de la cellule.
#[derive(Clone, Copy)]
struct Arms {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

fn box_arms(ch: char) -> Option<Arms> {
    let a = |left, right, up, down| {
        Some(Arms {
            left,
            right,
            up,
            down,
        })
    };
    match ch {
        '\u{2500}' => a(true, true, false, false),
        '\u{2502}' => a(false, false, true, true),
        '\u{250C}' => a(false, true, false, true),
        '\u{2510}' => a(true, false, false, true),
        '\u{2514}' => a(false, true, true, false),
        '\u{2518}' => a(true, false, true, false),
        '\u{251C}' => a(false, true, true, true),
        '\u{2524}' => a(true, false, true, true),
        '\u{252C}' => a(true, true, false, true),
        '\u{2534}' => a(true, true, true, false),
        '\u{253C}' => a(true, true, true, true),
        _ => None,
    }
}

pub fn builtin_glyph(ch: char, metrics: CellMetrics) -> Option<GlyphBitmap> {
    let (w, h) = (metrics.width.max(1), metrics.height.max(1));
    let mut c = Canvas::new(w, h);
    let t = (w as f32 / 8.0).round().max(1.0) as u32;
    let (cx, cy) = (w / 2, h / 2);
    let (x0, y0) = (cx.saturating_sub(t / 2), cy.saturating_sub(t / 2));
    if let Some(arms) = box_arms(ch) {
        if arms.left {
            c.fill_rect(0, y0, cx + t.div_ceil(2), y0 + t);
        }
        if arms.right {
            c.fill_rect(x0, y0, w, y0 + t);
        }
        if arms.up {
            c.fill_rect(x0, 0, x0 + t, cy + t.div_ceil(2));
        }
        if arms.down {
            c.fill_rect(x0, y0, x0 + t, h);
        }
        return Some(c.into_bitmap(metrics.baseline));
    }
    let (wf, hf) = (w as f32, h as f32);
    let cyf = hf / 2.0;
    // Demi-disque : ellipse de largeur w et de hauteur h, centrée sur le bord plat.
    let ellipse =
        |x: f32, y: f32, center_x: f32| ((x - center_x) / wf).powi(2) + ((y - cyf) / cyf).powi(2);
    let line_px = t as f32;
    match ch {
        '\u{2588}' => c.fill_rect(0, 0, w, h),
        '\u{2580}' => c.fill_rect(0, 0, w, cy),
        '\u{2584}' => c.fill_rect(0, cy, w, h),
        '\u{258C}' => c.fill_rect(0, 0, cx, h),
        '\u{2590}' => c.fill_rect(cx, 0, w, h),
        '\u{2591}' => c.fill_where(64, |_, _| true),
        '\u{2592}' => c.fill_where(128, |_, _| true),
        '\u{2593}' => c.fill_where(192, |_, _| true),
        // Triangle plein vers la droite : base à gauche, pointe au milieu à droite.
        '\u{E0B0}' => c.fill_where(255, |x, y| x / wf <= 1.0 - (y - cyf).abs() / cyf),
        '\u{E0B2}' => c.fill_where(255, |x, y| (wf - x) / wf <= 1.0 - (y - cyf).abs() / cyf),
        // Chevrons : bande fine le long des deux arêtes du triangle.
        '\u{E0B1}' => c.fill_where(255, |x, y| {
            ((x / wf) - (1.0 - (y - cyf).abs() / cyf)).abs() * wf <= line_px
        }),
        '\u{E0B3}' => c.fill_where(255, |x, y| {
            (((wf - x) / wf) - (1.0 - (y - cyf).abs() / cyf)).abs() * wf <= line_px
        }),
        '\u{E0B4}' => c.fill_where(255, |x, y| ellipse(x, y, 0.0) <= 1.0),
        '\u{E0B6}' => c.fill_where(255, |x, y| ellipse(x, y, wf) <= 1.0),
        '\u{E0B5}' => {
            let inner = ((wf - line_px) / wf).powi(2);
            c.fill_where(255, |x, y| {
                let d = ellipse(x, y, 0.0);
                d <= 1.0 && d >= inner
            });
        }
        '\u{E0B7}' => {
            let inner = ((wf - line_px) / wf).powi(2);
            c.fill_where(255, |x, y| {
                let d = ellipse(x, y, wf);
                d <= 1.0 && d >= inner
            });
        }
        _ => return None,
    }
    Some(c.into_bitmap(metrics.baseline))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> CellMetrics {
        CellMetrics {
            width: 10,
            height: 20,
            baseline: 16,
            underline_y: 18,
            underline_thickness: 1,
            strike_y: 10,
        }
    }

    fn alpha(b: &GlyphBitmap, x: u32, y: u32) -> u8 {
        b.data[((y * b.width + x) * 4 + 3) as usize]
    }

    fn glyph(ch: char) -> GlyphBitmap {
        builtin_glyph(ch, metrics()).unwrap_or_else(|| panic!("{ch:?} n'est pas procédural"))
    }

    #[test]
    fn bitmap_fills_the_cell_and_sits_at_the_cell_top() {
        let b = glyph('█');
        assert_eq!((b.width, b.height, b.left, b.top), (10, 20, 0, 16));
        assert!(!b.is_color);
        assert!(b.data.chunks(4).all(|px| px == [255, 255, 255, 255]));
    }

    #[test]
    fn horizontal_and_vertical_lines_cross_the_whole_cell() {
        let h = glyph('─');
        assert!(
            (0..10).all(|x| alpha(&h, x, 10) == 255),
            "ligne au milieu, de bord à bord"
        );
        assert_eq!(alpha(&h, 5, 2), 0);
        let v = glyph('│');
        assert!((0..20).all(|y| alpha(&v, 5, y) == 255));
        assert_eq!(alpha(&v, 1, 10), 0);
    }

    #[test]
    fn corners_and_tees_only_draw_their_arms() {
        let c = glyph('┌');
        assert_eq!(alpha(&c, 0, 10), 0, "pas de bras à gauche");
        assert_eq!(alpha(&c, 9, 10), 255, "bras à droite");
        assert_eq!(alpha(&c, 5, 0), 0, "pas de bras en haut");
        assert_eq!(alpha(&c, 5, 19), 255, "bras en bas");
        let t = glyph('├');
        assert_eq!(alpha(&t, 5, 0), 255);
        assert_eq!(alpha(&t, 5, 19), 255);
        assert_eq!(alpha(&t, 9, 10), 255);
        assert_eq!(alpha(&t, 0, 10), 0);
        let x = glyph('┼');
        assert!(
            alpha(&x, 0, 10) == 255
                && alpha(&x, 9, 10) == 255
                && alpha(&x, 5, 0) == 255
                && alpha(&x, 5, 19) == 255
        );
    }

    #[test]
    fn half_blocks_and_shades() {
        let top = glyph('▀');
        assert_eq!((alpha(&top, 5, 2), alpha(&top, 5, 17)), (255, 0));
        let left = glyph('▌');
        assert_eq!((alpha(&left, 1, 10), alpha(&left, 8, 10)), (255, 0));
        assert!(glyph('░').data.chunks(4).all(|px| px[3] == 64));
        assert!(glyph('▓').data.chunks(4).all(|px| px[3] == 192));
    }

    #[test]
    fn powerline_shapes() {
        let tri = glyph('\u{E0B0}');
        assert_eq!(alpha(&tri, 0, 0), 255, "base pleine à gauche");
        assert_eq!(alpha(&tri, 9, 0), 0, "coin haut droit vide");
        assert_eq!(alpha(&tri, 9, 10), 255, "pointe au milieu à droite");
        let tri_l = glyph('\u{E0B2}');
        assert_eq!(alpha(&tri_l, 9, 0), 255);
        assert_eq!(alpha(&tri_l, 0, 0), 0);
        let disc = glyph('\u{E0B4}');
        assert_eq!(alpha(&disc, 0, 10), 255, "centre du bord plat");
        assert_eq!(alpha(&disc, 9, 0), 0, "coin vide");
        assert_eq!(
            alpha(&disc, 0, 0),
            255,
            "le bord plat est plein de haut en bas"
        );
        let ring = glyph('\u{E0B5}');
        assert_eq!(alpha(&ring, 3, 10), 0, "creux à l'intérieur");
        assert!(
            ring.data.chunks(4).any(|px| px[3] == 255),
            "contour présent"
        );
        let chevron = glyph('\u{E0B1}');
        assert!(chevron.data.chunks(4).any(|px| px[3] == 255));
        assert_eq!(alpha(&chevron, 0, 10), 0, "le chevron ne remplit pas");
    }

    #[test]
    fn unknown_characters_are_not_builtin() {
        assert!(builtin_glyph('A', metrics()).is_none());
        assert!(!is_builtin('A'));
        assert!(is_builtin('┼') && is_builtin('\u{E0B6}'));
    }
}
