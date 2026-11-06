# Barre de titre — jalon 3 « Titre et couleur » : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Renommer la fenêtre en place (double clic sur le titre, action `rename_window`), donner à la barre une couleur de session depuis le nuancier ouvert sous le bouton couleur, et écrire cette couleur dans `[titlebar] background` du TOML par l'entrée « Par défaut » sans toucher au reste du fichier.

**Architecture:** Le modèle pur (`Model`) gagne un nom de fenêtre (`window_name`) et une couleur de session (`titlebar_color`, index ANSI du nuancier) ; le renommage réutilise `Rename` avec une cible `RenameTarget::Window`, le nuancier réutilise `ContextMenu` avec une cible `MenuTarget::TitleBar`. Le rendu reçoit la couleur de session par `TitleBarState.background`. L'écriture du TOML est une fonction pure sur le texte (`toml_edit`) plus une écriture atomique, exécutée comme effet ; le veilleur de configuration recharge ensuite.

**Tech Stack:** Rust 2024, winit, wgpu, `toml_edit` (nouvelle dépendance de `rustty`), `rustty_config::Rgba8`.

**Spec:** `docs/superpowers/specs/2026-11-01-titlebar-design.md` (sections « Actions », « Écriture dans le TOML », « Tests », jalon 3).

## Global Constraints

- Règles du dépôt : code en anglais, commentaires et commits en français, TDD, aucune mention d'assistant dans les commits.
- Chaque commit bumpe la version alpha dans `Cargo.toml` racine (`cargo update -w` ensuite) et ajoute une entrée `## 0.1.0-alpha.N — 2026-11-06 · « titre »` en tête de `CHANGELOG.md`. Première version du plan : alpha.158 (commit du plan), puis une par tâche.
- Avant chaque commit : `cargo fmt --all`, `cargo clippy --workspace --all-targets` (0 avertissement), `cargo test --workspace` depuis la racine ; `git add` ciblé.
- Libellés exacts de la spec : entrées « Par défaut » et « Couleur du fichier » ; actions `rename_window` (raccourci par défaut `ctrl+shift+alt+w`) et `pick_titlebar_color` (aucun raccourci) ; nom de fenêtre limité à 64 caractères, vide = retour au titre du shell ; curseur `▌`.
- « Seul « Par défaut » du nuancier écrit » ; écriture atomique (fichier temporaire puis renommage) ; fichier absent → créé depuis le modèle ; échec → bandeau d'erreur ; jamais reformater le TOML au-delà de la clé visée.
- Anti-objectif : les images de référence (`title_bar.png`, `tab_bar.png`) restent vertes avec `[titlebar]` à ses défauts.
- Ne jamais tuer un processus nommé `rustty` (la session tourne dedans).

## Review Focus

1. Fichier de configuration avec `[titlebar]` en table en ligne (`titlebar = { enabled = true }`) : « Par défaut » doit y poser `background` sans casser la forme (test tâche 5 `an_inline_titlebar_table_keeps_its_form`).
2. Fichier TOML invalide au moment de « Par défaut » : rien n'est écrit, bandeau d'erreur, couleur de session conservée (test tâche 5 `an_invalid_file_is_left_alone`).
3. Dossier de configuration non inscriptible : l'écriture atomique échoue sans laisser de fichier temporaire (test tâche 5 `a_failed_write_leaves_no_temporary_file`).
4. Rechargement de `[colors]` avec une couleur de session : la barre suit la nouvelle palette (la couleur est un index ANSI, pas un RGB figé — test tâche 3 `a_session_color_replaces_the_background` côté rendu, et ruling ci-dessous).
5. Fin d'un terminal pendant que le nuancier de la barre est ouvert : le nuancier ne vise aucun onglet et reste ouvert (test tâche 3 `the_title_bar_palette_survives_a_terminal_exit`).

**Ruling de conception (à reporter dans le ledger) :** la spec écrit `Model.titlebar_color: Option<Rgb>` ; le plan stocke l'index ANSI (`Option<usize>`) pour garder le modèle sans palette et faire suivre la couleur de session aux rechargements de `[colors]`. La couleur est résolue (`palette.ansi[slot]`) au rendu et à l'écriture. Coût si faux : un `Option<Rgb>` se substitue en une tâche.

