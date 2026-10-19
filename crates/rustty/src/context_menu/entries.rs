//! Les entrées des menus contextuels : éléments, libellés, actions et liste
//! propre à chaque cible.

use rustty_config::{Action, SplitAxis};

use crate::accent::{ACCENT_NAMES, ACCENT_SLOTS};
use crate::tab::TermId;

/// Ce sur quoi le menu a été ouvert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuTarget {
    Pane(TermId),
    /// Index de l'onglet.
    Tab(usize),
    Header(TermId),
    /// Nuancier de l'onglet d'index donné.
    Palette(usize),
}

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
    CloseTab,
    CloseOtherTabs,
    ChangeColor,
    MoveToNewTab,
    RenamePane,
    /// Une couleur ANSI du nuancier (index de la palette).
    Color(usize),
    DefaultColor,
}

impl MenuItem {
    /// L'action de la configuration que l'entrée déclenche, s'il y en a une.
    pub fn action(self) -> Option<Action> {
        Some(match self {
            Self::Copy => Action::Copy,
            Self::Paste => Action::Paste,
            Self::SplitVertical => Action::Split(SplitAxis::Vertical),
            Self::SplitHorizontal => Action::Split(SplitAxis::Horizontal),
            Self::ToggleZoom => Action::ToggleZoom,
            Self::RenameTab => Action::RenameTab,
            Self::NewTab => Action::NewTab,
            Self::ClosePane => Action::CloseWindow,
            Self::CloseTab
            | Self::CloseOtherTabs
            | Self::ChangeColor
            | Self::MoveToNewTab
            | Self::RenamePane
            | Self::Color(_)
            | Self::DefaultColor => return None,
        })
    }

    /// La couleur ANSI à dessiner en pastille devant le libellé.
    pub fn swatch(self) -> Option<usize> {
        match self {
            Self::Color(slot) => Some(slot),
            _ => None,
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
            Self::CloseTab => "Fermer l'onglet",
            Self::CloseOtherTabs => "Fermer les autres onglets",
            Self::ChangeColor => "Changer la couleur",
            Self::MoveToNewTab => "Déplacer dans un nouvel onglet",
            Self::RenamePane => "Renommer le panneau",
            Self::Color(slot) => ACCENT_SLOTS
                .iter()
                .position(|s| *s == slot)
                .map_or("", |i| ACCENT_NAMES[i]),
            Self::DefaultColor => "Couleur par défaut",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuEntry {
    Item(MenuItem),
    Separator,
}

/// Les entrées du menu d'une cible, dans l'ordre d'affichage.
pub fn entries(target: &MenuTarget) -> Vec<MenuEntry> {
    use MenuEntry::{Item, Separator};
    use MenuItem as M;
    match target {
        MenuTarget::Pane(_) => vec![
            Item(M::Copy),
            Item(M::Paste),
            Separator,
            Item(M::SplitVertical),
            Item(M::SplitHorizontal),
            Item(M::ToggleZoom),
            Separator,
            Item(M::RenameTab),
            Item(M::NewTab),
            Separator,
            Item(M::ClosePane),
        ],
        MenuTarget::Tab(_) => vec![
            Item(M::RenameTab),
            Item(M::ChangeColor),
            Item(M::NewTab),
            Separator,
            Item(M::CloseTab),
            Item(M::CloseOtherTabs),
        ],
        MenuTarget::Header(_) => vec![
            Item(M::SplitHorizontal),
            Item(M::SplitVertical),
            Item(M::ToggleZoom),
            Separator,
            Item(M::MoveToNewTab),
            Item(M::RenamePane),
            Separator,
            Item(M::ClosePane),
        ],
        MenuTarget::Palette(_) => ACCENT_SLOTS
            .iter()
            .map(|slot| Item(M::Color(*slot)))
            .chain([Separator, Item(M::DefaultColor)])
            .collect(),
    }
}
