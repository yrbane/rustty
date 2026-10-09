//! Table des raccourcis : les défauts, surchargés par la section `[keys]`.

use std::collections::HashMap;

use serde::Deserialize;

use crate::action::{Action, FocusDirection, ResizeDir, SplitAxis};
use crate::keys::KeyCombo;

#[derive(Clone, Debug, PartialEq)]
pub struct KeyMap {
    bindings: HashMap<KeyCombo, Action>,
}

/// Raccourcis par défaut, proches de ceux de kitty.
const DEFAULTS: &[(&str, Action)] = &[
    ("ctrl+shift+t", Action::NewTab),
    ("ctrl+shift+q", Action::CloseTab),
    ("ctrl+shift+w", Action::CloseWindow),
    ("ctrl+shift+right", Action::NextTab),
    ("ctrl+shift+left", Action::PrevTab),
    ("ctrl+tab", Action::NextTab),
    ("ctrl+shift+tab", Action::PrevTab),
    ("alt+1", Action::GoToTab(1)),
    ("alt+2", Action::GoToTab(2)),
    ("alt+3", Action::GoToTab(3)),
    ("alt+4", Action::GoToTab(4)),
    ("alt+5", Action::GoToTab(5)),
    ("ctrl+shift+o", Action::Split(SplitAxis::Horizontal)),
    ("ctrl+shift+e", Action::Split(SplitAxis::Vertical)),
    ("shift+left", Action::Focus(FocusDirection::Left)),
    ("shift+right", Action::Focus(FocusDirection::Right)),
    ("shift+up", Action::Focus(FocusDirection::Up)),
    ("shift+down", Action::Focus(FocusDirection::Down)),
    ("ctrl+left", Action::Resize(ResizeDir::Narrower)),
    ("ctrl+right", Action::Resize(ResizeDir::Wider)),
    ("ctrl+up", Action::Resize(ResizeDir::Taller)),
    ("ctrl+down", Action::Resize(ResizeDir::Shorter)),
    ("ctrl+shift+z", Action::ToggleZoom),
    ("ctrl+shift+r", Action::Rotate),
    ("ctrl+shift+c", Action::Copy),
    ("ctrl+shift+v", Action::Paste),
    ("shift+page_up", Action::ScrollPages(1)),
    ("shift+page_down", Action::ScrollPages(-1)),
    ("ctrl+shift+end", Action::ScrollToBottom),
    ("ctrl+shift+f5", Action::ReloadConfig),
    ("ctrl+shift+alt+t", Action::RenameTab),
    ("ctrl+plus", Action::IncreaseFontSize),
    ("ctrl+shift+plus", Action::IncreaseFontSize),
    ("ctrl+equal", Action::IncreaseFontSize),
    ("ctrl+minus", Action::DecreaseFontSize),
    ("ctrl+0", Action::ResetFontSize),
    ("ctrl+shift+0", Action::ResetFontSize),
];

impl KeyMap {
    pub fn empty() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn defaults() -> Self {
        let mut km = Self::empty();
        for (combo, action) in DEFAULTS {
            km.insert(combo.parse().expect("les défauts sont valides"), *action);
        }
        km
    }

    pub fn insert(&mut self, combo: KeyCombo, action: Action) {
        self.bindings.insert(combo, action);
    }

    /// L'action liée à `combo`, `None` si rien n'est lié ou si c'est `Unbind`.
    pub fn resolve(&self, combo: KeyCombo) -> Option<Action> {
        match self.bindings.get(&combo) {
            None | Some(Action::Unbind) => None,
            Some(a) => Some(*a),
        }
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn bindings(&self) -> impl Iterator<Item = (&KeyCombo, &Action)> {
        self.bindings.iter()
    }
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::defaults()
    }
}

impl<'de> Deserialize<'de> for KeyMap {
    /// La section `[keys]` est lue clé par clé pour que chaque erreur nomme le
    /// raccourci concerné, puis appliquée par-dessus les défauts.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let raw = toml::Table::deserialize(deserializer)?;
        let mut km = Self::defaults();
        // Graphie utilisée pour chaque combinaison déjà vue dans cette table.
        let mut seen: HashMap<KeyCombo, String> = HashMap::new();
        for (text, value) in raw {
            let combo: KeyCombo = text.parse().map_err(D::Error::custom)?;
            if let Some(previous) = seen.insert(combo, text.clone()) {
                return Err(D::Error::custom(format!(
                    "raccourci « {text} » déjà défini sous la graphie « {previous} »"
                )));
            }
            let shown = value.to_string();
            let action: Action = value.try_into().map_err(|e: toml::de::Error| {
                D::Error::custom(format!("raccourci « {text} » = {shown} : {}", e.message()))
            })?;
            km.insert(combo, action);
        }
        Ok(km)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_tab_switches_tabs_by_default() {
        let k = KeyMap::defaults();
        assert_eq!(
            k.resolve("ctrl+tab".parse().unwrap()),
            Some(Action::NextTab)
        );
        assert_eq!(
            k.resolve("ctrl+shift+tab".parse().unwrap()),
            Some(Action::PrevTab)
        );
    }

