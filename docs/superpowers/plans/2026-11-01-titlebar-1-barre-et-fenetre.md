# Barre de titre maison, jalon 1 : barre et fenêtre — plan d'implémentation (plan 11)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Retirer les décorations système et dessiner la barre de titre de rustty : fond noir à reflet, titre centré, boutons réduire / agrandir / fermer fonctionnels, déplacement par la barre et redimensionnement par les bords, le tout paramétré par `[titlebar]`.

**Architecture:** Un type couleur avec alpha dans `rustty-config` et une section `[titlebar]` validée. Dans `rustty-render`, le pipeline `quad` apprend les coins arrondis, la bordure et le dégradé vertical (même sortie qu'aujourd'hui aux valeurs neutres), et un module pur `chrome/title_bar.rs` produit disposition, zones cliquables et chrome, testé par image de référence. Dans le binaire, `geometry::window_geometry` réserve la hauteur, deux modules purs (`window_edges.rs`, `title_input.rs`) décident du bord saisi et du seuil de déplacement, et un câblage mince dans `input.rs`, `relayout.rs`, `render.rs`, `window_state.rs` appelle winit (`set_decorations`, `drag_window`, `drag_resize_window`, `set_maximized`, `set_minimized`).

**Tech Stack:** Rust 1.96 (CI : clippy 1.99), winit 0.30, wgpu 30, serde + toml 0.9.

**Spec:** `docs/superpowers/specs/2026-11-01-titlebar-design.md` (sections « Disposition », « Aspect », « Actions » pour les trois boutons de fenêtre, « Déplacer et redimensionner », « Priorité des appuis », « Rendu », « Configuration » pour les clés de ce jalon). Les icônes Material, les boutons réglages / couleur / dossier, le renommage et le nuancier sont les jalons 2 et 3.

## Global Constraints

- TDD : chaque test est écrit et vu échouer avant le code.
- Code en anglais ; commentaires, messages, libellés et commits en français. Aucune mention d'assistant dans les commits.
- Chaque commit = une version : alpha.138 pour la tâche 1, puis +1 par tâche ; `Cargo.toml` racine (`version = "0.1.0-alpha.N"`), `cargo update -w` pour `Cargo.lock`, entrée `CHANGELOG.md` `## 0.1.0-alpha.N — 2026-11-01 · « titre »` dans le même commit.
- Avant chaque commit, depuis la racine : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets` (zéro avertissement), `cargo test --workspace`. `git add` ciblé, jamais `-A`.
- Fichiers d'environ 300 lignes au plus. Déjà à la limite : `input.rs`, `window_state.rs`, `model.rs`, `chrome/mod.rs` → n'y ajouter que des appels, des champs ou des déclarations ; la logique va dans les nouveaux modules nommés ci-dessous. Tests séparés en `*_tests.rs` (motif `#[cfg(test)] #[path = "x_tests.rs"] mod tests;`).
- Toutes les tailles de `[titlebar]` sont en **pixels logiques** ; la conversion en pixels physiques (`(v as f64 * scale_factor).round() as u32`) se fait une seule fois, dans `TitleBarStyle::from_config`.
- Valeurs par défaut (spec « Aspect », « Configuration ») : `enabled = true`, `height = 28`, `background = "#141414"`, `gloss = 0.10`, `separator_color = "#000000"`, `title_color = "#d8d8d8"`, `padding = 6`, `spacing = 4`, `resize_border = 6`. Bouton commun : `size = 22`, `background = "#00000000"`, `border_color = "#00000000"`, `border_width = 0`, `radius = 5`, `icon_color = "#bdbdbd"`, `hover_background = "#ffffff1a"`, `hover_border_color = "#00000000"`, `hover_icon_color = "#ffffff"`. `minimize` / `maximize` / `close` : `size = 14`, `radius = 7`, `border_width = 1`, `border_color = "#00000099"`, `background` et `hover_background` `"#f6bd3b"` / `"#3dc84f"` / `"#ff5f57"`, `icon_color = "#00000000"`, `hover_icon_color = "#111111"`.
- Bornes (spec « Configuration ») : `height` 16..=64, `gloss` 0.0..=1.0, `size` 8..=height, `radius` 0..=32, `border_width` 0..=4, `resize_border` 0..=16, `padding` 0..=32, `spacing` 0..=32.
- Seuil de déplacement : 4 px. Double clic : `mouse::click::DOUBLE_CLICK` (400 ms). La barre de titre est toujours en haut, au-dessus de la barre d'onglets.
- Les images de référence existantes (`tests/golden/tab_bar.png` et les autres) ne changent pas : `UPDATE_GOLDEN=1` n'est utilisé que pour les nouvelles images.
- Linux vérifié à l'écran (GNOME Wayland) ; macOS et Windows compilent (`drag_resize_window` renvoie `NotSupported` sur macOS : l'erreur est ignorée).

## Review Focus

1. Écran HiDPI (`scale_factor = 2`) : la barre fait 56 px physiques et les boutons sont sous la souris → test `style_scales_logical_sizes_to_physical` (tâche 4).
2. Fenêtre plus étroite que les boutons : aucun débordement d'entier, les boutons de droite restent à droite et le titre disparaît → test `narrow_window_keeps_buttons_and_drops_the_title` (tâche 4).
3. `gloss = 0` : la barre est une couleur plate, sans liseré → test `zero_gloss_gives_a_flat_bar` (tâche 4).
4. Fenêtre agrandie : les bords ne redimensionnent plus et le bouton agrandir restaure → tests `no_edge_when_maximized` (tâche 7) et `maximize_button_restores_when_maximized` (tâche 8).
5. Bouton droit ou milieu sur la barre : ne déplace jamais la fenêtre et n'arme rien → test `only_the_left_button_arms_a_move` (tâche 7).
6. `enabled = false` à chaud : plus de barre, décorations système de retour, les panneaux remontent → tests `disabled_title_bar_reserves_nothing` (tâche 5) et `apply_config_toggles_decorations` (tâche 6).

---

### Task 1 : couleur avec alpha dans la configuration

