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
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parametrized {
    split: Option<SplitAxis>,
    focus: Option<FocusDirection>,
    resize: Option<ResizeDir>,
    opacity: Option<f32>,
    go_to_tab: Option<u8>,
    scroll_lines: Option<i32>,
    scroll_pages: Option<i32>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Spec {
    Simple(Simple),
    Parametrized(Parametrized),
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        match Spec::deserialize(deserializer).map_err(|_| {
            D::Error::custom("action inconnue : attendu un nom comme \"new_tab\" ou une table comme { split = \"horizontal\" }")
        })? {
            Spec::Simple(s) => Ok(match s {
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
            }),
            Spec::Parametrized(p) => {
                let candidates = [
                    p.split.map(Self::Split),
                    p.focus.map(Self::Focus),
                    p.resize.map(Self::Resize),
                    p.opacity.map(Self::Opacity),
                    p.go_to_tab.map(Self::GoToTab),
                    p.scroll_lines.map(Self::ScrollLines),
                    p.scroll_pages.map(Self::ScrollPages),
                ];
                let mut found = candidates.into_iter().flatten();
                match (found.next(), found.next()) {
                    (Some(action), None) => Ok(action),
                    _ => Err(D::Error::custom("une action paramétrée a exactement une clé")),
                }
            }
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