---

### Task 1: Nom de fenêtre dans le modèle et action `rename_window`

**Files:**
- Modify: `crates/rustty/src/rename.rs` (`RenameTarget::Window`)
- Modify: `crates/rustty/src/model.rs` (champ `window_name`, arme `Action::RenameWindow`)
- Modify: `crates/rustty/src/model/close.rs` (`start_window_rename`, commit `Window`)
- Modify: `crates/rustty-config/src/action.rs`, `crates/rustty-config/src/keymap.rs`
- Modify: `crates/rustty-config/src/example.rs`, `docs/rustty.example.toml` (liste des actions, ligne `"ctrl+shift+alt+w" = "rename_window"`, clés du test de documentation)
- Test: `crates/rustty/src/model_tests.rs`, tests inline de `action.rs`, `keymap.rs`

**Interfaces:**
- Produces: `RenameTarget::Window` ; `Model.window_name: Option<String>` (pub) ; `Model::start_window_rename(&mut self) -> Vec<Effect>` (pré-rempli avec le nom courant, rend `vec![Effect::Redraw]`) ; `rename_key` sur une cible `Window` : `Commit(title)` → `window_name = title`, rend `vec![Effect::Relayout]` ; `Action::RenameWindow` (`"rename_window"`), `Action::PickTitleBarColor` (`"pick_titlebar_color"`) ; `Model::apply(Action::PickTitleBarColor)` rend `vec![Effect::PickTitleBarColor]` (variante ajoutée ici, exécutée en tâche 4).

- [ ] **Step 1: Tests du modèle (RED)**

```rust
#[test]
fn renaming_the_window_sets_its_name() {
    let (mut m, _) = Model::new(0.9, false);
    assert_eq!(m.start_window_rename(), vec![Effect::Redraw]);
    assert_eq!(m.renaming.as_ref().map(|r| r.target), Some(RenameTarget::Window));
    m.rename_key(RenameKey::Text("build".into()));
    assert_eq!(m.rename_key(RenameKey::Commit), vec![Effect::Relayout]);
    assert_eq!(m.window_name.as_deref(), Some("build"));
    assert!(m.renaming.is_none());
}

#[test]
fn an_empty_window_name_returns_to_the_shell_title() {
    let (mut m, _) = Model::new(0.9, false);
    m.window_name = Some("build".into());
    m.start_window_rename();
    assert_eq!(m.renaming.as_ref().unwrap().buffer, "build", "pré-rempli");
    for _ in 0..5 { m.rename_key(RenameKey::Backspace); }
    m.rename_key(RenameKey::Commit);
    assert_eq!(m.window_name, None);
}

#[test]
fn the_rename_window_action_starts_editing() {
    let (mut m, _) = Model::new(0.9, false);
    m.apply(Action::RenameWindow, false, &never_running);
    assert_eq!(m.renaming.as_ref().map(|r| r.target), Some(RenameTarget::Window));
    assert_eq!(
        m.apply(Action::PickTitleBarColor, false, &never_running),
        vec![Effect::PickTitleBarColor]
    );
}
```