    #[test]
    fn ctrl_tab_can_be_unbound() {
        let c = crate::config::Config::from_str("[keys]\n\"ctrl+tab\" = \"none\"\n").unwrap();
        assert_eq!(c.keys.resolve("ctrl+tab".parse().unwrap()), None);
    }

    #[test]
    fn rename_and_zoom_have_default_bindings() {
        let k = KeyMap::defaults();
        let r = |s: &str| k.resolve(s.parse().unwrap());
        assert_eq!(r("ctrl+shift+alt+t"), Some(Action::RenameTab));
        assert_eq!(r("ctrl+plus"), Some(Action::IncreaseFontSize));
        assert_eq!(r("ctrl+shift+plus"), Some(Action::IncreaseFontSize));
        assert_eq!(r("ctrl+equal"), Some(Action::IncreaseFontSize));
        assert_eq!(r("ctrl+minus"), Some(Action::DecreaseFontSize));
        assert_eq!(r("ctrl+0"), Some(Action::ResetFontSize));
        assert_eq!(
            r("ctrl+shift+0"),
            Some(Action::ResetFontSize),
            "AZERTY : 0 demande shift"
        );
    }
    use crate::action::{FocusDirection, SplitAxis};

    fn combo(s: &str) -> KeyCombo {
        s.parse().unwrap()
    }

    #[test]
    fn defaults_cover_the_v01_actions() {
        let km = KeyMap::defaults();
        assert_eq!(km.resolve(combo("ctrl+shift+t")), Some(Action::NewTab));
        assert_eq!(
            km.resolve(combo("ctrl+shift+o")),
            Some(Action::Split(SplitAxis::Horizontal))
        );
        assert_eq!(
            km.resolve(combo("ctrl+shift+e")),
            Some(Action::Split(SplitAxis::Vertical))
        );
        assert_eq!(
            km.resolve(combo("shift+left")),
            Some(Action::Focus(FocusDirection::Left))
        );
        assert_eq!(km.resolve(combo("ctrl+shift+c")), Some(Action::Copy));
        assert_eq!(km.resolve(combo("ctrl+shift+v")), Some(Action::Paste));
        assert_eq!(
            km.resolve(combo("shift+page_up")),
            Some(Action::ScrollPages(1))
        );
        assert_eq!(
            km.resolve(combo("ctrl+shift+f5")),
            Some(Action::ReloadConfig)
        );
        assert_eq!(km.resolve(combo("ctrl+c")), None, "ctrl+c reste au shell");
        assert!(km.len() >= 20);
    }

    #[test]
    fn user_bindings_override_and_unbind() {
        let km: KeyMap = toml::from_str("\"ctrl+shift+t\" = \"close_tab\"\n\"ctrl+shift+q\" = \"none\"\n\"f9\" = { split = \"vertical\" }\n").unwrap();
        assert_eq!(km.resolve(combo("ctrl+shift+t")), Some(Action::CloseTab));
        assert_eq!(km.resolve(combo("ctrl+shift+q")), None, "délié");
        assert_eq!(
            km.resolve(combo("f9")),
            Some(Action::Split(SplitAxis::Vertical))
        );
        assert_eq!(
            km.resolve(combo("ctrl+shift+e")),
            Some(Action::Split(SplitAxis::Vertical)),
            "défaut non touché conservé"
        );
    }

    #[test]
    fn same_combo_written_differently_is_one_binding() {
        let km: KeyMap = toml::from_str("\"Shift+Ctrl+T\" = \"close_tab\"\n").unwrap();
        assert_eq!(km.resolve(combo("ctrl+shift+t")), Some(Action::CloseTab));
    }

    #[test]
    fn two_spellings_of_the_same_combo_are_an_error() {
        let err = toml::from_str::<KeyMap>(
            "\"ctrl+shift+t\" = \"close_tab\"\n\"shift+ctrl+t\" = \"new_tab\"\n",
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ctrl+shift+t"), "{msg}");
        assert!(msg.contains("shift+ctrl+t"), "{msg}");
    }

    #[test]
    fn errors_name_the_offending_binding() {
        let err = toml::from_str::<KeyMap>("\"ctrl+\" = \"new_tab\"\n").unwrap_err();
        assert!(err.to_string().contains("ctrl+"), "{err}");
        let err = toml::from_str::<KeyMap>("\"ctrl+shift+t\" = \"new_tabz\"\n").unwrap_err();
        assert!(err.to_string().contains("ctrl+shift+t"), "{err}");
        assert!(err.to_string().contains("new_tabz"), "{err}");
    }

    #[test]
    fn empty_keymap_resolves_nothing() {
        assert_eq!(KeyMap::empty().resolve(combo("ctrl+shift+t")), None);
        assert_eq!(KeyMap::empty().len(), 0);
    }
}