**Files:**
- Modify: `crates/rustty-config/src/color.rs`
- Modify: `crates/rustty-config/src/lib.rs` (export `Rgba8`)
- Modify: `crates/rustty-render/src/color.rs`
- Commit aussi : `docs/superpowers/specs/2026-11-01-titlebar-design.md`, ce plan, `.gitignore` (`.superpowers/`).

**Interfaces:**
- Produces: `rustty_config::Rgba8 { r: u8, g: u8, b: u8, a: u8 }` avec `const fn new(r, g, b, a)`, `const fn opaque(rgb: Rgb) -> Self`, `FromStr` (`#rrggbb` → a = 255, `#rrggbbaa`), `Display` (`#rrggbb` si a = 255, sinon `#rrggbbaa`), `Deserialize` via `FromStr` ; `ColorParseError` inchangé mais son message devient « attendu #rrggbb ou #rrggbbaa » pour `Rgba8` (le message de `Rgb` ne change pas).
- Produces: `rustty_render::Rgba::from_rgba8(c: rustty_config::Rgba8) -> Rgba` (canaux / 255) et `Rgba::lighten(self, amount: f32) -> Rgba` (chaque canal `c + (1 - c) * amount`, alpha inchangé).

- [ ] **Étape 1 : tests qui échouent dans `color.rs` (module `tests` existant)**

```rust
#[test]
fn rgba8_parses_six_and_eight_digits() {
    assert_eq!("#141414".parse::<Rgba8>().unwrap(), Rgba8::new(0x14, 0x14, 0x14, 0xff));
    assert_eq!("#ffffff1a".parse::<Rgba8>().unwrap(), Rgba8::new(0xff, 0xff, 0xff, 0x1a));
    assert_eq!("#00000000".parse::<Rgba8>().unwrap(), Rgba8::new(0, 0, 0, 0));
    assert_eq!(Rgba8::opaque(Rgb::new(1, 2, 3)), Rgba8::new(1, 2, 3, 255));
}

#[test]
fn rgba8_rejects_other_shapes() {
    for bad in ["#fff", "#1234567", "#123456789", "ffffff1a", "#gg00000a"] {
        assert!(bad.parse::<Rgba8>().is_err(), "{bad}");
    }
}

#[test]
fn rgba8_displays_the_shortest_exact_form() {
    assert_eq!(Rgba8::new(0x14, 0x14, 0x14, 0xff).to_string(), "#141414");
    assert_eq!(Rgba8::new(0xff, 0xff, 0xff, 0x1a).to_string(), "#ffffff1a");
}
```

Et dans `crates/rustty-render/src/color.rs` (module `tests` existant ou à créer) :

```rust
#[test]
fn from_rgba8_and_lighten() {
    let c = Rgba::from_rgba8(rustty_config::Rgba8::new(255, 0, 0, 128));
    assert!((c.r - 1.0).abs() < 1e-6 && c.g == 0.0 && (c.a - 128.0 / 255.0).abs() < 1e-6);
    let l = Rgba::new(0.0, 0.5, 1.0, 0.3).lighten(0.5);
    assert_eq!(l, Rgba::new(0.5, 0.75, 1.0, 0.3));
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty-config rgba8 && cargo test -p rustty-render from_rgba8` → erreurs de compilation (`Rgba8` inconnu, `from_rgba8` inconnu).

- [ ] **Étape 3 : implémenter** `Rgba8` dans `rustty-config/src/color.rs` (même style que `Rgb`, `Hash` et `Default` compris), l'exporter dans `lib.rs` à côté de `Rgb`, puis `from_rgba8` et `lighten` dans `rustty-render/src/color.rs`.

- [ ] **Étape 4 : vérifier** — les quatre tests passent ; `cargo test --workspace` vert.

- [ ] **Étape 5 : commit** — alpha.138 « Couleurs avec alpha et design de la barre de titre » : `git add` des deux `color.rs`, `lib.rs`, `.gitignore`, spec, plan, `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`.

### Task 2 : section `[titlebar]`

**Files:**
- Create: `crates/rustty-config/src/titlebar.rs` (structs, défauts, `resolve`)
- Create: `crates/rustty-config/src/titlebar_tests.rs`
- Modify: `crates/rustty-config/src/config.rs` (champ `titlebar`, validation)
- Modify: `crates/rustty-config/src/lib.rs` (`pub mod titlebar; pub use titlebar::{ButtonStyle, TitleBar, TitleButtons};`)
- Modify: `crates/rustty-config/src/example.rs` (section documentée après `[window]`, liste du test `every_section_and_key_is_documented` : `"[titlebar]"`, `"enabled"`, `"gloss"`, `"separator_color"`, `"title_color"`, `"resize_border"`, `"[titlebar.button]"`, `"hover_icon_color"`, `"[titlebar.buttons.close]"`)
- Modify: `docs/rustty.example.toml` (même texte que `DEFAULT_TOML`, le test `example_is_published` le vérifie)

**Interfaces:**
- Produces, dans `rustty_config` :

