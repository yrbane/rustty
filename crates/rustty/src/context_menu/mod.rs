//! Le menu du clic droit dans un panneau : entrées, raccourcis affichés,
//! placement dans la fenêtre, test de clic et dessin. Pur.

mod chrome;
mod entries;
mod layout;

pub use chrome::menu_chrome;
pub use entries::{MenuEntry, MenuItem, MenuTarget, entries};
pub use layout::{MenuClick, MenuLayout, layout};

use rustty_config::{Action, KeyMap};

use crate::tab::TermId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenu {
    /// Ce sur quoi le menu a été ouvert.
    pub target: MenuTarget,
    /// Point du clic, en pixels.
    pub x: u32,
    pub y: u32,
    /// Une sélection existe dans ce panneau.
    pub can_copy: bool,
    /// Le panneau est zoomé (libellé « Réduire »).
    pub zoomed: bool,
    /// Il existe d'autres onglets à fermer.
    pub can_close_others: bool,
    /// Index dans `entries` de l'entrée survolée.
    pub hovered: Option<usize>,
    /// Les entrées de la cible, figées à l'ouverture.
    pub entries: Vec<MenuEntry>,
    /// Raccourcis de chaque entrée de `entries`, figés à l'ouverture.
    pub shortcuts: Vec<Option<String>>,
}

impl ContextMenu {
    fn build(target: MenuTarget, x: u32, y: u32, keys: Option<&KeyMap>) -> Self {
        let entries = entries(&target);
        let shortcuts = match keys {
            Some(keys) => shortcuts(&entries, keys),
            None => vec![None; entries.len()],
        };
        Self {
            target,
            x,
            y,
            can_copy: false,
            zoomed: false,
            can_close_others: true,
            hovered: None,
            entries,
            shortcuts,
        }
    }

    pub fn pane(term: TermId, x: u32, y: u32, can_copy: bool, zoomed: bool, keys: &KeyMap) -> Self {
        Self {
            can_copy,
            zoomed,
            ..Self::build(MenuTarget::Pane(term), x, y, Some(keys))
        }
    }

    // Les trois constructeurs suivants servent à la tâche 8 (clic droit).
    #[allow(dead_code)]
    pub fn tab(index: usize, x: u32, y: u32, can_close_others: bool, keys: &KeyMap) -> Self {
        Self {
            can_close_others,
            ..Self::build(MenuTarget::Tab(index), x, y, Some(keys))
        }
    }

    #[allow(dead_code)]
    pub fn header(term: TermId, x: u32, y: u32, keys: &KeyMap) -> Self {
        Self::build(MenuTarget::Header(term), x, y, Some(keys))
    }

    #[allow(dead_code)]
    pub fn palette(tab: usize, x: u32, y: u32) -> Self {
        Self::build(MenuTarget::Palette(tab), x, y, None)
    }

    /// Le terminal visé par le menu (panneau ou bandeau), s'il y en a un.
    pub fn term(&self) -> Option<TermId> {
        match self.target {
            MenuTarget::Pane(t) | MenuTarget::Header(t) => Some(t),
            MenuTarget::Tab(_) | MenuTarget::Palette(_) => None,
        }
    }

    pub(crate) fn enabled(&self, item: MenuItem) -> bool {
        match item {
            MenuItem::Copy => self.can_copy,
            MenuItem::CloseOtherTabs => self.can_close_others,
            _ => true,
        }
    }

    /// Le libellé affiché : le menu d'onglet dit « Renommer » tout court.
    pub fn label(&self, item: MenuItem) -> &'static str {
        match (self.target, item) {
            (MenuTarget::Tab(_), MenuItem::RenameTab) => "Renommer",
            _ => item.label(self.zoomed),
        }
    }
}

/// Le raccourci le plus court lié à `action`, tel qu'on l'écrit dans `[keys]`.
pub fn shortcut_for(keys: &KeyMap, action: Action) -> Option<String> {
    keys.bindings()
        .filter(|(_, bound)| **bound == action)
        .map(|(combo, _)| combo.to_string())
        .min_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
}

/// Les raccourcis de chaque entrée (`None` pour un séparateur ou une entrée
/// sans action de configuration).
pub fn shortcuts(entries: &[MenuEntry], keys: &KeyMap) -> Vec<Option<String>> {
    entries
        .iter()
        .map(|e| match e {
            MenuEntry::Item(item) => item.action().and_then(|a| shortcut_for(keys, a)),
            MenuEntry::Separator => None,
        })
        .collect()
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
