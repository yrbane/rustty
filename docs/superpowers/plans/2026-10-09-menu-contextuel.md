# Menu contextuel — plan d'implémentation (plan 7)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** demande de seb du 2026-10-09 : « un menu au clic droit dans les panneaux pour, entre autres, fermer un panneau ».

**Architecture :** un module pur `context_menu.rs` (entrées, libellés, raccourcis lus dans la configuration, mise en page bornée à la fenêtre, test de clic, traduction en chrome) et un module de dispatch `menu_input.rs` (ouvrir, survoler, exécuter, fermer) ; le menu est dessiné dans la dernière passe de rendu, au-dessus des panneaux zoomés.

## Décisions

- Ouverture : clic droit dans un panneau ; dans une application qui capte la souris, `shift`+clic droit (même règle que la sélection).
- Entrées : Copier (grisé sans sélection dans ce panneau), Coller, —, Diviser verticalement, Diviser horizontalement, Agrandir / Réduire le panneau, —, Renommer l'onglet, Nouvel onglet, —, Fermer le panneau. Chaque entrée affiche son raccourci (le plus court de `[keys]`), rien si elle n'en a pas.
- Exécution : le panneau cliqué prend le focus, puis l'action de la configuration correspondante s'exécute (`copy`, `paste`, `split`, `toggle_zoom`, `rename_tab`, `new_tab`, `close_window` — confirmation de fermeture comprise).
- Fermeture du menu : Échap (consommée), un clic hors du menu (consommé), une autre touche (qui agit ensuite normalement), la perte du focus, un redimensionnement.
- Placement : au point du clic, décalé vers la gauche ou vers le haut pour rester dans la fenêtre.
- Images (`img`) : plan 8.

## Global Constraints

Comme les plans 5 et 6 ; tâche N → `0.1.0-alpha.(90+N)` (le plan porte `0.1.0-alpha.90`).

## Review Focus

1. Menu ouvert près d'un bord ou dans une fenêtre plus petite que lui : toujours entièrement visible si possible, jamais de coordonnée négative. Test `menu_stays_inside_the_window`.
2. Clic sur un séparateur ou une entrée grisée : rien ne s'exécute, le menu reste ouvert. Test `separators_and_disabled_items_are_not_clickable`.
3. Fermeture du panneau par le menu quand un programme tourne : la confirmation habituelle s'affiche. Vérifié par le modèle existant (`close_window`).
4. Raccourci affiché après rechargement de la configuration : celui de la nouvelle `[keys]`. Test `shortcut_is_the_shortest_binding`.
5. Menu ouvert puis panneau fermé par le shell (`exit`) : le menu se ferme. Dispatch, parcours `docs/e2e.md`.

### Task 1 : `context_menu.rs` (pur)

**Interfaces :** `MenuItem::{Copy, Paste, SplitVertical, SplitHorizontal, ToggleZoom, RenameTab, NewTab, ClosePane}` avec `action() -> Action` et `label(zoomed) -> &str` ; `MenuEntry::{Item(MenuItem), Separator}` ; `ENTRIES` ; `ContextMenu { term, x, y, can_copy, zoomed, hovered: Option<usize> }` ; `shortcut_for(&KeyMap, Action) -> Option<String>` ; `MenuLayout { rect, rows: Vec<(usize, PixelRect)> }` ; `layout(&ContextMenu, viewport, CellMetrics, &[Option<String>]) -> MenuLayout` ; `MenuLayout::hit(&self, &ContextMenu, x, y) -> Option<usize>` (index d'entrée cliquable) ; `menu_chrome(&ContextMenu, &MenuLayout, &[Option<String>], &Palette, CellMetrics) -> Chrome`.

- [ ] Tests : `entries_map_to_config_actions`, `zoom_label_follows_the_state`, `shortcut_is_the_shortest_binding`, `menu_opens_at_the_click`, `menu_stays_inside_the_window`, `hit_finds_items_by_row`, `separators_and_disabled_items_are_not_clickable`, `chrome_highlights_the_hovered_item_and_greys_disabled_ones`. — échec — implémentation — vert — commit `rustty : menu contextuel, modèle et dessin (0.1.0-alpha.91)`.

### Task 2 : Dispatch, rendu, documentation

**Files :** create `crates/rustty/src/menu_input.rs` ; modify `input.rs`, `window_state.rs`, `render.rs`, `app.rs`, `wheel_input.rs`, `README.md`, `docs/e2e.md`.

- [ ] Clic droit → ouverture ; mouvement → survol ; clic gauche → exécution ou fermeture ; Échap / autre touche / perte du focus / redimensionnement → fermeture ; dessin dans la passe finale ; README et e2e. Vérification : suite verte, test de fumée. Commit `rustty : menu contextuel au clic droit dans les panneaux (0.1.0-alpha.92)`, push, PR, CI, revue finale.
