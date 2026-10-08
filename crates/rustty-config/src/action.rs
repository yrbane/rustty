//! Ce qu'un raccourci déclenche. Deux écritures TOML : une chaîne pour les
//! actions sans paramètre, une table à une clé pour les autres.

use serde::Deserialize;

/// Même convention que `rustty-layout` : `horizontal` empile, `vertical` juxtapose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResizeDir {
    Narrower,
    Wider,
    Taller,
    Shorter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    GoToTab(u8),
    CloseWindow,
    Split(SplitAxis),
    Focus(FocusDirection),
    Resize(ResizeDir),
    ToggleZoom,
    Rotate,
    /// Variation d'opacité, positive ou négative.
    Opacity(f32),
    Copy,
    Paste,
    ScrollLines(i32),
    ScrollPages(i32),
    ScrollToBottom,
    ReloadConfig,
    /// `"none"` : retire un raccourci par défaut.
    Unbind,
}

/// Les actions sans paramètre, telles qu'écrites dans le TOML.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Simple {
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    CloseWindow,
    ToggleZoom,
    Rotate,
    Copy,
    Paste,
    ScrollToBottom,
    ReloadConfig,
    None,
}

/// Les actions à paramètre : une table avec exactement une clé.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parametrized {
    split: Option<SplitAxis>,
    focus: Option<FocusDirection>,
    resize: Option<ResizeDir>,
    opacity: Option<Delta>,
    go_to_tab: Option<u8>,
    scroll_lines: Option<i32>,
    scroll_pages: Option<i32>,
}

/// Variation d'opacité : nombre TOML (`+0.05`) ou chaîne (`"+0.05"`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
enum Delta {
    Number(f32),
    Text(String),
}

impl Delta {
    /// Valeur finie comprise entre -1.0 et 1.0.
    fn into_opacity(self) -> Result<f32, String> {
        let value = match self {
            Self::Number(n) => n,
            Self::Text(t) => t
                .trim()
                .parse::<f32>()
                .map_err(|_| format!("opacity : « {t} » n'est pas un nombre"))?,
        };
        if value.is_finite() && value.abs() <= 1.0 {
            Ok(value)
        } else {
            Err("opacity : variation attendue entre -1.0 et 1.0".to_owned())
        }
    }
}

