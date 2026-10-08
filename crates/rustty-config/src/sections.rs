//! Les sections du fichier TOML et leurs valeurs par défaut. Chaque section
//! accepte un sous-ensemble de clés et refuse les clés inconnues.

use serde::Deserialize;

use crate::color::Rgb;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Font {
    /// Famille demandée ; repli automatique sur une police à chasse fixe du système.
    pub family: String,
    /// Taille en points.
    pub size: f32,
    /// Afficher le gras avec la couleur vive correspondante (0–7 → 8–15).
    pub bold_is_bright: bool,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            family: "monospace".into(),
            size: 11.0,
            bold_is_bright: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Window {
    /// Opacité du fond, 0.0 (transparent) à 1.0 (opaque).
    pub opacity: f32,
    /// Marge intérieure en pixels autour de la grille.
    pub padding: u32,
    /// Lignes d'historique conservées par fenêtre.
    pub scrollback_lines: usize,
    /// Demander confirmation avant de fermer une fenêtre dont le shell a des enfants.
    pub confirm_close_with_running_children: bool,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            padding: 4,
            scrollback_lines: 10_000,
            confirm_close_with_running_children: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabBarPosition {
    Top,
    Bottom,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CloseButtonStyle {
    pub foreground: Rgb,
    pub background: Rgb,
    pub hover_foreground: Rgb,
    pub hover_background: Rgb,
}

impl Default for CloseButtonStyle {
    fn default() -> Self {
        Self {
            foreground: Rgb::new(0xff, 0xff, 0xff),
            background: Rgb::new(0xd3, 0x2f, 0x2f),
            hover_foreground: Rgb::new(0xff, 0xff, 0xff),
            hover_background: Rgb::new(0xef, 0x53, 0x50),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tabs {
    pub position: TabBarPosition,
    /// Nombre d'onglets à partir duquel la barre s'affiche.
    pub min_tabs: u32,
    /// Bouton de fermeture cliquable sur chaque onglet.
    pub close_button: bool,
    /// Gabarit du titre : `{index}` et `{title}` sont remplacés.
    pub title_template: String,
    pub close_button_style: CloseButtonStyle,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            position: TabBarPosition::Top,
            min_tabs: 1,
            close_button: true,
            title_template: "{index}: {title}".into(),
            close_button_style: CloseButtonStyle::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Rgb,
    pub selection_background: Rgb,
    /// Les 16 couleurs ANSI : 0–7 normales, 8–15 vives.
    pub palette: [Rgb; 16],
}

impl Default for Colors {
    /// Thème Catppuccin Mocha.
    fn default() -> Self {
        let p = |r, g, b| Rgb::new(r, g, b);
        Self {
            foreground: p(0xcd, 0xd6, 0xf4),
            background: p(0x1e, 0x1e, 0x2e),
            cursor: p(0xf5, 0xe0, 0xdc),
            selection_background: p(0x45, 0x47, 0x5a),
            palette: [
                p(0x45, 0x47, 0x5a),
                p(0xf3, 0x8b, 0xa8),
                p(0xa6, 0xe3, 0xa1),
                p(0xf9, 0xe2, 0xaf),
                p(0x89, 0xb4, 0xfa),
                p(0xf5, 0xc2, 0xe7),
                p(0x94, 0xe2, 0xd5),
                p(0xba, 0xc2, 0xde),
                p(0x58, 0x5b, 0x70),
                p(0xf3, 0x8b, 0xa8),
                p(0xa6, 0xe3, 0xa1),
                p(0xf9, 0xe2, 0xaf),
                p(0x89, 0xb4, 0xfa),
                p(0xf5, 0xc2, 0xe7),
                p(0x94, 0xe2, 0xd5),
                p(0xa6, 0xad, 0xc8),
            ],
        }
    }
}
