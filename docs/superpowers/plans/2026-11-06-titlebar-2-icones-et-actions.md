# Barre de titre — jalon 2 « Icônes et actions » : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** La barre de titre dessine ses six boutons avec les glyphes Material Symbols Rounded embarqués, et les boutons réglages et dossier ouvrent le TOML dans l'éditeur et le dossier courant du shell, aussi depuis `[keys]`.

**Architecture:** Une police Material réduite à sept glyphes est embarquée dans `rustty-render` et chargée dans une face dédiée, à `icon_size` px logiques × `scale_factor`, indépendante de la police de texte ; les `ChromeIcon` (jalon 1) passent par cette face via la clé du cache de glyphes. Les actions externes sont deux effets (`OpenConfig`, `OpenCwd`) exécutés par la fenêtre : résolution pure de la commande (clé TOML, `$VISUAL`, `$EDITOR`, ouvreur du bureau), lancement détaché, échec dans le bandeau d'erreur.

**Tech Stack:** Rust 2024, winit, wgpu, swash (rastérisation), fonttools (script de génération de la police, venv jetable), `directories` (dossier personnel).

**Spec:** `docs/superpowers/specs/2026-11-01-titlebar-design.md` (sections Aspect, Actions, Rendu, Configuration, Tests ; jalon 2 de la section Jalons). Jalon 1 livré en alpha.149 par `docs/superpowers/plans/2026-11-01-titlebar-1-barre-et-fenetre.md`.

## Global Constraints

- Toutes les tailles de `[titlebar]` sont en **pixels logiques**, multipliées par le `scale_factor` à la mise en page et à chaque `ScaleFactorChanged` ; les bornes de validation s'entendent dans cette unité.
- Validation : `icon_size` 8 à 32 ; clé inconnue → erreur positionnée.
- Rechargement à chaud : tout change sans redémarrer (`icon_size`, `editor`, `file_manager` compris).
- Police `MaterialSymbolsRounded-subset.ttf` (Apache 2.0, `LICENSE` à côté), **sept glyphes** : `settings`, `palette`, `folder_open`, `remove`, `crop_square`, `filter_none`, `close` ; régénérée par `scripts/material-icons.sh` (fonttools dans un venv jetable). Chargée dans une face **dédiée** : jamais le repli système pour les codepoints en zone privée.
- Pas de passe ni de renderer supplémentaire pour les icônes.
- Anti-objectifs : ne jamais bloquer l'interface en lançant l'éditeur ou l'explorateur (processus détaché, sortie ignorée) ; ne pas changer d'un pixel la barre d'onglets ni les panneaux (`tests/golden/tab_bar.png` reste verte) ; ne jamais toucher au thème du bureau.
- Modèle `--init-config`, `docs/rustty.example.toml`, README et le test `every_section_and_key_is_documented` couvrent chaque clé nouvelle.
- Règles du dépôt : code en anglais, commentaires et commits en français ; `cargo fmt --all`, `cargo clippy --workspace --all-targets` sans avertissement, `cargo test --workspace` vert avant chaque commit ; chaque commit bumpe la version dans `Cargo.toml` (racine, `cargo update -w`) et ajoute une entrée `CHANGELOG.md` en tête `## 0.1.0-alpha.N — AAAA-MM-JJ · « titre »` ; `git add` ciblé ; aucune mention d'assistant.
- Ne jamais lancer `pkill`/`killall rustty` (le terminal de développement est rustty) ; un binaire de test se tue par PID.

## Review Focus

1. Écran HiDPI : les icônes sont rastérisées à `icon_size × scale_factor`, indépendamment de la taille de la police de texte → tests `icon_glyphs_scale_with_the_icon_size_not_the_text_size` (tâche 3) et `style_scales_logical_sizes_to_physical` (tâche 4).
2. `editor = ""` avec `$VISUAL` et `$EDITOR` absents ou vides → ouvreur du bureau ; une valeur avec arguments (`"code --wait"`) est découpée → tests `editor_falls_back_from_key_to_env_to_opener` et `blank_values_are_skipped` (tâche 5).
3. Fichier TOML absent au clic sur réglages → créé depuis le modèle, jamais écrasé s'il existe → test `config_file_is_created_once_then_left_alone` (tâche 5).
4. Panneau sans dossier lisible, ou aucun panneau → le dossier personnel, jamais une erreur → test `folder_to_open_falls_back_to_home` (tâche 5).
5. Fond de barre clair → titre et icônes des boutons d'outils assombris, boutons de fenêtre inchangés ; bouton d'outil `visible = false` → la zone de titre s'élargit → tests `light_background_darkens_title_and_tool_icons` et `hidden_tool_buttons_free_the_title_zone` (tâche 4).

