//! La configuration complète : analyse, validation.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::ConfigError;
use crate::keymap::KeyMap;
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
    pub keys: KeyMap,
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

    /// Lit et valide `path`. Un fichier absent n'est pas une erreur : ce sont
    /// les défauts. Un fichier illisible en est une.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_str(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// `~/.config/rustty/rustty.toml` sur Linux, l'équivalent ailleurs.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "rustty").map(|d| d.config_dir().join("rustty.toml"))
    }

    /// Charge le fichier par défaut ; sans chemin résolvable, ce sont les défauts.
    pub fn load_default() -> Result<Self, ConfigError> {
        match Self::default_path() {
            Some(path) => Self::load(&path),
            None => Ok(Self::default()),
        }
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

    #[test]
    fn keys_section_overrides_defaults_and_is_optional() {
        let c = Config::from_str("[keys]\n\"ctrl+shift+n\" = \"new_tab\"\n").unwrap();
        assert_eq!(
            c.keys.resolve("ctrl+shift+n".parse().unwrap()),
            Some(crate::action::Action::NewTab)
        );
        assert_eq!(
            c.keys.resolve("ctrl+shift+t".parse().unwrap()),
            Some(crate::action::Action::NewTab)
        );
        assert_eq!(
            Config::from_str("").unwrap().keys.len(),
            crate::keymap::KeyMap::defaults().len()
        );
    }

    #[test]
    fn bad_binding_in_keys_section_is_a_positioned_parse_error() {
        let err = Config::from_str("[font]\nsize = 12\n\n[keys]\n\"ctlr+t\" = \"new_tab\"\n")
            .unwrap_err();
        match err {
            ConfigError::Parse { line, message, .. } => {
                assert!(message.contains("ctlr+t"), "{message}");
                assert!(
                    line >= 4,
                    "au moins la ligne de la table [keys], obtenu {line}"
                );
            }
            other => panic!("attendu Parse, obtenu {other:?}"),
        }
    }

    fn scratch_file(name: &str, contents: Option<&str>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustty-config-tests-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        match contents {
            Some(c) => std::fs::write(&path, c).unwrap(),
            None => {
                let _ = std::fs::remove_file(&path);
            }
        }
        path
    }

    #[test]
    fn missing_file_means_defaults() {
        let path = scratch_file("absent.toml", None);
        assert_eq!(Config::load(&path).unwrap(), Config::default());
    }

    #[test]
    fn existing_file_is_parsed_and_validated() {
        let path = scratch_file("ok.toml", Some("[font]\nsize = 13\n"));
        assert_eq!(Config::load(&path).unwrap().font.size, 13.0);
        let path = scratch_file("bad.toml", Some("[window]\nopacity = 7\n"));
        assert!(matches!(
            Config::load(&path),
            Err(ConfigError::Invalid {
                field: "window.opacity",
                ..
            })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_file_is_an_io_error_naming_the_path() {
        use std::os::unix::fs::PermissionsExt;
        let path = scratch_file("locked.toml", Some("[font]\nsize = 13\n"));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        let result = Config::load(&path);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        if nix_is_root() {
            return; // root lit tout : le cas ne peut pas être reproduit
        }
        match result {
            Err(ConfigError::Io { path: p, .. }) => assert_eq!(p, path),
            other => panic!("attendu Io, obtenu {other:?}"),
        }
    }

    #[cfg(unix)]
    fn nix_is_root() -> bool {
        std::fs::read_to_string("/proc/self/status")
            .map(|s| s.lines().any(|l| l.starts_with("Uid:\t0\t")))
            .unwrap_or(false)
    }

    #[test]
    fn default_path_is_rustty_toml_inside_a_rustty_directory() {
        if let Some(p) = Config::default_path() {
            assert_eq!(
                p.file_name().and_then(|n| n.to_str()),
                Some("rustty.toml"),
                "{p:?}"
            );
            assert!(p.components().any(|c| c.as_os_str() == "rustty"), "{p:?}");
        }
    }
}