Tests config, dans `action.rs` (`parse("\"rename_window\"")`, `parse("\"pick_titlebar_color\"")`) et `keymap.rs` (`r("ctrl+shift+alt+w") == Some(Action::RenameWindow)`), dans le style des tests voisins ; ajouter `"rename_window"` et `"pick_titlebar_color"` aux clés du test de documentation de `example.rs`.

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty renaming_the_window 2>&1 | tail -5`
Expected: erreur de compilation (`RenameTarget::Window`, `start_window_rename`, `window_name` inexistants).

- [ ] **Step 3: Implémenter**

`RenameTarget::Window` ; `Model.window_name: Option<String>` (initialisé `None`) ; `start_window_rename` ; branche `RenameTarget::Window => self.window_name = title` dans `rename_key` ; `Action::RenameWindow => self.start_window_rename()` et `Action::PickTitleBarColor => vec![Effect::PickTitleBarColor]` dans `apply` (la variante `Effect::PickTitleBarColor` est ajoutée avec un `todo!()`-libre : branche `Effect::PickTitleBarColor => {}` provisoire dans `run_effects`, remplacée en tâche 4). Dans `apply`, la remise à zéro de `renaming` garde sa condition (`action != Action::RenameTab`) : `RenameWindow` ouvre de toute façon une nouvelle édition. Config : variantes `RenameWindow` / `PickTitleBarColor` dans `Action` et `Simple`, liaison par défaut `("ctrl+shift+alt+w", Action::RenameWindow)` après `rename_tab`, modèle et exemple TOML.

- [ ] **Step 4: Vérifier le succès, puis la suite complète**

Run: `cargo test --workspace 2>&1 | grep -E 'test result|FAILED|panicked' | sort | uniq -c`
Expected: tout `ok`, 0 failed.

- [ ] **Step 5: Commit** (alpha.159, « Nom de fenêtre : cible de renommage et action rename_window »)

```bash
git add crates/rustty/src/rename.rs crates/rustty/src/model.rs crates/rustty/src/model/close.rs crates/rustty/src/model_tests.rs crates/rustty/src/effects.rs crates/rustty-config/src/action.rs crates/rustty-config/src/keymap.rs crates/rustty-config/src/example.rs docs/rustty.example.toml Cargo.toml Cargo.lock CHANGELOG.md
```

---

### Task 2: Double clic sur le titre et affichage de l'édition dans la barre

**Files:**
- Modify: `crates/rustty/src/title.rs` (`bar_title`)
- Modify: `crates/rustty/src/title_bar.rs` (`window_title_text`)
- Modify: `crates/rustty/src/title_input.rs` (`title_release`, `title_drag_moved`), `crates/rustty/src/window_state.rs` (champ `title_clicks: DoubleClick`)
- Test: `crates/rustty/src/title.rs` (inline), `crates/rustty/src/title_input_tests.rs`

**Interfaces:**
- Consumes: `Model.window_name`, `Model::start_window_rename`, `RenameTarget::Window` (tâche 1) ; `DoubleClick::register(target, now)` / `reset` (`crate::mouse::DoubleClick`, cible `0`).
- Produces: `title::bar_title(window_name: Option<&str>, tab_name: Option<&str>, shell_title: &str, editing: Option<&str>, version: &str) -> String` ; `title_input::title_click_renames(event: TitleDragEvent, target: TitleTarget) -> bool`.

- [ ] **Step 1: Tests (RED)**

```rust
// title.rs
#[test]
fn the_bar_title_prefers_the_window_name_then_the_tab_name() {
    assert_eq!(bar_title(Some("deploy"), Some("build"), "zsh", None, "1.0"), "deploy — rustty 1.0");
    assert_eq!(bar_title(None, Some("build"), "zsh", None, "1.0"), "build — rustty 1.0");
    assert_eq!(bar_title(None, None, "zsh", None, "1.0"), "zsh — rustty 1.0");
}

#[test]
fn editing_the_window_name_shows_the_buffer_and_a_cursor() {
    assert_eq!(bar_title(Some("deploy"), None, "zsh", Some("dep"), "1.0"), "dep▌");
    assert_eq!(bar_title(None, None, "", Some(""), "1.0"), "▌");
}

