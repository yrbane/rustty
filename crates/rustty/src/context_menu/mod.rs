//! Le menu du clic droit dans un panneau : entrées, raccourcis affichés,
//! placement dans la fenêtre, test de clic et dessin. Pur.

mod layout;

pub use layout::{MenuLayout, layout, menu_chrome};

use rustty_config::{Action, KeyMap, SplitAxis};

use crate::tab::TermId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem {
    Copy,
    Paste,
    SplitVertical,
    SplitHorizontal,
    ToggleZoom,
    RenameTab,
    NewTab,
    ClosePane,
}

impl MenuItem {
    /// L'action de la configuration que l'entrée déclenche.
    pub fn action(self) -> Action {
        match self {
            Self::Copy => Action::Copy,
            Self::Paste => Action::Paste,
            Self::SplitVertical => Action::Split(SplitAxis::Vertical),
            Self::SplitHorizontal => Action::Split(SplitAxis::Horizontal),
            Self::ToggleZoom => Action::ToggleZoom,
            Self::RenameTab => Action::RenameTab,
            Self::NewTab => Action::NewTab,
            Self::ClosePane => Action::CloseWindow,
        }
    }

    pub fn label(self, zoomed: bool) -> &'static str {
        match self {
            Self::Copy => "Copier",
            Self::Paste => "Coller",
            Self::SplitVertical => "Diviser verticalement",
            Self::SplitHorizontal => "Diviser horizontalement",
            Self::ToggleZoom if zoomed => "Réduire le panneau",
            Self::ToggleZoom => "Agrandir le panneau",
            Self::RenameTab => "Renommer l'onglet",
            Self::NewTab => "Nouvel onglet",
            Self::ClosePane => "Fermer le panneau",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    Item(MenuItem),
    Separator,
}

pub const ENTRIES: [MenuEntry; 11] = [
    MenuEntry::Item(MenuItem::Copy),
    MenuEntry::Item(MenuItem::Paste),
    MenuEntry::Separator,
    MenuEntry::Item(MenuItem::SplitVertical),
    MenuEntry::Item(MenuItem::SplitHorizontal),
    MenuEntry::Item(MenuItem::ToggleZoom),
    MenuEntry::Separator,
    MenuEntry::Item(MenuItem::RenameTab),
    MenuEntry::Item(MenuItem::NewTab),
    MenuEntry::Separator,
    MenuEntry::Item(MenuItem::ClosePane),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenu {
    /// Panneau sur lequel le menu a été ouvert.
    pub term: TermId,
    /// Point du clic, en pixels.
    pub x: u32,
    pub y: u32,
    /// Une sélection existe dans ce panneau.
    pub can_copy: bool,
    /// Le panneau est zoomé (libellé « Réduire »).
    pub zoomed: bool,
    /// Index dans `ENTRIES` de l'entrée survolée.
    pub hovered: Option<usize>,
}

impl ContextMenu {
    pub(crate) fn enabled(&self, item: MenuItem) -> bool {
        item != MenuItem::Copy || self.can_copy
    }
}

/// Le raccourci le plus court lié à `action`, tel qu'on l'écrit dans `[keys]`.
pub fn shortcut_for(keys: &KeyMap, action: Action) -> Option<String> {
    keys.bindings()
        .filter(|(_, bound)| **bound == action)
        .map(|(combo, _)| combo.to_string())
        .min_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
}

/// Les raccourcis de chaque entrée de `ENTRIES` (`None` pour un séparateur).
pub fn shortcuts(keys: &KeyMap) -> Vec<Option<String>> {
    ENTRIES
        .iter()
        .map(|e| match e {
            MenuEntry::Item(item) => shortcut_for(keys, item.action()),
            MenuEntry::Separator => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_map_to_config_actions() {
        assert_eq!(MenuItem::ClosePane.action(), Action::CloseWindow);
        assert_eq!(
            MenuItem::SplitVertical.action(),
            Action::Split(SplitAxis::Vertical)
        );
        assert_eq!(
            MenuItem::SplitHorizontal.action(),
            Action::Split(SplitAxis::Horizontal)
        );
        assert_eq!(MenuItem::Copy.action(), Action::Copy);
        assert_eq!(MenuItem::RenameTab.action(), Action::RenameTab);
        assert!(ENTRIES.contains(&MenuEntry::Item(MenuItem::ClosePane)));
        assert_eq!(
            ENTRIES.last(),
            Some(&MenuEntry::Item(MenuItem::ClosePane)),
            "fermer en dernier, à l'écart"
        );
    }

    #[test]
    fn zoom_label_follows_the_state() {
        assert_eq!(MenuItem::ToggleZoom.label(false), "Agrandir le panneau");
        assert_eq!(MenuItem::ToggleZoom.label(true), "Réduire le panneau");
        assert_eq!(MenuItem::ClosePane.label(false), "Fermer le panneau");
    }

    #[test]
    fn shortcut_is_the_shortest_binding() {
        let keys = KeyMap::defaults();
        assert_eq!(
            shortcut_for(&keys, Action::NextTab).as_deref(),
            Some("ctrl+tab")
        );
        assert_eq!(
            shortcut_for(&keys, Action::CloseWindow).as_deref(),
            Some("ctrl+shift+w")
        );
        let custom = rustty_config::Config::from_str(
            "[keys]\n\"ctrl+shift+w\" = \"none\"\n\"alt+w\" = \"close_window\"\n",
        )
        .unwrap();
        assert_eq!(
            shortcut_for(&custom.keys, Action::CloseWindow).as_deref(),
            Some("alt+w")
        );
        assert_eq!(shortcut_for(&KeyMap::empty(), Action::Copy), None);
    }
}