---

### Task 1 : police Material embarquée

**Files:**
- Create: `scripts/material-icons.sh`
- Create: `crates/rustty-render/assets/MaterialSymbolsRounded-subset.ttf`, `crates/rustty-render/assets/LICENSE-MaterialSymbols`
- Create: `crates/rustty-render/src/font/icons.rs` ; Modify: `crates/rustty-render/src/font/mod.rs` (`pub mod icons;` et ré-export `pub use icons::{IconFace, glyphs};`), `crates/rustty-render/src/lib.rs` (ré-export `IconFace`, `glyphs` sous le nom `icons`)
- Test: tests dans `icons.rs`

**Interfaces:**
- Consumes: `FaceData` et `glyph_in` de `font/loader.rs` (rendre `glyph_in` `pub(crate)`).
- Produces:

```rust
// font/icons.rs
pub const ICON_FONT: &[u8] = include_bytes!("../../assets/MaterialSymbolsRounded-subset.ttf");
pub mod glyphs {
    pub const SETTINGS: char = '\u{e8b8}';
    pub const PALETTE: char = '\u{e40a}';
    pub const FOLDER_OPEN: char = '\u{e2c8}';
    pub const REMOVE: char = '\u{e15b}';
    pub const CROP_SQUARE: char = '\u{e3c6}';
    pub const FILTER_NONE: char = '\u{e3e0}';
    pub const CLOSE: char = '\u{e5cd}';
    pub const ALL: [char; 7];
}
pub struct IconFace { /* face: FaceData, size_px: f32 */ }
impl IconFace {
    pub fn new(size_px: f32) -> Self;        // police embarquée, toujours valide
    pub fn size_px(&self) -> f32;
    pub fn face(&self) -> &FaceData;
    pub fn glyph_id(&self, ch: char) -> Option<u16>;   // None hors des sept glyphes
}
```

- [ ] **Étape 1 : écrire `scripts/material-icons.sh`** sur le modèle de `scripts/icons.sh` (`set -euo pipefail`, `cd` à la racine) : venv jetable dans `mktemp -d`, `pip install fonttools`, téléchargement (par `python3 -c` et `urllib`, pas `curl`) de `https://github.com/google/material-design-icons/raw/master/variablefont/MaterialSymbolsRounded%5BFILL%2CGRAD%2Copsz%2Cwght%5D.ttf` et du `.codepoints` voisin, extraction des sept codepoints depuis le fichier `.codepoints` (nom → hexa), `fonttools varLib.instancer` à `wght=400 FILL=0 GRAD=0 opsz=20`, puis `pyftsubset --unicodes=<liste>` vers `crates/rustty-render/assets/MaterialSymbolsRounded-subset.ttf` ; copie de la licence Apache 2.0 du dépôt dans `LICENSE-MaterialSymbols` ; affiche les sept couples `nom U+xxxx` et la taille du fichier.
- [ ] **Étape 2 : lancer le script** — `bash scripts/material-icons.sh` ; vérifier que les codepoints affichés sont ceux du bloc `glyphs` ci-dessus (sinon corriger le bloc, pas le script) et que le fichier fait moins de 10 Ko.
- [ ] **Étape 3 : tests qui échouent** dans `icons.rs` :

```rust
#[test]
fn every_icon_has_a_glyph_in_the_subset() {
    let face = IconFace::new(14.0);
    for ch in glyphs::ALL {
        assert!(face.glyph_id(ch).is_some(), "{:x}", u32::from(ch));
    }
}
#[test]
fn the_subset_holds_nothing_else() {
    let face = IconFace::new(14.0);
    assert_eq!(face.glyph_id('a'), None);
    assert_eq!(face.glyph_id('\u{e000}'), None, "zone privée hors des sept");
}
```

