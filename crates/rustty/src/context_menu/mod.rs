//! Le menu du clic droit dans un panneau : entrées, raccourcis affichés,
//! placement dans la fenêtre, test de clic et dessin. Pur.

mod chrome;
mod layout;

pub use chrome::menu_chrome;
pub use layout::{MenuClick, MenuLayout, layout};

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
    /// Raccourcis de chaque entrée de `ENTRIES`, figés à l'ouverture.
    pub shortcuts: Vec<Option<String>>,
}

impl ContextMenu {
    pub fn new(term: TermId, x: u32, y: u32, can_copy: bool, zoomed: bool, keys: &KeyMap) -> Self {
        Self {
            term,
            x,
            y,
            can_copy,
            zoomed,
            hovered: None,
            shortcuts: shortcuts(keys),
        }
    }

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
        // `match` exhaustif : ajouter une entrée sans la lister ne compile plus.
        for entry in ENTRIES {
            let MenuEntry::Item(item) = entry else {
                continue;
            };
            let expected = match item {
                MenuItem::Copy => Action::Copy,
                MenuItem::Paste => Action::Paste,
                MenuItem::SplitVertical => Action::Split(SplitAxis::Vertical),
                MenuItem::SplitHorizontal => Action::Split(SplitAxis::Horizontal),
                MenuItem::ToggleZoom => Action::ToggleZoom,
                MenuItem::RenameTab => Action::RenameTab,
                MenuItem::NewTab => Action::NewTab,
                MenuItem::ClosePane => Action::CloseWindow,
            };
            assert_eq!(item.action(), expected, "{item:?}");
        }
        let items = ENTRIES
            .iter()
            .filter(|e| matches!(e, MenuEntry::Item(_)))
            .count();
        assert_eq!(items, 8, "les huit entrées sont au menu");
        assert_eq!(
            ENTRIES.last(),
            Some(&MenuEntry::Item(MenuItem::ClosePane)),
            "fermer en dernier, à l'écart"
        );
    }

    #[test]
    fn shortcuts_are_captured_at_open() {
        let menu = ContextMenu::new(TermId(1), 10, 20, true, false, &KeyMap::defaults());
        assert_eq!(menu.shortcuts, shortcuts(&KeyMap::defaults()));
        assert_eq!(menu.shortcuts.len(), ENTRIES.len());
        assert_eq!((menu.x, menu.y, menu.can_copy), (10, 20, true));
        let bare = ContextMenu::new(TermId(1), 0, 0, false, true, &KeyMap::empty());
        assert!(bare.shortcuts.iter().all(Option::is_none));
        assert!(bare.zoomed && bare.hovered.is_none());
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