// title_input_tests.rs
#[test]
fn only_a_click_on_the_title_counts_towards_a_rename() {
    assert!(title_click_renames(TitleDragEvent::Click, TitleTarget::Title));
    assert!(!title_click_renames(TitleDragEvent::Click, TitleTarget::Background));
    assert!(!title_click_renames(TitleDragEvent::None, TitleTarget::Title));
    assert!(!title_click_renames(TitleDragEvent::StartMove, TitleTarget::Title));
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty bar_title title_click_renames 2>&1 | tail -5`
Expected: erreur de compilation (fonctions inexistantes).

- [ ] **Step 3: Implémenter**

`bar_title` : `editing` → `format!("{buffer}▌")`, sinon `window_title(window_name.or(tab_name), shell_title, version)`. `window_title_text` passe `self.model.window_name`, le `custom_title` de l'onglet actif, le titre du shell et le tampon si `renaming.target == RenameTarget::Window` (le titre winit suit donc l'édition : `rename_key` rend `Relayout`, et `relayout()` appelle `update_title()`). Dans `title_release` : `let event = self.title_drag.release();` puis, si `title_click_renames(event, target_at_release)` et `self.title_clicks.register(0, Instant::now())` → `return Some(self.model.start_window_rename())`. Dans `title_drag_moved`, sur `StartMove` : `self.title_clicks.reset()`. `leave_title_bar` remet aussi `title_clicks` à zéro.

- [ ] **Step 4: Vérifier le succès**

Run: `cargo test --workspace 2>&1 | grep -E 'test result|FAILED|panicked' | sort | uniq -c`
Expected: tout `ok`.

- [ ] **Step 5: Commit** (alpha.160, « Double clic sur le titre : renommage de la fenêtre en place »)

---

### Task 3: Nuancier de la barre et couleur de session

**Files:**
- Modify: `crates/rustty/src/context_menu/entries.rs` (`MenuTarget::TitleBar`, `MenuItem::SaveDefaultColor`, `MenuItem::ConfigColor`, entrées)
- Modify: `crates/rustty/src/context_menu/mod.rs` (`ContextMenu::title_bar`, `dropped_by_exit_of`)
- Modify: `crates/rustty/src/model.rs` (`titlebar_color`, `Effect::SaveTitleBarColor(usize)`), `crates/rustty/src/model/menu.rs` (`title_bar_command`)
- Modify: `crates/rustty-render/src/chrome/title_bar.rs` (`TitleBarState.background: Option<Rgba>`), `crates/rustty/src/title_bar.rs` (la passe)
- Test: `crates/rustty/src/context_menu/mod_tests.rs`, `crates/rustty/src/model/menu_tests.rs`, `crates/rustty-render/src/chrome/title_bar_tests.rs` (+ `background: None` dans les trois littéraux existants, et dans `tests/offscreen.rs` s'il en construit un)

**Interfaces:**
- Produces: `MenuTarget::TitleBar` ; `MenuItem::SaveDefaultColor` (libellé « Par défaut »), `MenuItem::ConfigColor` (« Couleur du fichier »), tous deux sans `action()` ni `swatch()` ; `entries(&MenuTarget::TitleBar)` = 12 `Color(slot)` de `ACCENT_SLOTS`, `Separator`, `SaveDefaultColor`, `ConfigColor` ; `ContextMenu::title_bar(x: u32, y: u32) -> Self` (sans raccourcis) ; `Model.titlebar_color: Option<usize>` (pub, index ANSI) ; `Effect::SaveTitleBarColor(usize)` ; `TitleBarState.background: Option<Rgba>` (couleur de session, `None` = celle du style).

- [ ] **Step 1: Tests (RED)**

```rust
// mod_tests.rs
#[test]
fn the_title_bar_palette_lists_the_accents_then_default_and_file() {
    let menu = ContextMenu::title_bar(10, 28);
    let items: Vec<_> = menu.entries.iter().filter_map(|e| match e { MenuEntry::Item(i) => Some(*i), _ => None }).collect();
    assert_eq!(items.len(), 14);
    assert_eq!(items[12], MenuItem::SaveDefaultColor);
    assert_eq!(items[13], MenuItem::ConfigColor);
    assert_eq!(menu.label(MenuItem::SaveDefaultColor), "Par défaut");
    assert_eq!(menu.label(MenuItem::ConfigColor), "Couleur du fichier");
    assert!(menu.shortcuts.iter().all(Option::is_none));
}

#[test]
fn the_title_bar_palette_survives_a_terminal_exit() {
    assert!(!ContextMenu::title_bar(0, 0).dropped_by_exit_of(TermId(1)));
}

// menu_tests.rs
#[test]
fn a_swatch_gives_the_window_a_session_color() {
    let mut m = three_tabs();
    assert_eq!(m.menu_command(MenuTarget::TitleBar, MenuItem::Color(4), false, &idle), vec![Effect::Relayout]);
    assert_eq!(m.titlebar_color, Some(4));
    assert_eq!(m.menu_command(MenuTarget::TitleBar, MenuItem::ConfigColor, false, &idle), vec![Effect::Relayout]);
    assert_eq!(m.titlebar_color, None);
}

#[test]
fn default_writes_the_session_color_and_does_nothing_without_one() {
    let mut m = three_tabs();
    assert!(m.menu_command(MenuTarget::TitleBar, MenuItem::SaveDefaultColor, false, &idle).is_empty());
    m.titlebar_color = Some(2);
    assert_eq!(
        m.menu_command(MenuTarget::TitleBar, MenuItem::SaveDefaultColor, false, &idle),
        vec![Effect::SaveTitleBarColor(2)]
    );
    assert_eq!(m.titlebar_color, Some(2), "effacée par l'effet, après l'écriture");
}

// title_bar_tests.rs (rendu)
#[test]
fn a_session_color_replaces_the_background() {
    let style = TitleBarStyle::from_config(&TitleBar::default(), 1.0);
    let layout = layout_title_bar(400, &style);
    let red = Rgba::new(0.8, 0.1, 0.1, 1.0);
    let chrome = title_bar_chrome(&layout, &style, TitleBarState { title: "x", hover: None, maximized: false, background: Some(red) }, metrics());
    assert_eq!(chrome.shapes[0].style.fill_bottom, red, "le fond de la barre est la couleur de session");
    assert_eq!(chrome.shapes[0].style.fill, red.lighten(style.gloss));
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty title_bar_palette session_color && cargo test -p rustty-render a_session_color 2>&1 | tail -5`
Expected: erreurs de compilation (variantes, champ et constructeur inexistants).

- [ ] **Step 3: Implémenter**

Entrées et libellés ; `ContextMenu::title_bar(x, y) = Self::build(MenuTarget::TitleBar, x, y, None)` ; `dropped_by_exit_of` → `false` pour `TitleBar`. `Model::title_bar_command(item)` : `Color(slot)` → `titlebar_color = Some(slot)`, `ConfigColor` → `None`, les deux rendent `vec![Effect::Relayout]` ; `SaveDefaultColor` → `titlebar_color.map(|s| vec![Effect::SaveTitleBarColor(s)]).unwrap_or_default()` ; autres → vide. Branche provisoire `Effect::SaveTitleBarColor(_) => {}` dans `run_effects` (tâche 6). Rendu : `let background = state.background.unwrap_or(style.background);` utilisé partout où `style.background` l'était dans `title_bar_chrome` (fond, voile de survol, lisibilité des icônes et du titre). `OsWindow::title_bar_chrome` passe `self.model.titlebar_color.map(|s| self.palette.ansi[s])`.

- [ ] **Step 4: Vérifier le succès (images de référence comprises)**

Run: `cargo test --workspace 2>&1 | grep -E 'test result|FAILED|panicked' | sort | uniq -c`
Expected: tout `ok` ; `tests/golden/title_bar.png` inchangée.

- [ ] **Step 5: Commit** (alpha.161, « Nuancier de la barre de titre : couleur de session »)

---

### Task 4: Ouvrir le nuancier sous le bouton couleur et par `pick_titlebar_color`

**Files:**
- Modify: `crates/rustty/src/title_input.rs` (`palette_anchor`, `open_title_bar_palette`, `title_button_action`)
- Modify: `crates/rustty/src/effects.rs` (`Effect::PickTitleBarColor => self.open_title_bar_palette()`)
- Modify: `crates/rustty/src/menu_input.rs` si nécessaire (aucun cas `ChangeColor` pour la cible `TitleBar`)
- Test: `crates/rustty/src/title_input_tests.rs`

**Interfaces:**
- Consumes: `ContextMenu::title_bar(x, y)` (tâche 3), `TitleBarLayout { bar, buttons, title, .. }` (`rustty_render`), `Model::accepts_context_menu`.
- Produces: `title_input::palette_anchor(layout: &TitleBarLayout) -> (u32, u32)` : coin bas-gauche du bouton `Color` s'il est disposé, sinon `(layout.title.x, layout.bar.height)` ; `OsWindow::open_title_bar_palette(&mut self) -> Vec<Effect>` (rien sans barre maison ou si `!accepts_context_menu()`, sinon `menu = Some(ContextMenu::title_bar(x, y))`, annule un glisser, rend `vec![Effect::Redraw]`).

- [ ] **Step 1: Test (RED)**

```rust
#[test]
fn the_palette_hangs_under_the_color_button_or_the_title() {
    let style = TitleBarStyle::from_config(&TitleBar::default(), 1.0);
    let layout = layout_title_bar(400, &style);
    let (_, rect) = layout.buttons.iter().find(|(b, _)| *b == TitleButton::Color).unwrap();
    assert_eq!(palette_anchor(&layout), (rect.x, rect.y + rect.height));
    let mut cfg = TitleBar::default();
    cfg.buttons.color.visible = Some(false); // ou la forme exacte de la surcharge `visible` dans rustty-config
    let hidden = layout_title_bar(400, &TitleBarStyle::from_config(&cfg, 1.0));
    assert_eq!(palette_anchor(&hidden), (hidden.title.x, hidden.bar.height));
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty the_palette_hangs 2>&1 | tail -5`
Expected: erreur de compilation (`palette_anchor` inexistant).

- [ ] **Step 3: Implémenter**

`palette_anchor` ; `title_button_action` : `TitleButton::Color => self.open_title_bar_palette()` (retirer `Color` de la branche vide de `button_effects`, qui rend désormais `Vec::new()` pour `Close` seul) ; `run_effects` : `Effect::PickTitleBarColor => { let e = self.open_title_bar_palette(); self.run_effects(e, event_loop) }` ou, plus simple, appliquer le `Redraw` directement. Vérifier à la main dans `input.rs::on_press` que `menu_click` précède `title_press` : un clic sur le bouton couleur avec le nuancier ouvert ferme le nuancier (`MenuClick::Close`) sans le rouvrir — comportement voulu, comme un menu ordinaire.

- [ ] **Step 4: Vérifier le succès**

Run: `cargo test --workspace 2>&1 | grep -E 'test result|FAILED|panicked' | sort | uniq -c`
Expected: tout `ok`.

- [ ] **Step 5: Commit** (alpha.162, « Le bouton couleur ouvre le nuancier de la barre »)

---

### Task 5: Écriture de `[titlebar] background` avec `toml_edit`

**Files:**
- Modify: `Cargo.toml` (workspace : `toml_edit = "0.25"` ; vérifier avec `cargo add toml_edit -p rustty --dry-run` que la dernière 0.25.x s'installe sans réseau depuis le cache, sinon prendre la version du cache `~/.cargo/registry/cache`), `crates/rustty/Cargo.toml` (`toml_edit.workspace = true`)
- Create: `crates/rustty/src/config_write.rs`, `crates/rustty/src/config_write_tests.rs` ; `mod config_write;` dans `main.rs`

**Interfaces:**
- Produces: `config_write::with_titlebar_background(toml: &str, color: Rgba8) -> Result<String, toml_edit::TomlError>` ; `config_write::write_atomically(path: &Path, contents: &str) -> std::io::Result<()>` ; `config_write::rgba8_of(color: rustty_render::Rgba) -> Rgba8`.

- [ ] **Step 1: Tests (RED)** — dans `config_write_tests.rs`

```rust
const SAMPLE: &str = "# mon terminal\n[window]\nopacity = 0.9 # translucide\n\n[titlebar]\n# fond\nbackground = \"#141414\"\nheight = 28\n";

#[test]
fn the_background_key_is_replaced_in_place_and_nothing_else_moves() {
    let out = with_titlebar_background(SAMPLE, Rgba8::new(0xcc, 0x00, 0x00, 0xff)).unwrap();
    assert_eq!(out, SAMPLE.replace("\"#141414\"", "\"#cc0000\""));
}

#[test]
fn a_missing_key_or_section_is_added() {
    let out = with_titlebar_background("[titlebar]\nheight = 28\n", Rgba8::new(0, 0, 0xff, 0xff)).unwrap();
    assert!(out.contains("background = \"#0000ff\""));
    let out = with_titlebar_background("[window]\nopacity = 0.9\n", Rgba8::new(0, 0, 0xff, 0xff)).unwrap();
    assert!(out.starts_with("[window]\nopacity = 0.9\n"));
    assert!(out.contains("[titlebar]\nbackground = \"#0000ff\""));
    assert!(with_titlebar_background("", Rgba8::new(1, 2, 3, 0x80)).unwrap().contains("\"#01020380\""));
}

#[test]
fn an_inline_titlebar_table_keeps_its_form() {
    let out = with_titlebar_background("titlebar = { enabled = true }\n", Rgba8::new(0, 0, 0, 0xff)).unwrap();
    assert!(out.contains("titlebar = { enabled = true, background = \"#000000\" }"), "{out}");
    assert!(rustty_config::Config::from_str(&out).is_ok());
}

#[test]
fn an_invalid_file_is_left_alone() {
    assert!(with_titlebar_background("[titlebar\n", Rgba8::new(0, 0, 0, 0xff)).is_err());
}

#[test]
fn atomic_write_replaces_the_file_and_leaves_no_temporary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rustty.toml");
    std::fs::write(&path, "old").unwrap();
    write_atomically(&path, "new").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn a_failed_write_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent").join("rustty.toml");
    assert!(write_atomically(&path, "x").is_err());
    assert!(!dir.path().join("absent").exists());
}

#[test]
fn palette_colors_round_trip_to_rgba8() {
    let c = Rgba8::new(0x3d, 0xc8, 0x4f, 0xff);
    assert_eq!(rgba8_of(rustty_render::Rgba::from_rgba8(c)), c);
}
```

Vérifier que `tempfile` est déjà une dépendance de dev de `rustty` (sinon l'ajouter au workspace comme dev-dependency).

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty config_write 2>&1 | tail -5`
Expected: erreur de compilation (module inexistant).

- [ ] **Step 3: Implémenter**

`with_titlebar_background` : `toml.parse::<toml_edit::DocumentMut>()?`, puis `doc["titlebar"]["background"] = toml_edit::value(color.to_string())` — si `titlebar` n'existe pas, `doc["titlebar"] = toml_edit::table()` d'abord (une table en ligne existante est laissée en ligne par l'indexation) ; rendre `doc.to_string()`. `write_atomically` : fichier `<nom>.tmp-<pid>` dans le même dossier, `write` puis `fs::rename` ; en cas d'échec de l'écriture, retirer le temporaire avant de rendre l'erreur. `rgba8_of` : `Rgba::to_u8` → `Rgba8::new`.

- [ ] **Step 4: Vérifier le succès**

Run: `cargo test --workspace 2>&1 | grep -E 'test result|FAILED|panicked' | sort | uniq -c`
Expected: tout `ok`.

- [ ] **Step 5: Commit** (alpha.163, « Écriture ciblée de [titlebar] background avec toml_edit »)

---

### Task 6: Effet « Par défaut » : écriture, veilleur, bandeau

**Files:**
- Modify: `crates/rustty/src/effects.rs` (`save_titlebar_color`)
- Modify: `crates/rustty/src/window_state.rs` si `apply_config` doit effacer la couleur de session (non : elle reste, `Couleur du fichier` la retire)

**Interfaces:**
- Consumes: `Effect::SaveTitleBarColor(slot)` (tâche 3), `config_write::*` (tâche 5), `external::ensure_config_file`, `UserEvent::ConfigFileCreated`, `report_external`.

- [ ] **Step 1: Test (RED)** — la résolution du chemin et l'enchaînement lecture → remplacement → écriture sont des fonctions pures déjà testées ; pinner ici la seule décision nouvelle, dans `config_write_tests.rs` :

```rust
#[test]
fn saving_rewrites_only_the_background_of_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rustty.toml");
    std::fs::write(&path, "[titlebar]\nheight = 30 # haute\n").unwrap();
    save_background(&path, Rgba8::new(0x11, 0x22, 0x33, 0xff)).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "[titlebar]\nheight = 30 # haute\nbackground = \"#112233\"\n");
}

#[test]
fn saving_creates_a_missing_file_from_the_template_first() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("rustty.toml");
    save_background(&path, Rgba8::new(0x11, 0x22, 0x33, 0xff)).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("background = \"#112233\""));
    assert!(text.contains("[keys]"), "le modèle complet, pas un fichier vide");
}
```

`config_write::save_background(path: &Path, color: Rgba8) -> std::io::Result<()>` : `external::ensure_config_file(path)?`, lecture, `with_titlebar_background` (erreur TOML → `io::Error::other`), `write_atomically`.

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty saving_ 2>&1 | tail -5`
Expected: erreur de compilation (`save_background` inexistant).

- [ ] **Step 3: Implémenter**

`save_background` dans `config_write.rs`. Dans `effects.rs`, `Effect::SaveTitleBarColor(slot) => self.save_titlebar_color(slot)` : chemin = `config_path` ou `Config::default_path()` (sinon erreur « dossier de configuration introuvable »), `color = rgba8_of(self.palette.ansi[slot])`, `save_background`, puis `self.waker.wake(UserEvent::ConfigFileCreated)` ; en cas de succès : `self.model.titlebar_color = None`, `self.config.titlebar.background = color`, `self.rebuild_title_style()`, `request_redraw` (pas de clignotement avant le rechargement du veilleur) ; en cas d'échec : `report_external("enregistrer la couleur", result)`, couleur de session conservée.

- [ ] **Step 4: Vérifier le succès**

Run: `cargo test --workspace 2>&1 | grep -E 'test result|FAILED|panicked' | sort | uniq -c`
Expected: tout `ok`.

- [ ] **Step 5: Commit** (alpha.164, « « Par défaut » écrit la couleur de la barre dans la configuration »)

---

### Task 7: Documentation du jalon 3

**Files:**
- Modify: `README.md` (puce « Barre de titre » : double clic renomme, couleur ouvre le nuancier ; paragraphe `[titlebar]` : `color` n'attend plus, actions `rename_window` (`ctrl+shift+alt+w`) et `pick_titlebar_color`, « Par défaut » écrit `background` sans toucher au reste du fichier)
- Modify: `docs/e2e.md` section « Barre de titre » : cases à cocher pour double clic (Entrée, Échap, nom vide, 64 caractères, titre winit identique pendant l'édition), raccourci `ctrl+shift+alt+w`, nuancier sous le bouton (12 pastilles, « Par défaut », « Couleur du fichier »), persistance de la couleur de session au rechargement de `[colors]`, « Par défaut » avec fichier absent / commentaires conservés / fichier en lecture seule (bandeau), fermeture du nuancier par un clic ailleurs et par Échap
- Modify: `docs/superpowers/specs/2026-11-01-titlebar-design.md` : une ligne sous « Couleur » notant que la couleur de session est stockée comme index ANSI (ruling)
- Modify: `CHANGELOG.md`, `Cargo.toml`

- [ ] **Step 1: Rédiger, puis vérifier le test de documentation et la suite**

Run: `cargo test --workspace 2>&1 | grep -E 'test result|FAILED|panicked' | sort | uniq -c`
Expected: tout `ok` (`every_section_and_key_is_documented` compris).

- [ ] **Step 2: Commit** (alpha.165, « Barre de titre : documentation du jalon 3 »)

---

## Après les tâches

Revue finale par un sous-agent (modèle le plus capable) sur `git merge-base main HEAD..HEAD`, avec la section Review Focus et les rulings du ledger ; correctifs Critical/Important en une passe TDD ; puis `superpowers:finishing-a-development-branch` : PR vers `main`, CI verte, fusion, tag `v0.1.0-alpha.N`, pré-release GitHub avec la section CHANGELOG, `cargo install --path crates/rustty --force`.