- [ ] **Étape 4 : vérifier l'échec** — `cargo test -p rustty-render --lib icons` → compilation impossible.
- [ ] **Étape 5 : implémenter `icons.rs`** (`IconFace::new` construit `FaceData { data: Arc::new(ICON_FONT.to_vec()), index: 0 }` ; `glyph_id` = `glyph_in`).
- [ ] **Étape 6 : vérifier** — `cargo test -p rustty-render --lib icons` → 2 tests verts.
- [ ] **Étape 7 : commit** — alpha.150 « Police Material Symbols embarquée » (script, assets, licence, module).

### Task 2 : clés `icon_size`, `editor`, `file_manager` et actions `open_config`, `open_cwd`

**Files:**
- Modify: `crates/rustty-config/src/titlebar.rs` (struct `TitleBar`, `Default`, `validate`), `crates/rustty-config/src/titlebar_tests.rs`
- Modify: `crates/rustty-config/src/action.rs` (`Action::{OpenConfig, OpenCwd}`, `Simple::{OpenConfig, OpenCwd}` ↔ `"open_config"`, `"open_cwd"`, `From`), tests du module
- Modify: `crates/rustty-config/src/example.rs` (section `[titlebar]` : trois clés commentées après `resize_border` ; liste des actions dans `[keys]` ; tableau du test `every_section_and_key_is_documented`), `docs/rustty.example.toml` (même texte : le test compare les deux)
- Modify: `crates/rustty/src/model.rs` (`Action::OpenConfig => vec![Effect::OpenConfig]`, `Action::OpenCwd => vec![Effect::OpenCwd]`, variantes `Effect::OpenConfig`, `Effect::OpenCwd`), `crates/rustty/src/effects.rs` (les deux bras renvoient `tracing::debug!` pour l'instant : la tâche 5 les remplit)

**Interfaces:**
- Produces: `TitleBar { icon_size: u32 /* 14 */, editor: String /* "code" */, file_manager: String /* "" */, .. }` ; `Action::OpenConfig`, `Action::OpenCwd` ; `Effect::OpenConfig`, `Effect::OpenCwd`. Aucun raccourci par défaut dans `keymap.rs`.

- [ ] **Étape 1 : tests qui échouent**

```rust
// titlebar_tests.rs
#[test]
fn icon_size_and_external_commands_have_the_spec_defaults() {
    let t = TitleBar::default();
    assert_eq!((t.icon_size, t.editor.as_str(), t.file_manager.as_str()), (14, "code", ""));
    let c = Config::from_str("[titlebar]\nicon_size = 20\neditor = \"\"\nfile_manager = \"nautilus\"\n").unwrap();
    assert_eq!((c.titlebar.icon_size, c.titlebar.editor.as_str(), c.titlebar.file_manager.as_str()), (20, "", "nautilus"));
}
// dans bounds_are_validated_with_the_key_name, ajouter la ligne :
("[titlebar]\nicon_size = 33\n", "titlebar.icon_size"),
// action.rs, test window_actions : ajouter
assert_eq!(parse("\"open_config\"").unwrap(), Action::OpenConfig);
assert_eq!(parse("\"open_cwd\"").unwrap(), Action::OpenCwd);
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty-config` → compilation impossible.
- [ ] **Étape 3 : implémenter** config, actions, modèle et effets ; `bounded("titlebar.icon_size", self.icon_size, 8..=32)` dans `validate`.
- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert (`every_section_and_key_is_documented` compris).
- [ ] **Étape 5 : commit** — alpha.151 « Clés icon_size, editor, file_manager et actions open_config, open_cwd ».

### Task 3 : face d'icônes dans le renderer

**Files:**
- Modify: `crates/rustty-render/src/renderer.rs` (champ `icons: IconFace`, créé à `IconFace::new(DEFAULT_ICON_PX)` avec `const DEFAULT_ICON_PX: f32 = 14.0` ; `pub fn set_icon_size(&mut self, px: f32)`)
- Modify: `crates/rustty-render/src/renderer/glyph_cache.rs` (`GlyphKey { ch, variant, icon: bool }` ; `cached_glyph(ctx, ch, variant, icon)` ; `load_glyph` : si `icon`, rastérise `self.icons.face()` à `self.icons.size_px()` avec `self.icons.glyph_id(ch)?`, sans `builtin_glyph` ni repli ; `icon_instance` demande `icon = true`)
- Modify: `crates/rustty/src/renderers.rs` (`RendererSpec { icon_px: f32, .. }`, `build` appelle `set_icon_size(self.icon_px)` ; `pub fn set_icon_size(&mut self, px: f32)` sur tous les renderers), `crates/rustty/src/window_state.rs` (`renderer_spec` : `icon_px: self.title_style.icon_size as f32`)
- Modify: `crates/rustty-render/src/chrome/title_bar.rs` (`TitleBarStyle { icon_size: u32 /* physique */ }` depuis `px(cfg.icon_size, scale)`), `crates/rustty/src/title_bar.rs` (`rebuild_title_style` appelle ensuite `self.renderers.set_icon_size(self.title_style.icon_size as f32)`)
- Test: `crates/rustty-render/src/renderer/tests.rs`

**Interfaces:**
- Consumes: `IconFace`, `glyphs` (tâche 1) ; `ChromeIcon` et `icon_instance` (jalon 1, alpha.149).
- Produces: `Renderer::set_icon_size(px)` (recrée la face si la taille change et retire du cache les entrées `icon = true`) ; `TitleBarStyle::icon_size`.

- [ ] **Étape 1 : test qui échoue** (GPU, ignoré sans adaptateur comme ses voisins) :

```rust
#[test]
fn icon_glyphs_scale_with_the_icon_size_not_the_text_size() {
    let Ok(ctx) = GpuContext::headless() else { eprintln!("test GPU ignoré"); return; };
    let mut r = Renderer::with_atlas_size(&ctx, OFFSCREEN_FORMAT, FontSet::embedded(16.0),
        Palette::from_config(&Colors::default(), false), 0, 512);
    let icon = |ch| crate::frame::ChromeIcon { x: 100, y: 50, ch, color: Rgba::new(1.0, 1.0, 1.0, 1.0) };
    r.set_icon_size(14.0);
    let small = r.icon_instance(&ctx, &icon(crate::font::glyphs::CLOSE)).unwrap();
    r.set_icon_size(28.0);
    let big = r.icon_instance(&ctx, &icon(crate::font::glyphs::CLOSE)).unwrap();
    assert!(big.size[1] >= small.size[1] * 1.8, "{} vs {}", big.size[1], small.size[1]);
    assert!(small.size[1] >= 8.0 && small.size[1] <= 14.0, "encre d'un ✕ Material à 14 px : {}", small.size[1]);
    assert_eq!(r.icon_instance(&ctx, &icon('a')), None, "la face d'icônes ne couvre que les sept glyphes");
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty-render --lib icon_glyphs_scale` → `set_icon_size` inconnu.
- [ ] **Étape 3 : implémenter** renderer, cache, `Renderers`, spec, style.
- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert ; `tests/golden/tab_bar.png` inchangée (les icônes ne sont pas dessinées par ce test).
- [ ] **Étape 5 : commit** — alpha.152 « Face d'icônes Material dans le renderer ».

### Task 4 : six boutons et glyphes Material dans la barre

**Files:**
- Modify: `crates/rustty-render/src/chrome/title_bar.rs` (constantes de glyphes remplacées par `glyphs::*` : réduire `REMOVE`, agrandir `CROP_SQUARE`, restaurer `FILTER_NONE`, fermer `CLOSE`, réglages `SETTINGS`, couleur `PALETTE`, dossier `FOLDER_OPEN` ; `from_config` garde **tous** les boutons `visible` (plus de filtre `is_window_control`) ; icône d'un bouton d'outil : `readable_on(style.background, look.icon, DARK_TITLE)` hors survol et `readable_on(style.background, look.hover_icon, DARK_TITLE)` au survol ; boutons de fenêtre inchangés)
- Modify: `crates/rustty-render/src/chrome/title_bar_tests.rs`, `crates/rustty-render/tests/offscreen.rs` (golden `title_bar.png` régénérée avec `UPDATE_GOLDEN=1`, puis contrôlée agrandie)
- Modify: `crates/rustty/src/title_input.rs` (`button_effects` : `Settings → vec![Effect::OpenConfig]`, `Folder → vec![Effect::OpenCwd]`, `Color → vec![]` (jalon 3)), `crates/rustty/src/title_input_tests.rs`

**Interfaces:**
- Consumes: `glyphs` (tâche 1), `Effect::OpenConfig`/`OpenCwd` (tâche 2), `TitleBarStyle::icon_size` (tâche 3).
- Produces: `TitleBarLayout.buttons` dans l'ordre `[Settings, Color, Folder, Minimize, Maximize, Close]` (les masqués en moins) ; `Chrome.icons` : une icône par bouton, glyphe Material.

- [ ] **Étape 1 : tests qui échouent** (adapter les existants et ajouter) :

```rust
// style_scales_logical_sizes_to_physical : 6 boutons, buttons[0] (settings) size 44, radius 10 ; s.icon_size == 28
// window_controls_sit_at_the_right_in_order : names == [Settings, Color, Folder, Minimize, Maximize, Close] ;
//   settings == PixelRect::new(6, 3, 22, 22) ; color.x == 32 ; folder.x == 58 ; l.title.x == 84
#[test]
fn hidden_tool_buttons_free_the_title_zone() {
    let mut cfg = rustty_config::TitleBar::default();
    cfg.buttons.settings.visible = false; cfg.buttons.color.visible = false; cfg.buttons.folder.visible = false;
    let l = layout_title_bar(400, &TitleBarStyle::from_config(&cfg, 1.0));
    assert_eq!(l.buttons.len(), 3);
    assert_eq!(l.title.x, 6 + 4, "comme au jalon 1");
}
#[test]
fn icons_use_the_material_glyphs() {
    let c = title_bar_chrome(&layout_title_bar(400, &style()), &style(),
        TitleBarState { title: "x", hover: None, maximized: true }, metrics());
    let chars: Vec<char> = c.icons.iter().map(|i| i.ch).collect();
    assert_eq!(chars, [glyphs::SETTINGS, glyphs::PALETTE, glyphs::FOLDER_OPEN, glyphs::REMOVE, glyphs::FILTER_NONE, glyphs::CLOSE]);
}
#[test]
fn light_background_darkens_title_and_tool_icons() {
    let mut cfg = rustty_config::TitleBar::default();
    cfg.background = rustty_config::Rgba8::new(0xf0, 0xf0, 0xf0, 0xff);
    let s = TitleBarStyle::from_config(&cfg, 1.0);
    let c = title_bar_chrome(&layout_title_bar(400, &s), &s,
        TitleBarState { title: "x", hover: Some(TitleButton::Close), maximized: false }, metrics());
    assert_eq!(c.icons[0].color, DARK_TITLE, "réglages sur fond clair");
    assert_eq!(c.texts[0].color, DARK_TITLE, "titre sur fond clair");
    assert_eq!(c.icons[5].color, s.buttons[5].1.hover_icon, "fermer garde son style");
}
// title_input_tests.rs, maximize_button_restores_when_maximized : Settings → [Effect::OpenConfig], Folder → [Effect::OpenCwd], Color vide
```

  (`DARK_TITLE` est déjà une constante du module : les tests dans `title_bar_tests.rs` la voient par `use super::*`.)

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty-render --lib title_bar` → échecs sur le nombre de boutons et les glyphes.
- [ ] **Étape 3 : implémenter** ; `UPDATE_GOLDEN=1 cargo test -p rustty-render --test offscreen title_bar`, puis agrandir ×10 la zone des boutons (Pillow) et vérifier : trois icônes grises à gauche, pastilles à droite, icône centrée dans la pastille survolée.
- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert, `tab_bar.png` inchangée.
- [ ] **Étape 5 : commit** — alpha.153 « Six boutons et glyphes Material dans la barre ».

### Task 5 : réglages ouvre le TOML, dossier ouvre le cwd

**Files:**
- Create: `crates/rustty/src/external.rs` + `crates/rustty/src/external_tests.rs` (déclarer `mod external;` dans `main.rs`)
- Modify: `crates/rustty/src/effects.rs` (bras `OpenConfig` et `OpenCwd`), `crates/rustty/Cargo.toml` (`directories.workspace = true`)

**Interfaces:**
- Consumes: `Effect::OpenConfig`/`OpenCwd` (tâche 2), `self.config_path: Option<PathBuf>` et `config_watch::init_config` (existants), `TermWindow::cwd()`, `Banner::error` + `self.model.set_notice`.
- Produces:

```rust
// external.rs (pur sauf spawn_detached)
pub fn opener() -> Vec<String>;   // macos: ["open"] ; windows: ["cmd", "/C", "start", ""] ; sinon ["xdg-open"]
/// Première valeur non blanche parmi `configured`, `visual`, `editor`, découpée sur les blancs ; sinon `opener()`.
pub fn editor_command(configured: &str, visual: Option<&str>, editor: Option<&str>) -> Vec<String>;
pub fn file_manager_command(configured: &str) -> Vec<String>;   // non blanc → découpé ; sinon opener()
/// Le TOML existe ensuite : créé depuis le modèle s'il manquait (AlreadyExists n'est pas une erreur).
pub fn ensure_config_file(path: &Path) -> std::io::Result<()>;
pub fn folder_to_open(cwd: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf>;   // cwd, sinon home
/// Lance `command[0]` avec `command[1..]` puis `target`, stdio nuls, sans attendre ; un thread moissonne l'enfant.
pub fn spawn_detached(command: &[String], target: &Path) -> std::io::Result<()>;
```

- [ ] **Étape 1 : tests qui échouent** dans `external_tests.rs` :

```rust
#[test]
fn editor_falls_back_from_key_to_env_to_opener() {
    assert_eq!(editor_command("code --wait", Some("vim"), Some("nano")), ["code", "--wait"]);
    assert_eq!(editor_command("", Some("vim"), Some("nano")), ["vim"]);
    assert_eq!(editor_command("", None, Some("nano -w")), ["nano", "-w"]);
    assert_eq!(editor_command("", None, None), opener());
}
#[test]
fn blank_values_are_skipped() {
    assert_eq!(editor_command("  ", Some(" "), Some("")), opener());
    assert_eq!(file_manager_command(" \t"), opener());
    assert_eq!(file_manager_command("nautilus --new-window"), ["nautilus", "--new-window"]);
}
#[test]
fn opener_matches_the_platform() {
    if cfg!(target_os = "linux") { assert_eq!(opener(), ["xdg-open"]); }
}
#[test]
fn config_file_is_created_once_then_left_alone() {
    let dir = tempdir();                       // helper local : std::env::temp_dir() + nom unique, nettoyé en fin
    let path = dir.join("rustty.toml");
    ensure_config_file(&path).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), rustty_config::DEFAULT_TOML);
    std::fs::write(&path, "[titlebar]\nheight = 20\n").unwrap();
    ensure_config_file(&path).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "[titlebar]\nheight = 20\n");
}
#[test]
fn folder_to_open_falls_back_to_home() {
    let home = PathBuf::from("/home/x");
    assert_eq!(folder_to_open(Some(PathBuf::from("/tmp")), Some(home.clone())), Some(PathBuf::from("/tmp")));
    assert_eq!(folder_to_open(None, Some(home.clone())), Some(home));
    assert_eq!(folder_to_open(None, None), None);
}
#[test]
fn spawning_a_missing_program_is_an_error_not_a_panic() {
    assert!(spawn_detached(&["rustty-programme-inexistant".to_string()], Path::new("/")).is_err());
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty external` → compilation impossible.
- [ ] **Étape 3 : implémenter `external.rs`**, puis `effects.rs` :
  - `OpenConfig` : `path = self.config_path.clone().or_else(Config::default_path)` ; `None` → bandeau « dossier de configuration introuvable » ; sinon `ensure_config_file(&path)` puis `spawn_detached(&editor_command(&self.config.titlebar.editor, env("VISUAL"), env("EDITOR")), &path)` ; toute erreur → `self.model.set_notice(Some(Banner::error(format!("ouvrir la configuration : {e}"))))` + `request_redraw`. `env(name)` = `std::env::var(name).ok()`.
  - `OpenCwd` : `cwd = self.model.workspace.focused_term().and_then(|t| self.terms.get(&t)).and_then(TermWindow::cwd)` ; `home = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())` ; `folder_to_open` ; `None` → bandeau ; sinon `spawn_detached(&file_manager_command(&self.config.titlebar.file_manager), &dir)` ; erreur → bandeau « ouvrir le dossier : {e} ».
- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert ; à la main (binaire de test lancé par `setsid`, tué par PID) : clic sur réglages ouvre le TOML dans `code` (ou le bandeau si absent), `editor = ""` sans variables ouvre avec `xdg-open`, clic sur dossier ouvre le cwd dans l'explorateur, `[keys] "ctrl+shift+alt+o" = "open_config"` fait de même, l'interface ne gèle pas. Noter ce qui a été observé dans le commit.
- [ ] **Étape 5 : commit** — alpha.154 « Réglages et dossier ouvrent l'éditeur et l'explorateur ».

### Task 6 : le titre de la fenêtre suit le nom d'onglet personnalisé

**Files:**
- Modify: `crates/rustty/src/title.rs` (`window_title`), ses tests dans le même fichier
- Modify: `crates/rustty/src/title_bar.rs` (`window_title_text`)

**Interfaces:**
- Consumes: `Workspace::active_tab().custom_title: Option<String>`, `self.titles: HashMap<TermId, String>` (titre OSC du panneau actif).
- Produces: `pub fn window_title(custom: Option<&str>, term_title: &str, version: &str) -> String` : `custom` non vide → `"{custom} — rustty {version}"` ; sinon comportement actuel.

- [ ] **Étape 1 : tests qui échouent** dans `title.rs` :

```rust
#[test]
fn window_title_prefers_the_custom_tab_name() {
    assert_eq!(window_title(Some("build"), "zsh", "1.0"), "build — rustty 1.0");
    assert_eq!(window_title(None, "zsh", "1.0"), "zsh — rustty 1.0");
    assert_eq!(window_title(Some(""), "", "1.0"), "rustty 1.0");
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty window_title` → compilation impossible.
- [ ] **Étape 3 : implémenter** ; `window_title_text` passe `self.model.workspace.active_tab().custom_title.as_deref()` (la workspace peut être vide : `tabs().get(active)`, sinon `None`).
- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert.
- [ ] **Étape 5 : commit** — alpha.155 « Le titre de la fenêtre suit le nom d'onglet personnalisé ».

### Task 7 : documentation, E2E et publication

**Files:**
- Modify: `README.md` (section `[titlebar]` : icônes Material, `icon_size`, `editor`, `file_manager`, boutons réglages et dossier avec leur action, retirer « ne sont pas encore dessinés » ; `settings`/`color`/`folder` visibles, `color` sans effet jusqu'au jalon 3 ; actions `open_config`, `open_cwd` sans raccourci ; le titre de la fenêtre reprend le nom d'onglet personnalisé)
- Modify: `docs/e2e.md` (section « Barre de titre (plan 11) » : remplacer la case `settings`, `color`, `folder` par : icônes grises visibles à gauche, survol les éclaircit ; réglages ouvre le TOML dans l'éditeur, le crée s'il manque ; `editor = ""` → `$VISUAL`/`$EDITOR`/ouvreur ; éditeur introuvable → bandeau, interface vivante ; dossier ouvre le cwd du panneau actif, dossier personnel sans cwd ; `icon_size = 24` à chaud ; HiDPI : icônes doublées ; `visible = false` sur `settings` élargit le titre ; fond `#f0f0f0` : icônes d'outils et titre sombres, pastilles inchangées)
- Modify: `docs/superpowers/specs/2026-11-01-titlebar-design.md` (ligne de la police : chemin `crates/rustty-render/assets/`, licence `LICENSE-MaterialSymbols`)
- Modify: `CHANGELOG.md` (entrée qui résume le jalon)

- [ ] **Étape 1 : rédiger** README, e2e, spec, CHANGELOG ; relire le README en entier.
- [ ] **Étape 2 : vérifier** — `cargo test --workspace` vert.
- [ ] **Étape 3 : commit** — alpha.156 « Barre de titre : documentation du jalon 2 », push sur `main`, attendre la CI verte (filtrer `gh run list` par SHA), tag annoté `v0.1.0-alpha.156`, release GitHub en pré-version avec les notes de la section CHANGELOG, `cargo install --path crates/rustty --locked`.
- [ ] **Étape 4 : vitrines** — reportées au jalon 4 avec des captures ; le dire dans le compte rendu.