impl From<Simple> for Action {
    fn from(simple: Simple) -> Self {
        match simple {
            Simple::NewTab => Self::NewTab,
            Simple::CloseTab => Self::CloseTab,
            Simple::NextTab => Self::NextTab,
            Simple::PrevTab => Self::PrevTab,
            Simple::CloseWindow => Self::CloseWindow,
            Simple::ToggleZoom => Self::ToggleZoom,
            Simple::Rotate => Self::Rotate,
            Simple::Copy => Self::Copy,
            Simple::Paste => Self::Paste,
            Simple::ScrollToBottom => Self::ScrollToBottom,
            Simple::ReloadConfig => Self::ReloadConfig,
            Simple::None => Self::Unbind,
        }
    }
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        use toml::Value;
        let value = Value::deserialize(deserializer)?;
        let custom = |e: toml::de::Error| D::Error::custom(e.message());
        match value {
            Value::String(_) => Simple::deserialize(value).map(Self::from).map_err(custom),
            Value::Table(_) => {
                let p = Parametrized::deserialize(value).map_err(custom)?;
                let opacity = p
                    .opacity
                    .map(|d| d.into_opacity().map(Self::Opacity))
                    .transpose()
                    .map_err(D::Error::custom)?;
                let candidates = [
                    p.split.map(Self::Split),
                    p.focus.map(Self::Focus),
                    p.resize.map(Self::Resize),
                    opacity,
                    p.go_to_tab.map(Self::GoToTab),
                    p.scroll_lines.map(Self::ScrollLines),
                    p.scroll_pages.map(Self::ScrollPages),
                ];
                let mut found = candidates.into_iter().flatten();
                match (found.next(), found.next()) {
                    (Some(action), None) => Ok(action),
                    _ => Err(D::Error::custom(
                        "une action paramétrée a exactement une clé",
                    )),
                }
            }
            _ => Err(D::Error::custom(
                "action inconnue : attendu un nom comme \"new_tab\" ou une table comme { split = \"horizontal\" }",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml_value: &str) -> Result<Action, toml::de::Error> {
        #[derive(serde::Deserialize)]
        struct Doc {
            a: Action,
        }
        toml::from_str::<Doc>(&format!("a = {toml_value}")).map(|d| d.a)
    }

    #[test]
    fn string_forms() {
        assert_eq!(parse("\"new_tab\"").unwrap(), Action::NewTab);
        assert_eq!(parse("\"close_window\"").unwrap(), Action::CloseWindow);
        assert_eq!(parse("\"toggle_zoom\"").unwrap(), Action::ToggleZoom);
        assert_eq!(
            parse("\"scroll_to_bottom\"").unwrap(),
            Action::ScrollToBottom
        );
        assert_eq!(parse("\"none\"").unwrap(), Action::Unbind);
    }

    #[test]
    fn table_forms() {
        assert_eq!(
            parse("{ split = \"horizontal\" }").unwrap(),
            Action::Split(SplitAxis::Horizontal)
        );
        assert_eq!(
            parse("{ focus = \"left\" }").unwrap(),
            Action::Focus(FocusDirection::Left)
        );
        assert_eq!(
            parse("{ resize = \"wider\" }").unwrap(),
            Action::Resize(ResizeDir::Wider)
        );
        assert_eq!(parse("{ opacity = +0.05 }").unwrap(), Action::Opacity(0.05));
        assert_eq!(parse("{ opacity = -0.1 }").unwrap(), Action::Opacity(-0.1));
        assert_eq!(parse("{ go_to_tab = 3 }").unwrap(), Action::GoToTab(3));
        assert_eq!(
            parse("{ scroll_lines = -3 }").unwrap(),
            Action::ScrollLines(-3)
        );
        assert_eq!(
            parse("{ scroll_pages = 1 }").unwrap(),
            Action::ScrollPages(1)
        );
    }

    #[test]
    fn opacity_is_bounded_and_finite() {
        for bad in ["nan", "inf", "5.0", "-5.0"] {
            assert!(parse(&format!("{{ opacity = {bad} }}")).is_err(), "{bad}");
        }
        assert_eq!(parse("{ opacity = -0.1 }").unwrap(), Action::Opacity(-0.1));
    }

    #[test]
    fn opacity_accepts_a_string() {
        assert_eq!(
            parse("{ opacity = \"+0.05\" }").unwrap(),
            Action::Opacity(0.05)
        );
        assert_eq!(
            parse("{ opacity = \"-0.1\" }").unwrap(),
            Action::Opacity(-0.1)
        );
        let err = parse("{ opacity = \"abc\" }").unwrap_err();
        assert!(err.to_string().contains("abc"), "{err}");
        assert!(parse("{ opacity = \"nan\" }").is_err());
    }

    #[test]
    fn inner_messages_are_kept() {
        let err = parse("{ split = \"diagonal\" }").unwrap_err();
        assert!(err.to_string().contains("diagonal"), "{err}");
        assert!(err.to_string().contains("horizontal"), "{err}");
        let err = parse("\"new_tabz\"").unwrap_err();
        assert!(err.to_string().contains("new_tabz"), "{err}");
    }

    #[test]
    fn unknown_forms_are_errors() {
        assert!(parse("\"new_tabz\"").is_err());
        assert!(parse("{ split = \"diagonal\" }").is_err());
        assert!(
            parse("{ split = \"horizontal\", focus = \"left\" }").is_err(),
            "une seule clé par action"
        );
        assert!(parse("{ teleport = 1 }").is_err());
        assert!(parse("42").is_err());
    }
}
