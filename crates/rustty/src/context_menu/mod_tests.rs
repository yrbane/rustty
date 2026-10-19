use super::*;
use crate::accent::{ACCENT_NAMES, ACCENT_SLOTS};
use rustty_config::SplitAxis;

fn items(entries: &[MenuEntry]) -> Vec<MenuItem> {
    entries
        .iter()
        .filter_map(|e| match e {
            MenuEntry::Item(i) => Some(*i),
            MenuEntry::Separator => None,
        })
        .collect()
}

fn labels(menu: &ContextMenu) -> Vec<&'static str> {
    items(&menu.entries)
        .into_iter()
        .map(|i| menu.label(i))
        .collect()
}

#[test]
fn pane_menu_is_unchanged() {
    let menu = ContextMenu::pane(TermId(1), 0, 0, true, false, &KeyMap::defaults());
    assert_eq!(menu.target, MenuTarget::Pane(TermId(1)));
    // `match` exhaustif : ajouter une entrée sans la lister ne compile plus.
    for item in items(&menu.entries) {
        let expected = match item {
            MenuItem::Copy => Action::Copy,
            MenuItem::Paste => Action::Paste,
            MenuItem::SplitVertical => Action::Split(SplitAxis::Vertical),
            MenuItem::SplitHorizontal => Action::Split(SplitAxis::Horizontal),
            MenuItem::ToggleZoom => Action::ToggleZoom,
            MenuItem::RenameTab => Action::RenameTab,
            MenuItem::NewTab => Action::NewTab,
            MenuItem::ClosePane => Action::CloseWindow,
            other => panic!("{other:?} n'est pas au menu de panneau"),
        };
        assert_eq!(item.action(), Some(expected), "{item:?}");
    }
    assert_eq!(
        items(&menu.entries).len(),
        8,
        "les huit entrées sont au menu"
    );
    assert_eq!(menu.entries.len(), 11);
    assert_eq!(
        menu.entries.last(),
        Some(&MenuEntry::Item(MenuItem::ClosePane)),
        "fermer en dernier, à l'écart"
    );
}

#[test]
fn tab_menu_lists_its_five_items() {
    let menu = ContextMenu::tab(2, 0, 0, true, &KeyMap::defaults());
    assert_eq!(menu.target, MenuTarget::Tab(2));
    assert_eq!(
        labels(&menu),
        [
            "Renommer",
            "Changer la couleur",
            "Nouvel onglet",
            "Fermer l'onglet",
            "Fermer les autres onglets"
        ]
    );
    assert_eq!(menu.entries[3], MenuEntry::Separator);
}

#[test]
fn header_menu_lists_its_six_items() {
    let menu = ContextMenu::header(TermId(4), 0, 0, &KeyMap::defaults());
    assert_eq!(
        labels(&menu),
        [
            "Diviser horizontalement",
            "Diviser verticalement",
            "Agrandir le panneau",
            "Déplacer dans un nouvel onglet",
            "Renommer le panneau",
            "Fermer le panneau"
        ]
    );
    assert_eq!(menu.entries.len(), 8);
}

#[test]
fn palette_lists_twelve_colors_and_default() {
    let menu = ContextMenu::palette(0, 0, 0);
    assert_eq!(menu.entries.len(), 14);
    let all = items(&menu.entries);
    for (i, slot) in ACCENT_SLOTS.iter().enumerate() {
        assert_eq!(all[i], MenuItem::Color(*slot));
        assert_eq!(menu.label(all[i]), ACCENT_NAMES[i]);
    }
    assert_eq!(all[12], MenuItem::DefaultColor);
    assert_eq!(menu.label(all[12]), "Couleur par défaut");
    assert_eq!(menu.entries[12], MenuEntry::Separator);
    assert!(menu.shortcuts.iter().all(Option::is_none));
}

#[test]
fn close_others_disabled_with_a_single_tab() {
    let keys = KeyMap::defaults();
    assert!(!ContextMenu::tab(0, 0, 0, false, &keys).enabled(MenuItem::CloseOtherTabs));
    assert!(ContextMenu::tab(0, 0, 0, true, &keys).enabled(MenuItem::CloseOtherTabs));
    assert!(ContextMenu::tab(0, 0, 0, false, &keys).enabled(MenuItem::CloseTab));
}

#[test]
fn swatch_only_on_colors() {
    assert_eq!(MenuItem::Color(9).swatch(), Some(9));
    assert_eq!(MenuItem::DefaultColor.swatch(), None);
    assert_eq!(MenuItem::ChangeColor.swatch(), None);
    assert_eq!(MenuItem::Copy.swatch(), None);
    assert_eq!(MenuItem::CloseTab.action(), None);
    assert_eq!(MenuItem::Color(1).action(), None);
}

#[test]
fn shortcuts_are_captured_at_open() {
    let menu = ContextMenu::pane(TermId(1), 10, 20, true, false, &KeyMap::defaults());
    assert_eq!(
        menu.shortcuts,
        shortcuts(&menu.entries, &KeyMap::defaults())
    );
    assert_eq!(menu.shortcuts.len(), menu.entries.len());
    assert_eq!((menu.x, menu.y, menu.can_copy), (10, 20, true));
    let bare = ContextMenu::pane(TermId(1), 0, 0, false, true, &KeyMap::empty());
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

#[test]
fn tab_and_palette_menus_are_dropped_when_any_term_goes_away() {
    // Un onglet qui disparaît décale les index : ces menus n'ont plus de cible sûre.
    let keys = KeyMap::default();
    assert!(ContextMenu::tab(2, 0, 0, true, &keys).dropped_by_exit_of(TermId(7)));
    assert!(ContextMenu::palette(2, 0, 0).dropped_by_exit_of(TermId(7)));
}

#[test]
fn pane_and_header_menus_are_dropped_only_by_their_own_term() {
    let keys = KeyMap::default();
    let pane = ContextMenu::pane(TermId(1), 0, 0, false, false, &keys);
    let header = ContextMenu::header(TermId(1), 0, 0, &keys);
    for menu in [pane, header] {
        assert!(menu.dropped_by_exit_of(TermId(1)));
        assert!(!menu.dropped_by_exit_of(TermId(2)));
    }
}
