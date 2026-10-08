//! Résolution des couleurs : des `Color` abstraites de l'émulation vers des
//! `Rgba` concrètes, selon la palette de la configuration.

use rustty_config::{Colors, Rgb};
use rustty_vt::{Attrs, Color, Style};

/// Couleur flottante non pré-multipliée, composantes dans [0, 1].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgb(rgb: Rgb) -> Self {
        Self::new(
            f32::from(rgb.r) / 255.0,
            f32::from(rgb.g) / 255.0,
            f32::from(rgb.b) / 255.0,
            1.0,
        )
    }

    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_u8(self) -> [u8; 4] {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        [q(self.r), q(self.g), q(self.b), q(self.a)]
    }

    /// Assombrit les composantes de couleur, pas l'alpha.
    pub fn dim(self, factor: f32) -> Self {
        Self::new(self.r * factor, self.g * factor, self.b * factor, self.a)
    }
}

/// Facteur appliqué à l'avant-plan d'une cellule « atténuée » (SGR 2).
pub const DIM_FACTOR: f32 = 0.66;

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub foreground: Rgba,
    pub background: Rgba,
    pub cursor: Rgba,
    pub selection: Rgba,
    pub ansi: [Rgba; 16],
    pub bold_is_bright: bool,
}

impl Palette {
    pub fn from_config(colors: &Colors, bold_is_bright: bool) -> Self {
        Self {
            foreground: Rgba::from_rgb(colors.foreground),
            background: Rgba::from_rgb(colors.background),
            cursor: Rgba::from_rgb(colors.cursor),
            selection: Rgba::from_rgb(colors.selection_background),
            ansi: colors.palette.map(Rgba::from_rgb),
            bold_is_bright,
        }
    }

    /// Couleur concrète ; `default` sert pour `Color::Default` (avant-plan
    /// ou arrière-plan selon l'appelant).
    pub fn resolve(&self, color: Color, default: Rgba) -> Rgba {
        match color {
            Color::Default => default,
            Color::Indexed(n @ 0..=15) => self.ansi[usize::from(n)],
            Color::Indexed(n @ 16..=231) => {
                const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
                let i = usize::from(n - 16);
                Rgba::from_rgb(Rgb::new(LEVELS[i / 36], LEVELS[(i / 6) % 6], LEVELS[i % 6]))
            }
            Color::Indexed(n) => {
                let v = 8 + 10 * (n - 232);
                Rgba::from_rgb(Rgb::new(v, v, v))
            }
            Color::Rgb(r, g, b) => Rgba::from_rgb(Rgb::new(r, g, b)),
        }
    }

    /// Avant-plan et arrière-plan effectifs d'une cellule, attributs appliqués.
    pub fn cell_colors(&self, style: &Style) -> (Rgba, Rgba) {
        let attrs = style.attrs;
        let fg_color = match style.fg {
            Color::Indexed(n @ 0..=7) if self.bold_is_bright && attrs.contains(Attrs::BOLD) => {
                Color::Indexed(n + 8)
            }
            other => other,
        };
        let mut fg = self.resolve(fg_color, self.foreground);
        let mut bg = self.resolve(style.bg, self.background);
        if attrs.contains(Attrs::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }
        if attrs.contains(Attrs::DIM) {
            fg = fg.dim(DIM_FACTOR);
        }
        if attrs.contains(Attrs::HIDDEN) {
            fg = bg;
        }
        (fg, bg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustty_config::Colors;
    use rustty_vt::{Attrs, Color, Style};

    fn palette() -> Palette {
        Palette::from_config(&Colors::default(), false)
    }

    #[test]
    fn rgba_conversions() {
        let c = Rgba::from_rgb(Rgb::new(255, 0, 128));
        assert_eq!(c.to_array(), [1.0, 0.0, 128.0 / 255.0, 1.0]);
        assert_eq!(c.to_u8(), [255, 0, 128, 255]);
        assert_eq!(c.with_alpha(0.5).a, 0.5);
        assert_eq!(c.dim(0.5).to_u8(), [128, 0, 64, 255]);
    }

    #[test]
    fn default_colors_come_from_config() {
        let p = palette();
        assert_eq!(p.background.to_u8(), [0x1e, 0x1e, 0x2e, 255]);
        assert_eq!(p.ansi[1].to_u8(), [0xf3, 0x8b, 0xa8, 255]);
        assert_eq!(p.resolve(Color::Default, p.foreground), p.foreground);
        assert_eq!(p.resolve(Color::Indexed(1), p.foreground), p.ansi[1]);
    }

    #[test]
    fn cube_and_grayscale_indices() {
        let p = palette();
        assert_eq!(
            p.resolve(Color::Indexed(16), p.foreground).to_u8(),
            [0, 0, 0, 255]
        );
        assert_eq!(
            p.resolve(Color::Indexed(231), p.foreground).to_u8(),
            [255, 255, 255, 255]
        );
        assert_eq!(
            p.resolve(Color::Indexed(196), p.foreground).to_u8(),
            [255, 0, 0, 255],
            "16 + 36*5 = rouge pur"
        );
        assert_eq!(
            p.resolve(Color::Indexed(232), p.foreground).to_u8(),
            [8, 8, 8, 255]
        );
        assert_eq!(
            p.resolve(Color::Indexed(255), p.foreground).to_u8(),
            [238, 238, 238, 255]
        );
        assert_eq!(
            p.resolve(Color::Rgb(1, 2, 3), p.foreground).to_u8(),
            [1, 2, 3, 255]
        );
    }

    #[test]
    fn cell_colors_apply_inverse_dim_hidden_and_bold_is_bright() {
        let mut p = palette();
        let plain = Style {
            fg: Color::Indexed(1),
            bg: Color::Indexed(4),
            attrs: Attrs::empty(),
        };
        assert_eq!(p.cell_colors(&plain), (p.ansi[1], p.ansi[4]));
        let inverse = Style {
            attrs: Attrs::INVERSE,
            ..plain
        };
        assert_eq!(p.cell_colors(&inverse), (p.ansi[4], p.ansi[1]));
        let hidden = Style {
            attrs: Attrs::HIDDEN,
            ..plain
        };
        assert_eq!(p.cell_colors(&hidden), (p.ansi[4], p.ansi[4]));
        let dim = Style {
            fg: Color::Rgb(100, 100, 100),
            attrs: Attrs::DIM,
            ..plain
        };
        assert_eq!(p.cell_colors(&dim).0.to_u8(), [66, 66, 66, 255]);
        let bold = Style {
            attrs: Attrs::BOLD,
            ..plain
        };
        assert_eq!(
            p.cell_colors(&bold).0,
            p.ansi[1],
            "sans bold_is_bright, le gras ne change pas la couleur"
        );
        p.bold_is_bright = true;
        assert_eq!(p.cell_colors(&bold).0, p.ansi[9]);
        let bold_rgb = Style {
            fg: Color::Rgb(1, 2, 3),
            attrs: Attrs::BOLD,
            ..plain
        };
        assert_eq!(
            p.cell_colors(&bold_rgb).0.to_u8(),
            [1, 2, 3, 255],
            "seules les couleurs 0–7 s'éclaircissent"
        );
    }
}
