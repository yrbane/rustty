# Personnalisation — plan d'implémentation (plan 5)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** répondre aux demandes de seb du 2026-10-09 après la première prise en main du binaire : barre de split configurable (couleur fixe ou aléatoire par split), marges et espacement des onglets, renommage des onglets, couleurs des onglets (fixes ou aléatoires), zoom de police (ctrl+molette, ctrl++, ctrl+-, ctrl+0), icône `res/rustty.svg` partout (fenêtre, barre des tâches, exécutable Windows, menus Linux), plus d'avertissement de dépendance au lancement.

**Architecture :** chaque option vit dans `rustty-config` (TOML, défauts, validation, exemple publié), la géométrie dans `rustty-layout` (identifiants de division et rectangles des barres), le dessin dans `rustty-render` (style de barre d'onglets paramétré, contraste automatique), et la logique d'interface dans des modules purs du binaire (tirage des couleurs, renommage, double clic, zoom). Le câblage `winit` reste du dispatch.

**Tech Stack :** inchangé ; nouveaux outils de génération d'icônes hors build (`rsvg-convert`, ImageMagick, Pillow) et `winresource` 0.1 en dépendance de build Windows.

**Spec :** `docs/superpowers/specs/2026-10-08-rustty-design.md` (§3.3 config, §3.5 rendu, §3.6 binaire, §7 icône et version) complétée par les demandes de seb du 2026-10-09 (citées dans le CHANGELOG du commit du plan).

## Décisions prises faute de réponse (seb a renvoyé la demande sans trancher)

- **Couleurs aléatoires** : tirées parmi les 12 couleurs « vives » de la palette ANSI (indices 1–6 et 9–14), jamais deux fois de suite la même. Elles restent donc en accord avec le thème.
- **Renommage** : double clic sur l'onglet, et action `rename_tab` liée par défaut à `ctrl+shift+alt+t` (le raccourci de kitty). Le nom choisi remplace `{title}` dans le gabarit ; un nom vide rend le titre du shell.
- **Affichage d'images (`img <url/chemin>`)** : hors de ce plan. C'est le protocole graphique de kitty (spec, jalon v0.3) : il aura son propre plan 6.

## Global Constraints

- Code et identifiants en anglais ; commentaires, commits, CHANGELOG, docs en français ; aucune mention d'assistant dans les commits.
- Une responsabilité par fichier (≤ ~300 lignes), logique pure séparée du dispatch, TDD sur tout ce qui est pur.
- Tâche N → `0.1.0-alpha.(72+N)` (le commit du plan porte `0.1.0-alpha.72`), entrée en tête de `CHANGELOG.md` datée du jour du commit, `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` verts, `git add` ciblé.
- Les défauts reproduisent exactement le rendu actuel : les trois images de référence de `rustty-render` ne changent pas.
- Chaque nouvelle clé de configuration est documentée dans `DEFAULT_TOML` et donc dans `docs/rustty.example.toml` (le test miroir l'impose).

## Review Focus

1. **Valeurs de configuration extrêmes** (largeur de barre 0 ou 1000, marges énormes, couleur mal écrite) : erreur positionnée et ancienne config conservée, jamais de rectangle négatif ni de panique. Tests tâche 1 (`out_of_range_values_are_rejected`) et tâche 3 (`huge_paddings_never_overflow`).
2. **Renommage puis fermeture de l'onglet en cours d'édition, ou édition avec des caractères de contrôle collés** : l'édition s'annule proprement, aucun caractère de contrôle n'entre dans le titre. Tests tâche 5 (`closing_the_renamed_tab_cancels_the_edit`, `control_characters_are_ignored`).
3. **Zoom de police répété au-delà des bornes** : la taille reste entre 4 et 72 points, ctrl+0 revient à la taille de la config, un rechargement de config qui change la taille repart de la nouvelle base. Tests tâche 6 (`font_size_is_clamped`, `reset_returns_to_the_base`).
4. **Couleur de texte illisible sur une couleur d'onglet aléatoire ou choisie** : le texte prend la couleur claire ou sombre de la palette selon la luminance du fond. Test tâche 3 (`text_contrasts_with_the_tab_color`).
5. **Installation du raccourci de bureau dans un répertoire inexistant ou sans droits** : erreur claire, rien d'écrit à moitié. Test tâche 7 (`install_creates_every_file_under_data_home`).

---

### Task 1 : Configuration — splits, onglets, renommage, zoom, noms de touches

**Files :** `crates/rustty-config/src/{sections.rs, config.rs, action.rs, keymap.rs, keys.rs, example.rs, lib.rs}`, `docs/rustty.example.toml`, `crates/rustty/src/model.rs` (bras provisoires pour les nouvelles actions).

**Interfaces :**
- `Splits { border: bool = true, width: u32 = 2, color: Option<Rgb> = None, random_colors: bool = false }` dans `Config.splits` (section `[splits]`).
- `Tabs` gagne `padding_horizontal: u32 = 1` (cellules de chaque côté du titre), `padding_vertical: u32 = 2` (pixels au-dessus et au-dessous), `spacing: u32 = 0` (pixels entre deux onglets), `colors: TabColors`.
- `TabColors { bar_background, active_background, active_foreground, inactive_background, inactive_foreground: Option<Rgb>, random: bool }` (section `[tabs.colors]`, tout `None` / `false` par défaut = couleurs dérivées de la palette comme aujourd'hui).
- Validation : `splits.width` dans 1..=32, `tabs.padding_horizontal` ≤ 8, `tabs.padding_vertical` ≤ 32, `tabs.spacing` ≤ 64 ; message `ConfigError::Invalid { field, reason }`.
- `Action` gagne `RenameTab`, `IncreaseFontSize`, `DecreaseFontSize`, `ResetFontSize` (chaînes `"rename_tab"`, `"increase_font_size"`, `"decrease_font_size"`, `"reset_font_size"`).
- Noms de touches : `plus` → `+`, `minus` → `-`, `equal` → `=` ; l'affichage d'une combinaison écrit ces noms (aller-retour garanti).
- Défauts ajoutés : `ctrl+shift+alt+t` → `rename_tab` ; `ctrl+plus`, `ctrl+shift+plus`, `ctrl+equal` → `increase_font_size` ; `ctrl+minus` → `decrease_font_size` ; `ctrl+0` → `reset_font_size`.

- [ ] **Step 1 : tests.** Dans `config.rs` : `splits_and_tab_sections_parse` (TOML avec `[splits] border = false, width = 4, color = "#ff0000", random_colors = true` et `[tabs] padding_horizontal = 2, padding_vertical = 6, spacing = 3` + `[tabs.colors] active_background = "#112233", random = true` → valeurs lues) ; `out_of_range_values_are_rejected` (`width = 0`, `width = 1000`, `padding_horizontal = 9`, `padding_vertical = 33`, `spacing = 65` → `Invalid` avec le nom du champ) ; `unknown_key_in_new_sections_is_an_error` (`[splits] colour = "#fff"`). Dans `action.rs` : les quatre nouvelles chaînes. Dans `keys.rs` : `ctrl+plus`, `ctrl+minus`, `ctrl+equal` parsent vers `Char('+')`, `Char('-')`, `Char('=')` et s'affichent `ctrl+plus` etc. Dans `keymap.rs` : les nouveaux défauts se résolvent. Dans `example.rs` : `[splits]`, `random_colors`, `padding_horizontal`, `padding_vertical`, `spacing`, `[tabs.colors]`, `rename_tab`, `increase_font_size` documentés.
- [ ] **Step 2 : échec constaté** (`cargo test -p rustty-config`).
- [ ] **Step 3 : implémenter.** Les structs avec `#[serde(default, deny_unknown_fields)]` et `Default` explicite ; `Config` gagne `splits: Splits` ; `validate` avec les bornes ci-dessus ; `Simple` gagne les quatre variantes ; `parse_key` consulte `CHAR_NAMES = [("plus", '+'), ("minus", '-'), ("equal", '=')]` avant le reste et `Display` les réécrit ; `DEFAULTS` ; `DEFAULT_TOML` documente chaque clé (couleurs optionnelles en commentaire) et `docs/rustty.example.toml` en est la copie exacte. Dans le binaire, `Model::apply` reçoit un bras provisoire `Action::RenameTab | Action::IncreaseFontSize | Action::DecreaseFontSize | Action::ResetFontSize => Vec::new()` (remplacé aux tâches 5 et 6).
- [ ] **Step 4 : suite verte**, puis commit `rustty-config : options de splits, d'onglets, renommage et zoom (0.1.0-alpha.73)`.

### Task 2 : `rustty-layout` — identifiant des divisions et rectangles des barres

**Files :** `crates/rustty-layout/src/{geometry.rs, node.rs, tab_layout.rs, lib.rs}`.

**Interfaces :** `pub struct SplitId(pub u64)` (Copy, Eq, Hash, Ord, Debug). `Node::Split` gagne `id: SplitId`. **Contrat** : une division créée par `TabLayout::split(target, axis) -> Some(new)` a pour identifiant `SplitId(new.0)` ; elle le garde à travers `resize`, `rotate` et la fermeture d'autres panneaux. `TabLayout::dividers(&self, bounds: Rect, gap: u32) -> Vec<(SplitId, Rect)>` : l'interstice de chaque division, de la racine vers les feuilles ; vide si zoomé ou si `gap == 0`.

- [ ] **Step 1 : tests** (`tab_layout.rs`) : `a_vertical_split_has_one_vertical_divider` (`split(first, Vertical)` dans `Rect(0, 24, 802, 500)`, gap 2 → `[(SplitId(2), Rect(400, 24, 2, 500))]`) ; `nested_splits_list_every_divider` (deux divisions → deux barres, aucune ne chevauche un panneau de `rects`) ; `divider_ids_survive_closing_other_panes` (trois panneaux, fermer le deuxième créé garde l'id de la division restante) ; `no_dividers_when_zoomed_or_without_gap`.
- [ ] **Step 2 : échec constaté.**
- [ ] **Step 3 : implémenter** (`Node::dividers` récursif à partir de `split_rect` : pour `Vertical`, `Rect(a.x + a.width, bounds.y, b.x − (a.x + a.width), bounds.height)`, symétrique pour `Horizontal` ; rectangles de taille nulle omis ; l'id est porté par chaque reconstruction dans `remove`).
- [ ] **Step 4 : suite verte**, commit `rustty-layout : identifiants de division et rectangles des barres de split (0.1.0-alpha.74)`.

### Task 3 : `rustty-render` — style de barre d'onglets paramétré et contraste

**Files :** `crates/rustty-render/src/{chrome.rs, color.rs, lib.rs}`, `crates/rustty-render/tests/offscreen.rs`, appels dans `crates/rustty/src/{window_state.rs, banner.rs}`.

**Interfaces :**
- `Rgba::luminance(self) -> f32` (0.2126 r + 0.7152 g + 0.0722 b) ; `pub fn readable_on(background: Rgba, light: Rgba, dark: Rgba) -> Rgba` (sombre si luminance > 0.5).
- `TabBarStyle::from_config(palette: &Palette, tabs: &rustty_config::Tabs) -> TabBarStyle` remplace l'ancienne signature ; nouveaux champs `padding_horizontal`, `padding_vertical`, `spacing`, `light_text` (= `palette.foreground`), `dark_text` (= `palette.background`) ; chaque couleur de `tabs.colors` remplace la couleur dérivée de la palette quand elle est donnée.
- `TabSpec { title, active, accent: Option<Rgba> }` : avec un accent, le fond de l'onglet est l'accent (actif) ou l'accent assombri à 0,6 (inactif), et le texte `readable_on(fond, light_text, dark_text)`.
- `tab_bar_height(metrics, style: &TabBarStyle) = metrics.height + 2 × padding_vertical`. Largeur d'un onglet : `(2 × padding_horizontal + titre + bouton) × cw`, puis `spacing` pixels avant le suivant ; le titre commence à `padding_horizontal × cw`, le bouton à `(padding_horizontal + titre) × cw`, le texte à `y + padding_vertical`.

- [ ] **Step 1 : tests** (`chrome.rs`) : `default_style_keeps_the_previous_geometry` (les valeurs actuelles : 110 px, bouton à 60, « + » à 230) ; `horizontal_padding_widens_tabs` (padding 3 → 150 px, bouton à 100, titre à x = 30) ; `spacing_separates_tabs` (spacing 4 → second onglet à 114) ; `vertical_padding_sets_the_bar_height` (6 → 32 px, texte à y + 6) ; `configured_colors_override_the_palette` ; `accent_colors_the_tab_and_dims_inactive_ones` ; `text_contrasts_with_the_tab_color` (accent clair `#f9e2af` → `dark_text`, accent sombre `#313244` → `light_text`) ; `huge_paddings_never_overflow` (padding 8 dans 100 px → aucun onglet, aucune panique). `color.rs` : `luminance_of_black_white_and_green`. L'image de référence `tab_bar.png` reste identique (le test hors écran passe sans `UPDATE_GOLDEN`).
- [ ] **Step 2 : échec constaté.**
- [ ] **Step 3 : implémenter** ; `banner.rs` garde sa propre marge `BANNER_PADDING = 2` (le bandeau ne suit pas le style des onglets) et `window_state.rs` passe `&config.tabs`.
- [ ] **Step 4 : suite verte**, commit `rustty-render : marges, espacement, couleurs et contraste de la barre d'onglets (0.1.0-alpha.75)`.

### Task 4 : Couleurs d'accent tirées au sort, métadonnées d'onglet

**Files :** create `crates/rustty/src/accent.rs` ; modify `tab.rs`, `workspace.rs`, `model.rs`, `main.rs`.

**Interfaces :**
- `pub const ACCENT_SLOTS: [usize; 12] = [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14]` ; `pub struct AccentPicker` (xorshift64, `new(seed: u64)`, `from_clock()`, `next(&mut self) -> usize` dans `0..12`, jamais égal au précédent) ; `pub fn accent_color(palette: &Palette, accent: usize) -> Rgba`.
- `Tab` gagne `pub accent: usize`, `pub custom_title: Option<String>`, `split_accents: HashMap<SplitId, usize>` ; `Tab::new(first, accent)` ; `split(axis, new_term, accent) -> bool` mémorise `SplitId(new_window.0)` → `accent` ; `split_accent(SplitId) -> usize` (0 si inconnu).
- `Workspace::with_seed(seed) -> (Workspace, TermId)` ; `Workspace::new()` = `with_seed` sur l'horloge ; chaque `new_tab` et `split_focused` tire un accent ; `rename(index, Option<String>)`.
- `Model::new(opacity, hold)` inchangé (horloge) ; `Model::with_seed(opacity, hold, seed)` pour les tests.

- [ ] **Step 1 : tests** (`accent.rs`) : `never_twice_in_a_row`, `same_seed_same_sequence`, `uses_many_slots` (100 tirages → au moins 10 accents distincts), `zero_seed_is_valid`, `accent_color_maps_slots_to_vivid_palette_entries`. (`workspace.rs`) : `tabs_and_splits_get_accents` (deux onglets consécutifs ont des accents différents ; un split enregistre son accent sous `SplitId` de la nouvelle fenêtre) ; `rename_sets_and_clears_the_custom_title`.
- [ ] **Step 2 : échec constaté.** — **Step 3 : implémenter.** — **Step 4 : suite verte**, commit `rustty : couleurs d'accent et titres personnalisés des onglets (0.1.0-alpha.76)`.

### Task 5 : Renommage des onglets — édition, double clic, titres

**Files :** create `crates/rustty/src/rename.rs`, `crates/rustty/src/mouse/click.rs` ; modify `model.rs`, `title.rs`, `input.rs`, `window_state.rs`, `mouse/mod.rs`.

**Interfaces :**
- `rename::RenameKey::{Text(String), Backspace, Commit, Cancel}` ; `rename::Rename { pub tab: usize, pub buffer: String }` avec `apply(&mut self, key) -> RenameOutcome::{Editing, Commit(Option<String>), Cancel}` (texte : caractères de contrôle ignorés ; titre limité à 64 caractères ; `Commit` rend `None` pour un nom vide après `trim`).
- `Model` gagne `renaming: Option<Rename>`, `start_rename(tab) -> Vec<Effect>` (pré-remplit avec le titre personnalisé), `rename_key(key) -> Vec<Effect>` (`Commit` → `workspace.rename` + `Relayout`, sinon `Redraw`) ; `Action::RenameTab` → `start_rename(actif)` ; toute fermeture d'onglet annule l'édition.
- `title::display_title(template, index, shell_title, custom: Option<&str>, editing: Option<&str>) -> String` : en édition, `"{buffer}▌"` sans gabarit ; sinon le gabarit avec `{title}` = nom personnalisé ou titre du shell.
- `mouse::click::DoubleClick` : `register(&mut self, target: usize, now: Instant) -> bool` (vrai si même cible dans les 400 ms précédentes ; un double clic consomme le premier).
- `input.rs` : en édition, Entrée → `Commit`, Échap → `Cancel`, Retour arrière → `Backspace`, sinon le texte de la touche → `Text` ; le relâchement gauche sur `Tab(i)` enregistre le clic et, s'il est double, lance `start_rename(i)` au lieu d'activer.

- [ ] **Step 1 : tests** : `rename.rs` (`typing_and_backspace`, `control_characters_are_ignored`, `commit_trims_and_empty_means_none`, `cancel_keeps_nothing`, `length_is_capped`) ; `model.rs` (`rename_action_edits_the_active_tab`, `commit_renames_and_relayouts`, `closing_the_renamed_tab_cancels_the_edit`) ; `title.rs` (`custom_title_replaces_the_shell_title`, `editing_shows_the_buffer_with_a_cursor`) ; `click.rs` (`two_quick_clicks_on_the_same_tab`, `slow_or_different_clicks_are_single`, `a_triple_click_is_one_double_then_single`).
- [ ] **Step 2 : échec constaté.** — **Step 3 : implémenter.** — **Step 4 : suite verte**, commit `rustty : renommage des onglets au double clic et au clavier (0.1.0-alpha.77)`.

### Task 6 : Barres de split, couleurs d'onglets et zoom de police à l'écran

**Files :** create `crates/rustty/src/font_zoom.rs` ; modify `geometry.rs`, `render_frame.rs`, `model.rs`, `window_state.rs`, `input.rs`, `effects.rs`, `main.rs`.

**Interfaces :**
- `geometry::split_gap(splits: &Splits) -> u32` (`width` si `border`, sinon 0) ; `pane_rects(layout, content, gap)` ; `divider_rects(layout, content, gap) -> Vec<(SplitId, PixelRect)>` ; `PANE_GAP` disparaît.
- `render_frame::divider_quads(dividers: &[(SplitId, PixelRect)], color_of: impl Fn(SplitId) -> Rgba) -> Vec<ChromeQuad>` ; `tab_specs(titles, active, accents: Option<&[Rgba]>)`.
- Couleur d'une barre : `random_colors` → `accent_color(palette, tab.split_accent(id))`, sinon `splits.color` ou `palette.ansi[8]`. Couleur d'un onglet : `tabs.colors.random` → `accent_color(palette, tab.accent)`, sinon `None` (style).
- `font_zoom::FontChange::{Increase, Decrease, Reset}` ; `font_zoom::next_size(current: f32, base: f32, change) -> f32` (pas de 1 point, bornes 4–72, `Reset` → `base`).
- `Effect::FontSize(FontChange)` ; `Model::apply` : les trois actions de zoom → cet effet ; ctrl+molette → un effet par ligne accumulée (haut = agrandir) au lieu du défilement.
- `OsWindow` garde `font_size: f32` (initialisé à `config.font.size`, remis à la nouvelle base quand un rechargement change `font.size`) ; `rebuild_fonts` l'utilise ; un zoom reconstruit les polices puis `Relayout`.

- [ ] **Step 1 : tests** : `font_zoom.rs` (`steps_of_one_point`, `font_size_is_clamped`, `reset_returns_to_the_base`) ; `geometry.rs` (`gap_follows_the_border_option`, `divider_rects_convert_the_layout`) ; `render_frame.rs` (`divider_quads_take_their_color_per_split`, `tab_specs_carry_accents`) ; `model.rs` (`font_actions_emit_font_size_effects`).
- [ ] **Step 2 : échec constaté.** — **Step 3 : implémenter** (câblage dans `window_state.rs` : barres dessinées en chrome, accents d'onglet ; `input.rs` : ctrl+molette ; `effects.rs` : `FontSize`). — **Step 4 : suite verte + essai à l'écran** (split, ctrl+molette), commit `rustty : barres de split, couleurs d'onglets et zoom de police (0.1.0-alpha.78)`.

### Task 7 : Icône partout, raccourci de bureau, journal sans bruit

**Files :** create `scripts/icons.sh`, `assets/icons/rustty-{16,32,48,64,128,256,512}.png`, `assets/icons/rustty.ico`, `assets/icons/rustty.icns`, `crates/rustty/build.rs`, `crates/rustty/src/desktop.rs` ; modify `crates/rustty/Cargo.toml`, `cli.rs`, `main.rs`, `window_state.rs`, `README.md`, `docs/e2e.md`.

**Interfaces :**
- `scripts/icons.sh` régénère toutes les icônes depuis `assets/icon.svg` (qui est `res/rustty.svg` mis au carré) avec `rsvg-convert`, `magick` (ICO 16–256) et Pillow (ICNS).
- Fenêtre : icône `assets/icons/rustty-256.png` ; sous Linux, `app_id` Wayland et `WM_CLASS` X11 = `rustty` (`WindowAttributesExtWayland::with_name` et `WindowAttributesExtX11::with_name`), pour que GNOME/KDE associent la fenêtre au fichier `.desktop`.
- `desktop::desktop_entry(exec: &Path) -> String` ; `desktop::install(data_home: &Path, exec: &Path) -> io::Result<Vec<PathBuf>>` écrit `applications/rustty.desktop`, `icons/hicolor/<n>x<n>/apps/rustty.png` pour chaque taille et `icons/hicolor/scalable/apps/rustty.svg`. CLI `--install-desktop` (Linux) ; ailleurs, message expliquant que l'icône est déjà embarquée.
- `build.rs` : sous cible Windows, `winresource` embarque `assets/icons/rustty.ico` dans l'exécutable (`[target.'cfg(windows)'.build-dependencies] winresource = "0.1"`).
- Journal : filtre par défaut `error,rustty=warn` (les dépendances ne parlent qu'en cas d'erreur ; `RUSTTY_LOG` le remplace) — supprime l'avertissement d'`arboard` sous GNOME, inoffensif puisqu'il se replie sur le presse-papiers X11.

- [ ] **Step 1 : tests** : `desktop.rs` (`entry_names_the_binary_and_the_icon`, `install_creates_every_file_under_data_home`, `install_into_an_unwritable_place_is_an_error`) ; `cli.rs` (`install_desktop_flag`) ; `main.rs` (`default_log_filter_is_valid_and_quiet_for_dependencies`).
- [ ] **Step 2 : échec constaté.** — **Step 3 : implémenter**, générer les icônes, vérifier `cargo run -p rustty -- --install-desktop` puis la fenêtre dans la barre des tâches GNOME. — **Step 4 : suite verte**, README (icône, `--install-desktop`, nouvelles options) et `docs/e2e.md` (parcours des nouveautés), commit `rustty : icône partout, raccourci de bureau et journal discret (0.1.0-alpha.79)`, push, PR.

## Suite

Plan 6 : affichage d'images dans le terminal — protocole graphique de kitty dans `rustty-vt` (APC `_G`, transmission directe et par fichier, placement), stockage et textures dans `rustty-render`, et une commande `rustty img <chemin|url>` (équivalent de `kitty +kitten icat`).