```rust
#[derive(Clone, Debug, PartialEq, Deserialize)] #[serde(default, deny_unknown_fields)]
pub struct TitleBar { pub enabled: bool, pub height: u32, pub background: Rgba8, pub gloss: f32,
    pub separator_color: Rgba8, pub title_color: Rgba8, pub padding: u32, pub spacing: u32,
    pub resize_border: u32, pub button: ButtonStyle, pub buttons: TitleButtons }
#[derive(Clone, Debug, PartialEq, Deserialize)] #[serde(default, deny_unknown_fields)]
pub struct ButtonStyle { pub size: u32, pub background: Rgba8, pub border_color: Rgba8, pub border_width: u32,
    pub radius: u32, pub icon_color: Rgba8, pub hover_background: Rgba8, pub hover_border_color: Rgba8, pub hover_icon_color: Rgba8 }
/// Surcharge par bouton : une clé absente prend la valeur du style commun.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)] #[serde(default, deny_unknown_fields)]
pub struct ButtonOverride { pub visible: bool /* défaut true */, pub size: Option<u32>, pub background: Option<Rgba8>, … une Option par clé de ButtonStyle }
impl ButtonOverride { pub fn resolve(&self, base: &ButtonStyle) -> ButtonStyle }
#[derive(Clone, Debug, PartialEq, Deserialize)] #[serde(default, deny_unknown_fields)]
pub struct TitleButtons { pub settings: ButtonOverride, pub color: ButtonOverride, pub folder: ButtonOverride,
    pub minimize: ButtonOverride, pub maximize: ButtonOverride, pub close: ButtonOverride }
```

  `Default` de `TitleButtons` pose les surcharges de `minimize` / `maximize` / `close` des contraintes globales ; `settings`, `color`, `folder` sont `ButtonOverride::default()` (visible, rien d'autre). `Config.titlebar: TitleBar`.

- [ ] **Étape 1 : tests qui échouent** dans `titlebar_tests.rs` (et le test de clés de `example.rs` mis à jour)

```rust
#[test]
fn defaults_match_the_mockup() {
    let t = TitleBar::default();
    assert!(t.enabled);
    assert_eq!((t.height, t.padding, t.spacing, t.resize_border), (28, 6, 4, 6));
    assert_eq!(t.background, Rgba8::new(0x14, 0x14, 0x14, 0xff));
    assert_eq!(t.gloss, 0.10);
    let close = t.buttons.close.resolve(&t.button);
    assert_eq!((close.size, close.radius, close.border_width), (14, 7, 1));
    assert_eq!(close.background, Rgba8::new(0xff, 0x5f, 0x57, 0xff));
    assert_eq!(close.icon_color, Rgba8::new(0, 0, 0, 0));
    let settings = t.buttons.settings.resolve(&t.button);
    assert_eq!(settings, t.button, "sans surcharge, le style commun");
}

#[test]
fn overrides_only_what_they_name() {
    let c = Config::from_str("[titlebar.button]\nradius = 2\n[titlebar.buttons.close]\nbackground = \"#ff0000\"\n").unwrap();
    let close = c.titlebar.buttons.close.resolve(&c.titlebar.button);
    assert_eq!(close.radius, 7, "la surcharge par défaut du bouton garde son rayon");
    assert_eq!(close.background, Rgba8::new(255, 0, 0, 255));
    let settings = c.titlebar.buttons.settings.resolve(&c.titlebar.button);
    assert_eq!(settings.radius, 2);
}

#[test]
fn bounds_are_validated_with_the_key_name() {
    for (toml, key) in [
        ("[titlebar]\nheight = 8\n", "titlebar.height"),
        ("[titlebar]\ngloss = 1.5\n", "titlebar.gloss"),
        ("[titlebar.button]\nsize = 40\n", "titlebar.button.size"),
        ("[titlebar.buttons.close]\nradius = 99\n", "titlebar.buttons.close.radius"),
        ("[titlebar]\nresize_border = 17\n", "titlebar.resize_border"),
    ] {
        let err = Config::from_str(toml).unwrap_err().to_string();
        assert!(err.contains(key), "{toml}: {err}");
    }
    assert!(Config::from_str("[titlebar]\nvisible = true\n").is_err(), "clé inconnue");
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty-config titlebar` → compilation impossible.

- [ ] **Étape 3 : implémenter** `titlebar.rs`, le champ `Config.titlebar`, la validation dans `Config::validate` avec la fermeture `bounded` existante (`size` borné à `8..=titlebar.height` sur chaque bouton **résolu**, `gloss` vérifié à la main car `bounded` prend des `u32`), la section du modèle `DEFAULT_TOML` (chaque clé commentée en français, les six sous-tables `[titlebar.buttons.*]` présentes, celles de gauche avec seulement `visible = true`), et `docs/rustty.example.toml`.

- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert, dont `every_section_and_key_is_documented` et `example_is_published`.

- [ ] **Étape 5 : commit** — alpha.139 « Section [titlebar] de la configuration ».

### Task 3 : quads stylés (coins arrondis, bordure, dégradé)

**Files:**
- Modify: `crates/rustty-render/src/pipeline/quad.rs` (`QuadInstance`, attributs de vertex)
- Modify: `crates/rustty-render/src/shaders/quad.wgsl`
- Modify: `crates/rustty-render/src/frame.rs` (`ChromeShape`, `Chrome.shapes`)
- Modify: `crates/rustty-render/src/renderer.rs:121-129` (les formes rejoignent `overlay` après les quads)
- Modify: `crates/rustty-render/src/lib.rs` (exports `ChromeShape`, `QuadStyle`)
- Modify: `crates/rustty-render/src/chrome/mod.rs:239` (`Chrome { …, ..Default::default() }`)
- Test: `crates/rustty-render/tests/offscreen.rs` + nouvelle image `tests/golden/styled_quad.png`

**Interfaces:**
- Produces:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadStyle { pub fill: Rgba, pub fill_bottom: Rgba, pub radius: f32, pub border_width: f32, pub border: Rgba }
impl QuadStyle { pub fn flat(color: Rgba) -> Self /* fill = fill_bottom = color, radius 0, bordure 0 transparente */ }
#[repr(C)] pub struct QuadInstance { pos: [f32; 2], size: [f32; 2], color: [f32; 4], color_bottom: [f32; 4], border_color: [f32; 4], radius: f32, border_width: f32, _pad: [f32; 2] }
impl QuadInstance { pub fn new(x, y, w, h, color: Rgba) -> Self /* inchangé : style plat */ ; pub fn styled(x: f32, y: f32, w: f32, h: f32, style: QuadStyle) -> Self }
#[derive(Clone, Debug, PartialEq)] pub struct ChromeShape { pub rect: PixelRect, pub style: QuadStyle }
pub struct Chrome { pub quads: Vec<ChromeQuad>, pub shapes: Vec<ChromeShape>, pub texts: Vec<ChromeText> }
```

  Shader : attributs `0..=6` (`pos`, `size`, `color`, `color_bottom`, `border_color`, `radius`, `border_width`) ; le vertex transmet `pos`, `size` et les paramètres ; le fragment calcule `p = frag.xy - (pos + size / 2)`, `q = abs(p) - size / 2 + radius`, `d = length(max(q, 0)) + min(max(q.x, q.y), 0) - radius`, `coverage = 1 - smoothstep(-0.5, 0.5, d)`, `t = (frag.y - pos.y) / size.y`, couleur `mix(color, color_bottom, t)` puis `mix(couleur, border_color, step(-border_width, d))` quand `border_width > 0`, alpha × `coverage`. Avec `radius = 0` et `border_width = 0`, un rectangle aligné sur la grille donne `coverage = 1` sur tous ses pixels : sortie identique à l'actuelle.

- [ ] **Étape 1 : test qui échoue** dans `offscreen.rs`

```rust
#[test]
fn styled_quad_matches_golden() {
    use rustty_render::{ChromeShape, QuadStyle};
    let Some(ctx) = common::gpu_or_skip() else { return; };
    let mut r = renderer(&ctx);
    let (w, h) = (64, 40);
    let target = Offscreen::new(&ctx, w, h);
    let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
    let blue = Rgba::new(0.0, 0.0, 1.0, 1.0);
    let shapes = vec![
        ChromeShape { rect: PixelRect::new(4, 4, 24, 32), style: QuadStyle { fill: red, fill_bottom: blue, radius: 8.0, border_width: 2.0, border: Rgba::new(1.0, 1.0, 1.0, 1.0) } },
        ChromeShape { rect: PixelRect::new(36, 4, 24, 32), style: QuadStyle::flat(red) },
    ];
    let frame = Frame { viewport: (w, h), background: Rgba::new(0.0, 0.0, 0.0, 1.0), panes: vec![], chrome: Chrome { shapes, ..Default::default() } };
    r.render(&ctx, target.view(), &frame);
    let px = target.read_rgba(&ctx).unwrap();
    assert_matches_golden("styled_quad", &px, w, h);
    assert_eq!(pixel(&px, w, 4, 4), [0, 0, 0, 255], "le coin arrondi laisse voir le fond");
    assert_eq!(pixel(&px, w, 16, 5), [255, 255, 255, 255], "bordure en haut au milieu");
    assert_eq!(pixel(&px, w, 36, 4), [255, 0, 0, 255], "le quad plat garde son coin");
    let top = pixel(&px, w, 16, 8); let bottom = pixel(&px, w, 16, 31);
    assert!(top[0] > bottom[0] && bottom[2] > top[2], "dégradé rouge → bleu");
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty-render --test offscreen styled_quad` → compilation impossible.

- [ ] **Étape 3 : implémenter** `QuadStyle`, le nouvel `QuadInstance` (vérifier `size_of == 80`, attributs `vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32, 6 => Float32]`), le shader, `ChromeShape`, `Chrome.shapes`, le mappage dans `renderer.rs`, les exports.

- [ ] **Étape 4 : vérifier** — `UPDATE_GOLDEN=1 cargo test -p rustty-render --test offscreen styled_quad` crée l'image, puis `cargo test --workspace` vert **sans** `UPDATE_GOLDEN` : toutes les anciennes images de référence passent encore (`git status` ne montre aucune image modifiée sauf la nouvelle).

- [ ] **Étape 5 : commit** — alpha.140 « Quads à coins arrondis, bordure et dégradé » (`git add` de `tests/golden/styled_quad.png` inclus).

### Task 4 : disposition et chrome de la barre de titre

**Files:**
- Create: `crates/rustty-render/src/chrome/title_bar.rs`
- Create: `crates/rustty-render/src/chrome/title_bar_tests.rs`
- Modify: `crates/rustty-render/src/chrome/mod.rs` (`mod title_bar; pub use title_bar::…`), `lib.rs` (ré-exports)
- Test: `crates/rustty-render/tests/offscreen.rs` + `tests/golden/title_bar.png`

**Interfaces:**
- Consumes: `rustty_config::{TitleBar, ButtonStyle}`, `Rgba::from_rgba8`, `Rgba::lighten`, `QuadStyle`, `ChromeShape`, `title_fit::fit_title` (rendre `pub(super)` suffisant : même module parent).
- Produces:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TitleButton { Settings, Color, Folder, Minimize, Maximize, Close }
impl TitleButton { pub const ALL: [TitleButton; 6]; pub fn is_window_control(self) -> bool /* Minimize, Maximize, Close */ }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonLook { pub size: u32, pub background: Rgba, pub border: Rgba, pub border_width: u32, pub radius: u32, pub icon: Rgba, pub hover_background: Rgba, pub hover_border: Rgba, pub hover_icon: Rgba }
#[derive(Clone, Debug, PartialEq)]
pub struct TitleBarStyle { pub height: u32, pub background: Rgba, pub gloss: f32, pub separator: Rgba, pub title_color: Rgba, pub padding: u32, pub spacing: u32, pub buttons: Vec<(TitleButton, ButtonLook)> }
impl TitleBarStyle {
    /// Tailles converties en pixels physiques ; ce jalon ne garde que les boutons de fenêtre visibles.
    pub fn from_config(cfg: &rustty_config::TitleBar, scale_factor: f64) -> Self;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleTarget { None, Button(TitleButton), Title, Background }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleBarLayout { pub bar: PixelRect, pub separator: PixelRect, pub buttons: Vec<(TitleButton, PixelRect)>, pub title: PixelRect }
impl TitleBarLayout { pub fn hit_test(&self, x: u32, y: u32) -> TitleTarget; }
pub fn title_bar_height(style: &TitleBarStyle) -> u32 /* height + 1 (séparateur) */;
pub fn layout_title_bar(viewport_width: u32, style: &TitleBarStyle) -> TitleBarLayout;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitleBarState<'a> { pub title: &'a str, pub hover: Option<TitleButton>, pub maximized: bool }
pub fn title_bar_chrome(layout: &TitleBarLayout, style: &TitleBarStyle, state: TitleBarState, metrics: CellMetrics) -> Chrome;
```

  Disposition : boutons non-fenêtre de gauche à droite à partir de `bar.x + padding`, boutons de fenêtre de droite à gauche à partir de `bar.x + bar.width - padding` (ordre visuel Minimize, Maximize, Close), espacés de `spacing`, centrés verticalement ; `title` = ce qui reste entre les deux groupes moins `spacing` de chaque côté, largeur 0 (jamais négative : `saturating_sub`) s'il ne reste rien. `separator` = ligne de 1 px sous `bar`. Chrome : forme du fond (`fill = background.lighten(gloss)`, `fill_bottom = background`), liseré de 1 px en haut (`Rgba::new(1,1,1,gloss)`, omis si `gloss == 0`), séparateur, chaque bouton (look normal ou survolé), glyphe de bouton centré (ce jalon : `'–'` U+2013, `'□'` U+25A1 ou `'❐'` U+2750 si `maximized`, `'✕'` U+2715), titre tronqué par `fit_title(title, title.width / metrics.width)` centré dans `title`, de couleur `readable_on(style.background, style.title_color, Rgba::new(0.07, 0.07, 0.07, 1.0))` (spec « Lisibilité » : un fond clair impose un titre sombre ; sur le fond sombre par défaut, `readable_on` rend `title_color`).

- [ ] **Étape 1 : tests qui échouent** dans `title_bar_tests.rs`

```rust
fn style() -> TitleBarStyle { TitleBarStyle::from_config(&rustty_config::TitleBar::default(), 1.0) }
fn metrics() -> CellMetrics { CellMetrics { width: 8, height: 16, baseline: 12, underline_y: 14, underline_thickness: 1, strike_y: 8 } }

#[test]
fn style_scales_logical_sizes_to_physical() {
    let s = TitleBarStyle::from_config(&rustty_config::TitleBar::default(), 2.0);
    assert_eq!((s.height, s.padding, s.spacing), (56, 12, 8));
    assert_eq!(s.buttons.len(), 3, "ce jalon : réduire, agrandir, fermer");
    assert_eq!(s.buttons[0].1.size, 28);
    assert_eq!(s.buttons[0].1.radius, 14);
}

#[test]
fn window_controls_sit_at_the_right_in_order() {
    let l = layout_title_bar(400, &style());
    assert_eq!(l.bar, PixelRect::new(0, 0, 400, 28));
    assert_eq!(l.separator, PixelRect::new(0, 28, 400, 1));
    let names: Vec<_> = l.buttons.iter().map(|(b, _)| *b).collect();
    assert_eq!(names, [TitleButton::Minimize, TitleButton::Maximize, TitleButton::Close]);
    let close = l.buttons[2].1;
    assert_eq!(close, PixelRect::new(400 - 6 - 14, 7, 14, 14));
    assert_eq!(l.buttons[1].1.x, close.x - 4 - 14);
    assert_eq!(l.title, PixelRect::new(6 + 4, 0, l.buttons[0].1.x - 4 - 10, 28));
}

#[test]
fn hit_test_distinguishes_buttons_title_and_background() {
    let l = layout_title_bar(400, &style());
    assert_eq!(l.hit_test(390, 10), TitleTarget::Button(TitleButton::Close));
    assert_eq!(l.hit_test(200, 10), TitleTarget::Title);
    assert_eq!(l.hit_test(2, 2), TitleTarget::Background);
    assert_eq!(l.hit_test(200, 40), TitleTarget::None);
}

#[test]
fn narrow_window_keeps_buttons_and_drops_the_title() {
    let l = layout_title_bar(20, &style());
    assert_eq!(l.buttons.len(), 3);
    assert_eq!(l.title.width, 0);
    assert!(l.buttons.iter().all(|(_, r)| r.x + r.width <= 20 || r.x == 0));
}

#[test]
fn chrome_has_gloss_hover_and_centered_title() {
    let s = style();
    let l = layout_title_bar(400, &s);
    let c = title_bar_chrome(&l, &s, TitleBarState { title: "zsh — rustty", hover: Some(TitleButton::Close), maximized: false }, metrics());
    assert_eq!(c.shapes[0].rect, l.bar);
    assert_eq!(c.shapes[0].style.fill, s.background.lighten(0.10));
    assert_eq!(c.shapes[1].rect, PixelRect::new(0, 0, 400, 1), "liseré");
    let close = c.shapes.iter().find(|sh| sh.rect == l.buttons[2].1).unwrap();
    assert_eq!(close.style.fill, s.buttons[2].1.hover_background);
    let title = c.texts.iter().find(|t| t.text == "zsh — rustty").unwrap();
    assert_eq!(title.x, l.title.x + (l.title.width - 12 * 8) / 2);
    assert_eq!(title.color, s.title_color);
    assert!(c.texts.iter().any(|t| t.text == "✕" && t.color == s.buttons[2].1.hover_icon));
}

#[test]
fn light_background_darkens_the_title() {
    let mut s = style();
    s.background = Rgba::new(0.95, 0.85, 0.2, 1.0);
    let l = layout_title_bar(400, &s);
    let c = title_bar_chrome(&l, &s, TitleBarState { title: "x", hover: None, maximized: false }, metrics());
    let title = c.texts.iter().find(|t| t.text == "x").unwrap();
    assert_eq!(title.color, Rgba::new(0.07, 0.07, 0.07, 1.0));
}

#[test]
fn zero_gloss_gives_a_flat_bar() {
    let mut s = style();
    s.gloss = 0.0;
    let l = layout_title_bar(100, &s);
    let c = title_bar_chrome(&l, &s, TitleBarState { title: "", hover: None, maximized: true }, metrics());
    assert_eq!(c.shapes[0].style.fill, s.background);
    assert_eq!(c.shapes[0].style.fill_bottom, s.background);
    assert_eq!(c.shapes[1].rect, l.separator, "pas de liseré : la deuxième forme est le séparateur");
    assert!(c.texts.iter().any(|t| t.text == "❐"), "agrandi : icône restaurer");
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty-render title_bar` → compilation impossible.

- [ ] **Étape 3 : implémenter** `title_bar.rs` (≤ 300 lignes ; si besoin, `title_bar_layout.rs` pour la disposition et `title_bar.rs` pour le chrome), exports dans `chrome/mod.rs` et `lib.rs`.

- [ ] **Étape 4 : image de référence** — ajouter dans `offscreen.rs` `title_bar_matches_golden` : viewport 320 × 30, style par défaut à l'échelle 1, `TitleBarState { title: "1: zsh — rustty 0.1", hover: Some(TitleButton::Maximize), maximized: false }`, `assert_matches_golden("title_bar", …)`, plus `pixel` au centre du bouton fermer = `#ff5f57` et au centre du bouton agrandir = `#3dc84f` (survolé mais même fond). Créer avec `UPDATE_GOLDEN=1`, puis vérifier sans.

- [ ] **Étape 5 : vérifier** — `cargo test --workspace` vert.

- [ ] **Étape 6 : commit** — alpha.141 « Disposition et dessin de la barre de titre » (image incluse).

### Task 5 : la géométrie réserve la barre de titre

**Files:**
- Modify: `crates/rustty/src/geometry.rs:21-43` et ses tests

**Interfaces:**
- Produces: `window_geometry(width, height, position, tab_count, min_tabs, bar_height, title_height: u32) -> WindowGeometry` avec un champ de plus `pub title_bar_y: Option<u32>` (`Some(0)` si `title_height > 0`). La barre d'onglets en haut commence à `title_height` ; `content` commence sous les deux ; en bas, `content_height = height - title_height - bar_height` (saturé). Tous les appels existants passent `0` jusqu'à la tâche 6.

- [ ] **Étape 1 : tests qui échouent** (dans le module `tests` de `geometry.rs`, les appels existants gagnent un `0` final)

```rust
#[test]
fn title_bar_pushes_tabs_and_content_down() {
    let g = window_geometry(800, 600, TabBarPosition::Top, 1, 1, 24, 29);
    assert_eq!(g.title_bar_y, Some(0));
    assert_eq!(g.tab_bar_y, Some(29));
    assert_eq!(g.content, Rect::new(0, 53, 800, 547));
    let b = window_geometry(800, 600, TabBarPosition::Bottom, 1, 1, 24, 29);
    assert_eq!(b.title_bar_y, Some(0));
    assert_eq!(b.tab_bar_y, Some(576));
    assert_eq!(b.content, Rect::new(0, 29, 800, 547));
}

#[test]
fn disabled_title_bar_reserves_nothing() {
    let g = window_geometry(800, 600, TabBarPosition::Top, 1, 1, 24, 0);
    assert_eq!(g.title_bar_y, None);
    assert_eq!(g.content, Rect::new(0, 24, 800, 576));
}

#[test]
fn title_bar_alone_in_a_tiny_window_never_underflows() {
    let g = window_geometry(10, 10, TabBarPosition::Hidden, 1, 1, 24, 29);
    assert_eq!(g.content.height, 0);
    assert_eq!(g.title_bar_y, Some(0));
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty geometry` → signature inconnue.
- [ ] **Étape 3 : implémenter**, mettre à jour l'appel dans `relayout.rs` avec `0`.
- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert.
- [ ] **Étape 5 : commit** — alpha.142 « La géométrie réserve la barre de titre ».

### Task 6 : la fenêtre dessine la barre et retire les décorations

**Files:**
- Create: `crates/rustty/src/title_bar.rs` (impl `OsWindow` : `title_style`, `relayout_title_bar`, `title_bar_chrome_for_frame`, `window_title_text`, `set_decorations_for`)
- Modify: `crates/rustty/src/window_state.rs` (champs `title_style: TitleBarStyle`, `title_bar: Option<TitleBarLayout>`, `title_hover: Option<TitleButton>`, `maximized: bool` ; `open` : `.with_decorations(!config.titlebar.enabled)` ; `apply_config`)
- Modify: `crates/rustty/src/relayout.rs` (hauteur passée à `window_geometry`, `self.title_bar = …`)
- Modify: `crates/rustty/src/render.rs` (chrome de la barre ajouté avant celui des onglets ; `chrome.shapes.extend`)
- Modify: `crates/rustty/src/app.rs` (`Resized` : `w.maximized = w.window.is_maximized()` avant `Relayout` ; `ScaleFactorChanged` : `w.title_style` refait)
- Modify: `crates/rustty/src/main.rs` ou `lib.rs` (`mod title_bar;`)
- Test: `crates/rustty/src/title_bar_tests.rs` pour `window_title_text` et la décision de décorations

**Interfaces:**
- Consumes: tâches 4 et 5.
- Produces:
  - `pub fn decorations_wanted(cfg: &rustty_config::TitleBar) -> bool` (= `!cfg.enabled`), fonction pure.
  - `pub fn decorations_change(old: &TitleBar, new: &TitleBar) -> Option<bool>` : `Some(decorations_wanted(new))` si `enabled` change, sinon `None`.
  - `OsWindow::title_bar_height(&self) -> u32` : `title_bar_height(&self.title_style)` si `config.titlebar.enabled`, sinon 0.
  - `OsWindow::window_title_text(&self) -> String` : le texte déjà donné à winit par `update_title` (factoriser : `update_title` appelle `window_title_text` puis `set_title`).

- [ ] **Étape 1 : tests qui échouent** dans `title_bar_tests.rs`

```rust
#[test]
fn apply_config_toggles_decorations() {
    let on = rustty_config::TitleBar::default();
    let mut off = on.clone();
    off.enabled = false;
    assert!(!decorations_wanted(&on));
    assert!(decorations_wanted(&off));
    assert_eq!(decorations_change(&on, &off), Some(true));
    assert_eq!(decorations_change(&on, &on), None);
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty title_bar` → compilation impossible.

- [ ] **Étape 3 : implémenter** :
  - `open` : `with_decorations(!config.titlebar.enabled)`, `title_style = TitleBarStyle::from_config(&config.titlebar, window.scale_factor())`, `maximized = window.is_maximized()`.
  - `relayout` : `title_height = self.title_bar_height()` passé à `window_geometry` ; `self.title_bar = g.title_bar_y.map(|y| layout_title_bar(w, &self.title_style))` (la barre est à `y = 0`).
  - `render` : si `title_bar` est présent, `title_bar_chrome(bar, &self.title_style, TitleBarState { title: &self.window_title_text(), hover: self.title_hover, maximized: self.maximized }, metrics)`, ses `shapes` et `texts` ajoutés à `chrome` ; le bandeau d'erreur réserve aussi `title_bar_height` quand il se place en haut (vérifier `banner_chrome`, même logique que `bar_at_bottom`).
  - `apply_config` : `title_style` refait ; `if let Some(d) = decorations_change(&self.config.titlebar, &new.titlebar) { self.window.set_decorations(d) }` **avant** `self.config = new`.
  - `ScaleFactorChanged` : `title_style` refait avec le nouveau facteur.

- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert ; `cargo run -p rustty` : la fenêtre n'a plus de décoration système, une barre noire à reflet avec trois pastilles à droite et le titre au centre apparaît ; les onglets et panneaux sont dessous ; `enabled = false` dans le TOML (rechargé à chaud) rend les décorations grises de winit ; `enabled = true` les retire. Noter le résultat observé dans le message de commit.

- [ ] **Étape 5 : commit** — alpha.143 « La fenêtre dessine sa barre de titre ».

### Task 7 : bords de redimensionnement et seuil de déplacement (pur)

**Files:**
- Create: `crates/rustty/src/window_edges.rs` + `window_edges_tests.rs`
- Create: `crates/rustty/src/title_input.rs` (machine pure `TitleDrag` dans ce jalon ; le câblage est la tâche 8) + `title_input_tests.rs`

**Interfaces:**
- Produces:

```rust
// window_edges.rs
pub fn resize_direction(x: f64, y: f64, width: u32, height: u32, border: u32, maximized: bool) -> Option<winit::window::ResizeDirection>;
pub fn resize_cursor(direction: ResizeDirection) -> winit::window::CursorIcon; // EwResize, NsResize, NwseResize (NW/SE), NeswResize (NE/SW)
// title_input.rs
pub const MOVE_THRESHOLD: f64 = 4.0;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TitleDrag { Idle, Armed { x: f64, y: f64 }, Moving }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleDragEvent { None, StartMove, Click }
impl TitleDrag {
    pub fn press(&mut self, button: MouseButton, x: f64, y: f64);           // seul Left arme
    pub fn moved(&mut self, x: f64, y: f64) -> TitleDragEvent;             // Armed + distance ≥ seuil → Moving, StartMove
    pub fn release(&mut self) -> TitleDragEvent;                           // Armed → Click ; Moving → None ; toujours Idle après
}
```

- [ ] **Étape 1 : tests qui échouent**

```rust
// window_edges_tests.rs
#[test]
fn edges_and_corners() {
    use winit::window::ResizeDirection as D;
    let f = |x, y| resize_direction(x, y, 800, 600, 6, false);
    assert_eq!(f(2.0, 300.0), Some(D::West));
    assert_eq!(f(797.0, 300.0), Some(D::East));
    assert_eq!(f(400.0, 1.0), Some(D::North));
    assert_eq!(f(400.0, 599.0), Some(D::South));
    assert_eq!(f(3.0, 3.0), Some(D::NorthWest));
    assert_eq!(f(796.0, 597.0), Some(D::SouthEast));
    assert_eq!(f(796.0, 2.0), Some(D::NorthEast));
    assert_eq!(f(2.0, 597.0), Some(D::SouthWest));
    assert_eq!(f(6.0, 300.0), None, "juste après la marge");
    assert_eq!(f(400.0, 300.0), None);
}

#[test]
fn no_edge_when_maximized_or_border_zero() {
    assert_eq!(resize_direction(1.0, 1.0, 800, 600, 6, true), None);
    assert_eq!(resize_direction(1.0, 1.0, 800, 600, 0, false), None);
}

// title_input_tests.rs
#[test]
fn only_the_left_button_arms_a_move() {
    let mut d = TitleDrag::Idle;
    d.press(MouseButton::Right, 10.0, 10.0);
    assert_eq!(d, TitleDrag::Idle);
    d.press(MouseButton::Left, 10.0, 10.0);
    assert_eq!(d, TitleDrag::Armed { x: 10.0, y: 10.0 });
}

#[test]
fn small_motion_is_a_click_large_motion_is_a_move() {
    let mut d = TitleDrag::Idle;
    d.press(MouseButton::Left, 10.0, 10.0);
    assert_eq!(d.moved(12.0, 11.0), TitleDragEvent::None);
    assert_eq!(d.release(), TitleDragEvent::Click);
    assert_eq!(d, TitleDrag::Idle);
    d.press(MouseButton::Left, 10.0, 10.0);
    assert_eq!(d.moved(14.0, 10.0), TitleDragEvent::StartMove, "4 px exactement");
    assert_eq!(d.moved(30.0, 10.0), TitleDragEvent::None, "déjà en mouvement");
    assert_eq!(d.release(), TitleDragEvent::None);
}
```

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty window_edges title_input` → compilation impossible.
- [ ] **Étape 3 : implémenter** les deux modules (`mod` déclarés dans `main.rs` / `lib.rs`).
- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert.
- [ ] **Étape 5 : commit** — alpha.144 « Bords de redimensionnement et seuil de déplacement ».

### Task 8 : câblage souris, boutons de fenêtre, actions clavier

**Files:**
- Modify: `crates/rustty/src/title_input.rs` (impl `OsWindow` : `title_press(&mut self, button, x, y) -> Option<Vec<Effect>>`, `title_moved(&mut self, x, y) -> Option<Vec<Effect>>`, `title_release(&mut self, button, x, y) -> Option<Vec<Effect>>`, `edge_press(&mut self, button, x, y) -> bool`, `title_button_action(&mut self, b: TitleButton) -> Vec<Effect>`)
- Modify: `crates/rustty/src/input.rs` (`on_press` : `edge_press` tout en haut, puis `title_press` juste après `spell_click` ; `on_cursor_moved` : `title_moved` juste après `drag_moved` ; `on_release` : `title_release` en premier)
- Modify: `crates/rustty/src/divider_drag.rs:45-66` (`update_cursor_icon` : curseur de bord en premier, via `resize_direction` + `resize_cursor`)
- Modify: `crates/rustty/src/window_state.rs` (champs `title_drag: TitleDrag`, `title_press: Option<TitleButton>`)
- Modify: `crates/rustty/src/app.rs` (`CloseRequested` délègue à `w.request_close_window()`)
- Modify: `crates/rustty/src/model.rs` (`Effect::{Minimize, ToggleMaximize}` ; `Action::MinimizeWindow => vec![Effect::Minimize]`, `Action::ToggleMaximize => vec![Effect::ToggleMaximize]`)
- Modify: `crates/rustty/src/effects.rs` (`Effect::Minimize => self.window.set_minimized(true)`, `Effect::ToggleMaximize => { let m = !self.window.is_maximized(); self.window.set_maximized(m); self.maximized = m; }`)
- Modify: `crates/rustty-config/src/action.rs` (`Simple::MinimizeWindow` ↔ `"minimize_window"`, `Simple::ToggleMaximize` ↔ `"toggle_maximize"`, tests d'analyse comme `rename_tab`) ; `keymap.rs` : aucun défaut.
- Modify: `crates/rustty-config/src/example.rs` (`[keys]` : les deux actions citées en commentaire comme les autres sans raccourci) et `docs/rustty.example.toml`.
- Test: `crates/rustty/src/title_input_tests.rs`

**Interfaces:**
- Consumes: tâches 4, 6, 7.
- Produces: `OsWindow::request_close_window(&mut self) -> Vec<Effect>` (le code aujourd'hui dans `app.rs:65-74`, déplacé dans `title_input.rs` ou `window_state.rs`), utilisé par `CloseRequested` et par le bouton fermer.
- Comportement :
  - `edge_press` : bouton gauche, `resize_direction(x, y, w, h, resize_border physique, self.maximized)` → `self.window.drag_resize_window(d)` (erreur ignorée avec `tracing::debug!`), vrai si pris.
  - `title_press` : `None` si pas de `title_bar` ou `hit_test == TitleTarget::None`. Ferme le menu (`self.menu = None`) et annule le renommage (`self.model.cancel_rename()`). `Button(b)` → `self.title_press = Some(b)`. `Title` / `Background` → `self.title_drag.press(button, x, y)`. Renvoie `Some(effects)` (au moins `Redraw` si le menu ou le renommage a changé).
  - `title_moved` : `self.title_drag.moved(x, y) == StartMove` → `self.window.drag_window()` (erreur ignorée) et `Some(vec![])`. Sinon, met à jour `title_hover` depuis `hit_test` (uniquement `Button(b)`) et renvoie `Some(vec![Redraw])` quand il change, `None` sinon pour laisser la suite s'exécuter.
  - `title_release` : si `title_press` est `Some(b)` : le remettre à `None`, et si `hit_test` donne encore `Button(b)` et bouton gauche → `title_button_action(b)`. Si `title_drag` n'est pas `Idle` : `release()` ; `Click` compte pour le double clic du titre dans le jalon 3, rien ici. Renvoie `Some` dès que la barre était concernée.
  - `title_button_action` : `Minimize → vec![Effect::Minimize]`, `Maximize → vec![Effect::ToggleMaximize]`, `Close → self.request_close_window()`, les autres → `vec![]` (jalon 2).

- [ ] **Étape 1 : tests qui échouent** dans `title_input_tests.rs` (fonctions pures ; les tests qui demandent une fenêtre winit restent manuels)

```rust
#[test]
fn maximize_button_restores_when_maximized() {
    // L'effet est le même ; c'est effects.rs qui lit is_maximized() : on vérifie le mappage.
    assert_eq!(button_effects(TitleButton::Maximize), vec![Effect::ToggleMaximize]);
    assert_eq!(button_effects(TitleButton::Minimize), vec![Effect::Minimize]);
    assert!(button_effects(TitleButton::Settings).is_empty(), "jalon 2");
}
```

  avec `pub fn button_effects(b: TitleButton) -> Vec<Effect>` pure dans `title_input.rs` (le cas `Close` passe par `request_close_window` et n'est pas dans cette fonction : `title_button_action` traite `Close` à part). Dans `rustty-config/src/action.rs`, ajouter aux tests existants `assert_eq!(parse("\"minimize_window\"").unwrap(), Action::MinimizeWindow)` et `toggle_maximize`.

- [ ] **Étape 2 : vérifier l'échec** — `cargo test -p rustty title_input && cargo test -p rustty-config action` → compilation impossible.
- [ ] **Étape 3 : implémenter** tout le câblage listé ci-dessus.
- [ ] **Étape 4 : vérifier** — `cargo test --workspace` vert ; `cargo run -p rustty` et vérifier à la main : survol des pastilles (icône apparaît), réduire, agrandir puis restaurer (icône change), fermer (confirmation si un programme tourne), glisser la barre déplace la fenêtre, clic simple sur la barre ne la déplace pas, les huit bords et coins redimensionnent avec le bon curseur, rien ne redimensionne quand la fenêtre est agrandie, clic droit sur la barre ne fait rien, un menu d'onglet ouvert se ferme en cliquant la barre. Noter ce qui a été observé dans le commit.
- [ ] **Étape 5 : commit** — alpha.145 « Boutons de fenêtre, déplacement et redimensionnement ».

### Task 9 : documentation, E2E et publication

**Files:**
- Modify: `README.md` (section « Configuration » : `[titlebar]` en une phrase par clé importante ; section « Souris » : déplacer par la barre, bords ; mention que `decorations_theme` ne sert que si `enabled = false`)
- Modify: `docs/e2e.md` (section « Barre de titre (plan 11) » : les vérifications manuelles des tâches 6 et 8, plus `enabled = false` à chaud, HiDPI si disponible)
- Modify: `CHANGELOG.md` (entrée de la version, qui résume le jalon)
- Modify: `docs/superpowers/specs/2026-11-01-titlebar-design.md` : dans le tableau des actions, remplacer « `close_window` (existe déjà) » par « aucune action : `close_window` existant ferme le panneau ; le bouton passe par `request_close_window` ».

- [ ] **Étape 1 : rédiger** README, e2e, spec, CHANGELOG.
- [ ] **Étape 2 : vérifier** — `cargo test --workspace` vert (le README n'est pas testé, mais `docs/rustty.example.toml` l'est), relire le README une fois en entier pour qu'il ne contredise plus rien.
- [ ] **Étape 3 : commit** — alpha.146 « Barre de titre maison : documentation du jalon 1 », push, attendre la CI (filtrer `gh run list` par SHA), tag annoté `v0.1.0-alpha.146`, release GitHub en pré-version avec les notes de la section CHANGELOG.
- [ ] **Étape 4 : vitrines** — reportées au jalon 4 avec des captures ; le dire dans le compte rendu.
