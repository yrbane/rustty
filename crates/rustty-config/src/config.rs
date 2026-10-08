//! La configuration complète : analyse, validation.

use serde::Deserialize;

use crate::error::ConfigError;
use crate::sections::{Colors, Font, Tabs, Window};

/// Plafond de lignes d'historique par fenêtre : au-delà, la mémoire explose
/// avant que l'utilisateur ne s'en serve.
pub const MAX_SCROLLBACK_LINES: usize = 1_000_000;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub font: Font,
    pub window: Window,
    pub tabs: Tabs,
    pub colors: Colors,
}

impl Config {
    /// Analyse un document TOML puis le valide.
    // Pas FromStr : l'erreur porte un contexte riche et la méthode valide en plus d'analyser.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(toml: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(toml).map_err(|e| ConfigError::from_toml(toml, &e))?;
        config.validate()?;
        Ok(config)
    }

    /// Bornes que le typage ne garantit pas.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let invalid = |field, reason: String| Err(ConfigError::Invalid { field, reason });
        if !(0.0..=1.0).contains(&self.window.opacity) {
            return invalid(
                "window.opacity",
                format!("{} n'est pas entre 0.0 et 1.0", self.window.opacity),
            );
        }
        if !(self.font.size > 0.0 && self.font.size.is_finite()) {
            return invalid(
                "font.size",
                format!("{} doit être strictement positive", self.font.size),
            );
        }
        if self.tabs.min_tabs == 0 {
            return invalid(
                "tabs.min_tabs",
                "doit valoir au moins 1 (utiliser position = \"hidden\" pour masquer la barre)"
                    .into(),
            );
        }
        if self.window.scrollback_lines > MAX_SCROLLBACK_LINES {
            return invalid(
                "window.scrollback_lines",
                format!(
                    "{} dépasse le plafond de {MAX_SCROLLBACK_LINES}",
                    self.window.scrollback_lines
                ),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgb;
    use crate::sections::TabBarPosition;

    #[test]
    fn empty_document_is_the_default_config() {
        assert_eq!(Config::from_str("").unwrap(), Config::default());
    }

    #[test]
    fn defaults_match_the_spec() {
        let c = Config::default();
        assert_eq!(c.font.family, "monospace");
        assert_eq!(c.font.size, 11.0);
        assert_eq!(c.window.opacity, 1.0);
        assert_eq!(c.window.scrollback_lines, 10_000);
        assert_eq!(c.tabs.position, TabBarPosition::Top);
        assert!(c.tabs.close_button);
        assert_eq!(
            c.tabs.close_button_style.background,
            Rgb::new(0xd3, 0x2f, 0x2f)
        );
        assert_eq!(c.colors.background, Rgb::new(0x1e, 0x1e, 0x2e));
        assert_eq!(c.colors.palette[1], Rgb::new(0xf3, 0x8b, 0xa8));
        assert_eq!(c.colors.palette[15], Rgb::new(0xa6, 0xad, 0xc8));
    }

    #[test]
    fn partial_sections_override_only_what_they_name() {
        let c = Config::from_str(
            "[font]\nsize = 14.5\n\n[window]\nopacity = 0.9\n\n[tabs]\nposition = \"bottom\"\n",
        )
        .unwrap();
        assert_eq!(c.font.size, 14.5);
        assert_eq!(
            c.font.family, "monospace",
            "le reste de la section garde le défaut"
        );
        assert_eq!(c.window.opacity, 0.9);
        assert_eq!(c.tabs.position, TabBarPosition::Bottom);
    }

    #[test]
    fn nested_close_button_style_and_palette() {
        let toml = "[tabs.close_button_style]\nhover_background = \"#ff0000\"\n\n[colors]\npalette = [\"#000000\", \"#111111\", \"#222222\", \"#333333\", \"#444444\", \"#555555\", \"#666666\", \"#777777\", \"#888888\", \"#999999\", \"#aaaaaa\", \"#bbbbbb\", \"#cccccc\", \"#dddddd\", \"#eeeeee\", \"#ffffff\"]\n";
        let c = Config::from_str(toml).unwrap();
        assert_eq!(
            c.tabs.close_button_style.hover_background,
            Rgb::new(255, 0, 0)
        );
        assert_eq!(
            c.tabs.close_button_style.foreground,
            Rgb::new(255, 255, 255),
            "défaut conservé"
        );
        assert_eq!(c.colors.palette[15], Rgb::new(255, 255, 255));
    }

    #[test]
    fn unknown_field_is_an_error_with_line() {
        let err = Config::from_str("[font]\nsize = 12\n\n[window]\nopacityy = 0.5\n").unwrap_err();
        match err {
            ConfigError::Parse { line, message, .. } => {
                assert_eq!(line, 5, "{message}");
                assert!(message.contains("opacityy"), "{message}");
            }
            other => panic!("attendu Parse, obtenu {other:?}"),
        }
        let err = Config::from_str("[fonts]\nsize = 12\n").unwrap_err();
        assert!(matches!(err, ConfigError::Parse { line: 1, .. }), "{err}");
    }

    #[test]
    fn syntax_error_is_positioned() {
        let err = Config::from_str("[font]\nsize = \n").unwrap_err();
        assert!(matches!(err, ConfigError::Parse { line: 2, .. }), "{err}");
    }

    #[test]
    fn wrong_palette_length_is_an_error() {
        let err = Config::from_str("[colors]\npalette = [\"#000000\"]\n").unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }), "{err}");
    }

    #[test]
    fn validation_rejects_out_of_range_values() {
        for (toml, field) in [
            ("[window]\nopacity = 1.5\n", "window.opacity"),
            ("[window]\nopacity = -0.1\n", "window.opacity"),
            ("[font]\nsize = 0\n", "font.size"),
            ("[font]\nsize = -3\n", "font.size"),
            ("[tabs]\nmin_tabs = 0\n", "tabs.min_tabs"),
            (
                "[window]\nscrollback_lines = 10000000\n",
                "window.scrollback_lines",
            ),
        ] {
            match Config::from_str(toml) {
                Err(ConfigError::Invalid { field: f, .. }) => assert_eq!(f, field, "{toml}"),
                other => panic!("{toml}: attendu Invalid, obtenu {other:?}"),
            }
        }
    }

    #[test]
    fn error_messages_are_in_french_and_name_the_position() {
        let err = Config::from_str("[window]\nopacity = 2\n").unwrap_err();
        assert!(err.to_string().contains("window.opacity"), "{err}");
        let err = Config::from_str("x = \n").unwrap_err();
        assert!(err.to_string().starts_with("ligne 1"), "{err}");
    }
}
