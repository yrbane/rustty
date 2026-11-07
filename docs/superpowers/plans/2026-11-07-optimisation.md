# Optimisation et refactorisation — plan d'exécution

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Appliquer les six chantiers de l'audit (profil release, chemin d'impression, reflow et resize par image, objets GPU persistants, structure, hygiène) sans changer le comportement visible, chaque chantier mesuré avant/après.

**Architecture:** Les optimisations restent locales à leur crate (`rustty-vt` pour l'impression et le reflow, `rustty-render` pour les tampons GPU et les pipelines, `rustty` pour le relayout différé et l'assemblage de l'image). Les benchmarks `examples/audit.rs` sont la mesure ; la suite de tests, clippy et les images de référence sont la garde.

**Tech Stack:** Rust 2024, wgpu, vte 0.15, cargo (profils), clippy.

**Spec:** `docs/superpowers/specs/2026-11-07-optimisation-design.md`

## Global Constraints

- Aucun changement de comportement visible, de format de configuration ni de protocole.
- Pas de `panic = "abort"` (`catch_unwind` dans `rustty-pty`).
- Chaque tâche : `cargo fmt --all`, `cargo clippy --workspace --all-targets --examples` à 0 avertissement, `cargo test --workspace` vert, images de référence inchangées (`cargo test -p rustty-render --test offscreen` sans `UPDATE_GOLDEN`), un commit avec bump alpha + CHANGELOG (en tête, format `## 0.1.0-alpha.N — 2026-11-07 · « titre »`).
- Les cibles chiffrées sont mesurées avec `cargo run --release -p rustty-vt --example audit` et `cargo run --release -p rustty-render --example audit`, résultats consignés dans `docs/perf.md`.
- Une optimisation sans mesure avant/après n'existe pas (anti-objectif de la spec).

## Review Focus

1. Chemin rapide d'impression sur la moitié gauche d'un caractère large : la moitié droite doit être blanchie comme avant — Task 2, `overwriting_the_left_half_of_a_wide_char_clears_the_right_half`.
2. Chemin rapide sur une cellule portant des combinants : ils doivent disparaître avec elle — Task 2, `overwriting_a_cell_drops_its_combining_marks`.
3. Reflow après le changement de représentation des combinants : un combinant sur une ligne repliée reste attaché à son caractère après redécoupage — Task 3, `combining_marks_follow_their_cell_through_reflow`.
4. Relayout différé : un redimensionnement de fenêtre suivi d'un `RedrawRequested` doit rendre avec les nouveaux rectangles (le différé est consommé avant le rendu) — Task 3, `render` appelle `flush_layout()` en premier ; vérifié par l'e2e « Splits » (glisser une barre : le texte se replie en direct).
5. Tampons GPU persistants : une image plus grande que la capacité fait croître le tampon une fois, puis plus rien ; le contenu reste exact — Task 4, `buffers_grow_once_for_a_larger_frame_then_stay`, image de référence `window` inchangée.

---

### Task 1: Profil release et tableau de référence

**Files:**
- Modify: `Cargo.toml` (racine : `[profile.release]`)
- Create: `docs/perf.md`
- Modify: `README.md` (section développement : une phrase sur les benchmarks)

**Interfaces:**
- Produces: `docs/perf.md` avec un tableau « avant » (chiffres de la spec § 1) et une colonne « après profil » ; les tâches suivantes y ajoutent leur colonne.

- [ ] **Step 1: Mesure de référence**

Run: `ls -l target/release/rustty && cargo run --release -q -p rustty-vt --example audit && cargo run --release -q -p rustty-render --example audit 2>/dev/null`
Expected: binaire ≈ 21,9 MiB ; chiffres ≈ spec § 1 (texte brut ≈ 90 MiB/s, resize ≈ 30 ms, render ≈ 770 µs).

- [ ] **Step 2: Profil**

Dans `Cargo.toml` racine :
```toml
[profile.release]
lto = "fat"
codegen-units = 1
opt-level = 3
strip = "symbols"
```

- [ ] **Step 3: Mesure après**

Run: `cargo build --release -p rustty && ls -l target/release/rustty && cargo run --release -q -p rustty-vt --example audit && cargo run --release -q -p rustty-render --example audit 2>/dev/null`
Expected: binaire ≤ 12 MiB ; benchmarks égaux ou meilleurs. Écrire les deux colonnes dans `docs/perf.md` (tableau par bench : nom, avant, après Task 1, cible) et la procédure de relance.

- [ ] **Step 4: Vérification et commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets --examples`
Expected: verts, 0 avertissement.

```bash
git add Cargo.toml docs/perf.md README.md Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.184 : profil release (LTO, une unité de codegen, symboles retirés)"
```

---

### Task 2: Chemin d'impression

**Files:**
- Modify: `crates/rustty-vt/src/line.rs:55-63` (`get_mut`, `set`), `crates/rustty-vt/src/term/print.rs` (`put_char`), `crates/rustty-vt/src/grid.rs:67-80` (`scroll_up`)
- Test: `crates/rustty-vt/src/term/print.rs` (`mod tests`), `crates/rustty-vt/src/grid.rs` (`mod tests`)

**Interfaces:**
- Consumes: rien.
- Produces: `Grid::scroll_up(&mut self, top, bottom, n, template) -> Vec<Line>` inchangé en signature, les lignes rendues sont les lignes évincées elles-mêmes (déplacées, pas clonées) ; `Line::set` et `Line::get_mut` inchangés en signature.

- [ ] **Step 1: Tests de garde (doivent déjà passer : ils pincent le comportement que le chemin rapide doit conserver)**

Dans `print.rs` `mod tests` (helpers `term`, `feed` de `crate::term::test_support`) :
```rust
#[test]
fn overwriting_the_left_half_of_a_wide_char_clears_the_right_half() {
    let mut t = term(6, 1);
    feed(&mut t, "漢\x1b[1Ga");
    assert_eq!(t.grid().cell(0, 0).c, 'a');
    assert_eq!(t.grid().cell(1, 0).c, ' ');
    assert!(!t.grid().cell(1, 0).is_wide_continuation());
}

#[test]
fn overwriting_a_cell_drops_its_combining_marks() {
    let mut t = term(6, 1);
    feed(&mut t, "e\u{301}\x1b[1Gx");
    assert_eq!(t.grid().line(0).zerowidth(0), None);
    assert_eq!(t.grid().cell(0, 0).c, 'x');
}
```
Dans `grid.rs` `mod tests` :
```rust
#[test]
fn scroll_up_returns_the_evicted_lines_and_blanks_the_bottom() {
    let mut g = Grid::new(3, 3);
    for r in 0..3 { g.line_mut(r).set(0, Cell::new(char::from(b'a' + r as u8), Default::default())); }
    let evicted = g.scroll_up(0, 2, 1, Cell::default());
    assert_eq!(evicted.len(), 1);
    assert_eq!(evicted[0].text(), "a  ");
    assert_eq!(g.lines()[0].text(), "b  ");
    assert_eq!(g.lines()[2].text(), "   ");
}
```
(Adapter `Grid::new` / `cell(col,row)` / `line(row)` aux accesseurs existants.)

- [ ] **Step 2: Les lancer**

Run: `cargo test -p rustty-vt -- overwriting_ scroll_up_returns`
Expected: PASS (3 tests). Ils sont la garde, pas le RED : le RED est le benchmark sous la cible.

- [ ] **Step 3: Mesure avant**

Run: `cargo run --release -q -p rustty-vt --example audit | grep -E 'texte brut|SGR dense|CR seul' -A1`
Expected: texte brut < 200 MiB/s (RED du chantier).

- [ ] **Step 4: Implémenter**

- `Line::set` / `Line::get_mut` : `if !self.zerowidth.is_empty() { self.zerowidth.remove(&col); }`.
- `Term::put_char` : avant la logique générale, chemin rapide quand `width == 1 && !self.cursor.pending_wrap && !self.modes.insert` : lire une fois la cellule cible ; si elle n'est ni `is_wide()` ni `is_wide_continuation()`, écrire `Cell::with_link(c, self.cursor.style, self.current_link)` via `line.set` puis avancer le curseur (même règle `col + 1 < cols` sinon `pending_wrap`), et retourner. Sinon, chemin général inchangé.
- `Grid::scroll_up` : remplacer `to_vec()` + `reset` par : pour chaque ligne évincée, `std::mem::replace(&mut self.lines[top + i], Line::filled(cols, template))` collecté dans `evicted`, puis `rotate_left(n)` du segment `top..=bottom` (les lignes vierges finissent en bas). Même résultat, zéro clone de cellules.

- [ ] **Step 5: Mesure après et suite**

Run: `cargo run --release -q -p rustty-vt --example audit | grep -E 'texte brut|SGR dense' -A1 && cargo test --workspace`
Expected: texte brut ≥ 200 MiB/s, SGR dense ≥ 120 MiB/s, suite verte. Si la cible n'est pas atteinte : mesurer `CR seul` pour isoler, puis retirer dans l'ordre `charsets.map` (court-circuit quand le jeu actif est ASCII : `Charsets::is_ascii()`), et `c.width()` (court-circuit `c.is_ascii_graphic() || c == ' '` → 1). Consigner la colonne dans `docs/perf.md`.

- [ ] **Step 6: Commit**

```bash
git add crates/rustty-vt/src docs/perf.md Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.185 : chemin d'impression rapide"
```

---

### Task 3: Reflow et relayout par image

**Files:**
- Modify: `crates/rustty-vt/src/reflow/mod.rs` (`Entry`, `Logical`, `append`, `layout`), `crates/rustty-vt/src/reflow/perf.rs`, `crates/rustty-vt/src/reflow/tests.rs`
- Modify: `crates/rustty/src/window_state.rs` (champ `layout_dirty: bool`), `crates/rustty/src/effects.rs:59-62` (`Effect::Relayout`), `crates/rustty/src/render.rs` (`render` commence par `flush_layout()`), `crates/rustty/src/relayout.rs` (`flush_layout`)
- Test: `crates/rustty-vt/src/reflow/tests.rs`, `crates/rustty-vt/src/reflow/perf.rs`

**Interfaces:**
- Produces: `OsWindow::flush_layout(&mut self)` (`pub(crate)`, appelle `relayout()` si `layout_dirty`, remet à `false`) ; `Effect::Relayout` ne fait plus que `layout_dirty = true` + `request_redraw()`. Tout code qui a besoin des rectangles à jour dans le même événement appelle `flush_layout()` (seul cas : `open`, qui appelle `relayout()` directement, inchangé).

- [ ] **Step 1: Tests**

Dans `reflow/tests.rs` :
```rust
#[test]
fn combining_marks_follow_their_cell_through_reflow() {
    let mut l = Line::new(6);
    for (c, ch) in "abcde".chars().enumerate() { l.set(c, Cell::new(ch, Default::default())); }
    l.push_zerowidth(3, '\u{301}');
    let (out, _) = reflow(vec![l], &[], 3);
    assert_eq!(out.len(), 2);
    assert_eq!(out[1].zerowidth(0), Some("\u{301}"), "le d et son accent passent sur la 2e ligne");
    assert_eq!(out[0].zerowidth(3), None);
}
```
Dans `reflow/perf.rs` : resserrer la borne du test existant à `from_millis(20)` ; ajouter
```rust
#[test]
fn reflowing_ten_thousand_long_lines_stays_bounded() {
    // 10 000 lignes de 157 caractères, 200 → 120 colonnes : tout se replie.
    let lines: Vec<Line> = (0..10_000).map(|i| { let mut l = Line::new(200); for (c, ch) in format!("{i:06} {}", "x".repeat(150)).chars().enumerate() { l.set(c, Cell::new(ch, Default::default())); } l }).collect();
    let start = std::time::Instant::now();
    let (out, _) = reflow(lines, &[], 120);
    assert_eq!(out.len(), 20_000);
    assert!(start.elapsed() < std::time::Duration::from_millis(1_000), "{:?}", start.elapsed());
}
```

- [ ] **Step 2: Les lancer**

Run: `cargo test -p rustty-vt -- reflow`
Expected: `combining_marks_follow…` PASS (garde), les deux perf PASS en debug (bornes larges). RED du chantier = bench `resize 200→120→200` > 10 ms.

- [ ] **Step 3: Implémenter le reflow**

- `Entry` devient `Cell` seul ; `Logical` gagne `zerowidth: Vec<(usize, String)>` (indice d'entrée, marques), rempli dans `append` seulement pour les colonnes présentes dans `line.zerowidth` (parcourir la table de la ligne, pas chaque cellule : ajouter `Line::zerowidth_entries(&self) -> impl Iterator<Item = (usize, &str)>`), et consommé dans `layout` par un curseur sur ce vecteur (trié par indice).
- `layout` : `Line::new(cols)` reste ; les lignes de sortie sont poussées sans réallocation intermédiaire (déjà le cas).
- `append` : réserver `entries` à `line.len()` à la première ligne (`reserve`).

- [ ] **Step 4: Mesure**

Run: `cargo run --release -q -p rustty-vt --example audit | grep resize && cargo test -p rustty-vt`
Expected: `resize 200→120→200` ≤ 10 ms ; tests verts. Sinon : profiler avec le bench en isolant `append` (collecte) et `layout` (émission) par deux mesures séparées ajoutées temporairement, et réduire celui qui domine (candidat : `Line::new` → `Line::filled` avec `Vec::with_capacity` + `resize`, ou réutiliser les `Vec<Cell>` des lignes d'entrée consommées).

- [ ] **Step 5: Relayout différé**

`window_state.rs` : champ `pub(crate) layout_dirty: bool` (init `false`). `effects.rs` : `Effect::Relayout => { self.layout_dirty = true; self.window.request_redraw(); }`. `relayout.rs` : `pub(crate) fn flush_layout(&mut self) { if std::mem::take(&mut self.layout_dirty) { self.relayout(); } }`. `render.rs` : première ligne de `render()` : `self.flush_layout();`. Vérifier par `grep -n 'relayout()' crates/rustty/src` que les autres appelants directs (`open`, rechargement de config, changement de police) restent cohérents : ceux qui lisent `pane_rects` juste après gardent l'appel direct.

- [ ] **Step 6: Suite et commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets --examples`
Expected: verts, 0 avertissement. Colonne `docs/perf.md`.

```bash
git add crates/rustty-vt/src crates/rustty/src docs/perf.md Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.186 : reflow allégé et relayout une fois par image"
```

---

### Task 4: Objets GPU persistants

**Files:**
- Create: `crates/rustty-render/src/pipeline/common.rs` (`InstanceBuffer`, `Globals`)
- Modify: `crates/rustty-render/src/pipeline/mod.rs`, `quad.rs`, `glyph.rs`, `image.rs`, `crates/rustty-render/src/renderer.rs` (`draw`, champs), `crates/rustty-render/src/atlas.rs` ou `pipeline/glyph.rs` (bind group de l'atlas conservé)
- Test: `crates/rustty-render/src/renderer/tests.rs`

**Interfaces:**
- Produces:
  - `pub(crate) struct InstanceBuffer { buffer: wgpu::Buffer, capacity: u64, count: u32 }` avec `new(device, label, usage) -> Self`, `write<T: bytemuck::Pod>(&mut self, device, queue, data: &[T]) -> bool` (vrai si le tampon a été recréé : doublement jusqu'à contenir `data`), `count(&self) -> u32`, `slice(&self) -> wgpu::BufferSlice<'_>`.
  - `pub(crate) struct Globals { buffer: wgpu::Buffer, group: wgpu::BindGroup, viewport: (u32, u32) }` avec `new(device, layout, viewport) -> Self`, `ensure(&mut self, device, queue, layout, viewport) -> bool` (réécrit si le viewport change ; ne recrée jamais le bind group), `group(&self) -> &wgpu::BindGroup`.
  - `Renderer` : champs `bg: InstanceBuffer, overlay: InstanceBuffer, pane_glyphs: InstanceBuffer, chrome_glyphs: InstanceBuffer, image_instances: InstanceBuffer, quad_globals: Globals, glyph_globals: Globals, image_globals: Globals`, et `pub(crate) gpu_objects: u32` (incrémenté à chaque création de tampon ou de bind group, hors textures d'image) exposé par `#[doc(hidden)] pub fn gpu_objects(&self) -> u32`.
  - `QuadPipeline::draw(&self, pass, globals: &Globals, instances: &InstanceBuffer)` ; `GlyphPipeline::draw(&self, pass, globals, atlas_group: &wgpu::BindGroup, instances)` ; `ImagePipeline::draw(&self, pass, globals, textures, instances, ids: &[u64])`. `prepare` disparaît des trois pipelines.
  - `AtlasTexture` garde son `bind_group: wgpu::BindGroup` créé à la construction et à chaque reconstruction (`rebuild`).

- [ ] **Step 1: Tests**

Dans `renderer/tests.rs` (helpers du fichier : contexte headless, renderer, cible hors écran, `Term`) :
```rust
#[test]
fn a_second_identical_frame_creates_no_gpu_objects() {
    let (ctx, mut r, target, snap) = fixture(40, 10);
    render_once(&ctx, &mut r, &target, &snap);
    render_once(&ctx, &mut r, &target, &snap);
    let after_two = r.gpu_objects();
    render_once(&ctx, &mut r, &target, &snap);
    assert_eq!(r.gpu_objects(), after_two);
}

#[test]
fn buffers_grow_once_for_a_larger_frame_then_stay() {
    let (ctx, mut r, target, small) = fixture(10, 2);
    render_once(&ctx, &mut r, &target, &small);
    render_once(&ctx, &mut r, &target, &small);
    let before = r.gpu_objects();
    let mut t = Term::new(120, 40, 0);
    t.input(&vec![b'x'; 120 * 40]);
    let big = t.snapshot();
    render_once(&ctx, &mut r, &target, &big);
    let grown = r.gpu_objects();
    assert!(grown > before, "croissance");
    render_once(&ctx, &mut r, &target, &big);
    assert_eq!(r.gpu_objects(), grown, "stable ensuite");
}
```
(`fixture(cols, rows)` et `render_once` : à écrire dans le module de tests si absents, sur le modèle de `tests/support/mod.rs`.)

- [ ] **Step 2: Les lancer**

Run: `cargo test -p rustty-render --lib -- gpu_objects a_second_identical buffers_grow`
Expected: erreur de compilation (`gpu_objects` absent).

- [ ] **Step 3: Implémenter**

`common.rs` comme ci-dessus ; `write` : si `data.len() * size_of::<T>() > capacity`, recréer avec `capacity = max(needed, capacity * 2)` (usage `VERTEX | COPY_DST`), sinon `queue.write_buffer`. Les trois pipelines ne gardent que `pipeline`, layouts, sampler ; `Renderer::draw` écrit les cinq tampons, appelle `ensure` sur les trois `Globals`, puis la passe. Le bind group de l'atlas est pris sur `self.atlas`. Compteur incrémenté dans `InstanceBuffer::new`/recréation, `Globals::new`, `AtlasTexture` (bind group), pas dans `ImageTextures` (déjà en cache par image, hors périmètre).

- [ ] **Step 4: Vérifier**

Run: `cargo test -p rustty-render && cargo run --release -q -p rustty-render --example audit 2>/dev/null`
Expected: tests et images de référence verts ; `Renderer::render` ≤ 600 µs ; colonne `docs/perf.md`.

- [ ] **Step 5: Commit**

```bash
git add crates/rustty-render/src docs/perf.md Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.187 : tampons et bind groups GPU persistants"
```

---

### Task 5: Structure de `rustty-render`

**Files:**
- Modify: `crates/rustty-render/src/pipeline/common.rs` (ajout `shader`, `instanced_pipeline`), `quad.rs`, `glyph.rs`, `image.rs` (constructeurs via `common`), `crates/rustty-render/src/grid.rs` (`pane_instances` découpé), `crates/rustty-render/src/renderer.rs` (`draw` découpé), `crates/rustty-render/src/frame.rs` (`PixelRect::contains_point`, `contains_px`), `crates/rustty-render/src/chrome/title_bar.rs:159` (`inside` → `contains_px`)
- Test: `crates/rustty-render/src/frame.rs` (`mod tests`), existants

**Interfaces:**
- Produces:
  - `pub(crate) fn shader(device, label: &str, wgsl: &str) -> wgpu::ShaderModule` ; `pub(crate) struct InstancedPipelineSpec<'a> { label: &'a str, shader: &'a wgpu::ShaderModule, bind_layouts: &'a [&'a wgpu::BindGroupLayout], stride: u64, attributes: &'a [wgpu::VertexAttribute], format: wgpu::TextureFormat }` ; `pub(crate) fn instanced_pipeline(device, spec: &InstancedPipelineSpec) -> wgpu::RenderPipeline` (blend alpha prémultiplié commun, `alpha_target`).
  - `impl PixelRect { pub fn contains_point(&self, x: f64, y: f64) -> bool; pub fn contains_px(&self, x: u32, y: u32) -> bool }` (bornes : `x >= self.x && x < self.x + self.width`, idem y).
  - `grid.rs` : `fn background_run(...)`, `fn glyph_for(cell, x, y, under_cursor, fg, palette) -> Option<GlyphRequest>`, `fn decorations_for(cell, hovered_link, x, y, cw, metrics, fg, out: &mut Vec<QuadInstance>)`, `fn cursor_instances(...)` ; `pane_instances` < 60 lignes.
  - `renderer.rs` : `struct Collected { backgrounds, requests, overlay, images, chrome_requests }` ; `fn collect(&self, frame) -> Collected` ; `fn upload(&mut self, ctx, collected, viewport) -> Uploaded` ; `fn pass(&self, encoder, view, load, uploaded)` ; `draw` < 40 lignes.

- [ ] **Step 1: Test**

`frame.rs` `mod tests` :
```rust
#[test]
fn contains_point_is_half_open() {
    let r = PixelRect::new(10, 20, 5, 5);
    assert!(r.contains_point(10.0, 20.0) && r.contains_point(14.9, 24.9));
    assert!(!r.contains_point(15.0, 20.0) && !r.contains_point(9.9, 20.0));
    assert!(r.contains_px(14, 24) && !r.contains_px(15, 24));
}
```

- [ ] **Step 2: Le lancer**

Run: `cargo test -p rustty-render --lib contains_point`
Expected: erreur de compilation.

- [ ] **Step 3: Implémenter** les interfaces ci-dessus ; remplacer `title_bar::inside` par `contains_px`. Découpages purement mécaniques (aucune logique ne change).

- [ ] **Step 4: Vérifier**

Run: `cargo test -p rustty-render && cargo clippy -p rustty-render --all-targets --examples`
Expected: verts (images de référence incluses), 0 avertissement. Mesure de duplication : le script Python de l'audit (fenêtres de 4 lignes) donne < 3 fenêtres entre les trois pipelines.

- [ ] **Step 5: Commit**

```bash
git add crates/rustty-render/src Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.188 : pipelines factorisés, pane_instances et draw découpés"
```

---

### Task 6: Structure de `rustty` et garde de taille

**Files:**
- Modify: `crates/rustty/src/render.rs` (`render` → `assemble` + `present`), `crates/rustty/src/window_state.rs` (`open` → `build_renderers`, `initial_state`), `crates/rustty/src/tooltip.rs` (`panel_quads`), `crates/rustty/src/context_menu/chrome.rs` (utilise `panel_quads`), `crates/rustty/src/pane_header.rs:60`, `crates/rustty/src/context_menu/layout.rs:39`, `crates/rustty/src/tooltip.rs:33` (→ `PixelRect::contains_point`)
- Create: `crates/rustty/tests/code_shape.rs`
- Test: `crates/rustty/tests/code_shape.rs`, `crates/rustty/src/tooltip.rs` (`mod tests`)

**Interfaces:**
- Consumes: `PixelRect::contains_point` (Task 5).
- Produces: `pub(crate) fn panel_quads(rect: PixelRect, palette: &Palette) -> [ChromeQuad; 2]` (cadre `palette.ansi[8]` puis fond `palette.background` en retrait de 1 px) ; `struct Assembled { base: (Vec<PaneView>, Chrome), sized: BTreeMap<SizeKey, (Vec<PaneView>, Chrome)>, overlay: Chrome, menu: Chrome }` ; `fn assemble(&self) -> Assembled` ; `fn present(&mut self, assembled: Assembled)`.

- [ ] **Step 1: Tests**

`crates/rustty/tests/code_shape.rs` :
```rust
//! Garde de forme : aucune fonction de plus de 100 lignes dans les sources
//! (tests exclus). Heuristique par accolades, suffisante pour un code formaté.
#[test]
fn no_function_exceeds_one_hundred_lines() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut offenders = Vec::new();
    for path in rust_sources(&root.join("crates")) { offenders.extend(long_functions(&path, 100)); }
    assert!(offenders.is_empty(), "{offenders:#?}");
}
```
avec `rust_sources(dir) -> Vec<PathBuf>` (récursif, `.rs`, exclut `*_tests.rs`, `tests.rs`, dossiers `tests` et `examples`) et `long_functions(path, max) -> Vec<String>` (« fichier:ligne nom (n lignes) », même heuristique que l'audit : une ligne qui commence par `fn`/`pub fn`/`pub(crate) fn`, profondeur d'accolades jusqu'au retour à zéro).

`tooltip.rs` `mod tests` :
```rust
#[test]
fn panel_quads_are_a_frame_and_an_inset_fill() {
    let p = Palette::from_config(&Colors::default(), false);
    let [frame, fill] = panel_quads(PixelRect::new(10, 10, 100, 50), &p);
    assert_eq!((frame.rect, frame.color), (PixelRect::new(10, 10, 100, 50), p.ansi[8]));
    assert_eq!((fill.rect, fill.color), (PixelRect::new(11, 11, 98, 48), p.background));
}
```

- [ ] **Step 2: Les lancer**

Run: `cargo test -p rustty --test code_shape && cargo test -p rustty --lib panel_quads`
Expected: `code_shape` FAIL en listant `render::render` (et `open` si encore > 100) ; `panel_quads` erreur de compilation.

- [ ] **Step 3: Implémenter** : `panel_quads` utilisé par `tooltip::chrome` et `context_menu::chrome::menu_chrome` ; les trois `contains`/`inside` → `PixelRect::contains_point` ; `render()` = `let a = self.assemble(); self.present(a);` ; `open` découpé (`build_renderers(&ctx, &surface, &config, &window, &palette) -> Renderers`, et la construction du `Self` dans `fn initial_state(...)` ou via `Default` des sous-états).

- [ ] **Step 4: Vérifier**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets --examples`
Expected: verts (dont `code_shape`), 0 avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/rustty/src crates/rustty/tests/code_shape.rs Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.189 : assemblage de l'image séparé de la présentation, garde de taille"
```

---

### Task 7: Hygiène et bilan

**Files:**
- Modify: `Cargo.toml` (racine : `[workspace.lints.clippy] perf = "warn"`, `base64 = "0.23"`), chaque `crates/*/Cargo.toml` (`[lints] workspace = true`), fichiers signalés par `clippy::perf`, `crates/rustty/src/input.rs` (`on_cursor_moved` prend un seul instantané), `crates/rustty/src/spell_input.rs` (`spell_hover(term, cell, snapshot: &Snapshot)`), `crates/rustty/src/link_input.rs` (`link_hover(term, cell, snapshot: &Snapshot)`, `link_under` prend le snapshot), `docs/perf.md` (colonne finale), `README.md`
- Test: existants

**Interfaces:**
- Produces: `spell_hover(&mut self, term: TermId, cell: Option<(usize, usize)>, snapshot: &Snapshot) -> bool`, `link_hover(&mut self, term, cell, snapshot: &Snapshot) -> bool` ; `link_modifiers_changed` prend son propre instantané.

- [ ] **Step 1: Lister**

Run: `cargo clippy --workspace --all-targets --examples -- -W clippy::perf 2>&1 | grep -E '^warning' -A2`
Expected: 14 avertissements.

- [ ] **Step 2: Implémenter** : corriger les 14 ; activer le lint au niveau workspace ; `base64` 0.23 si `cargo build` passe sans changement d'API (sinon ruling) ; un instantané par mouvement ; `docs/perf.md` : colonne « final » pour chaque bench, et le bilan (binaire, débit, resize, render, objets GPU par image, fonctions > 100 lignes = 0) ; README : phrase sur `docs/perf.md`.

- [ ] **Step 3: Vérifier**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets --examples && cargo tree -d --workspace | grep -c base64`
Expected: verts, 0 avertissement, `base64` sans doublon (0 ou une seule version listée).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock crates docs/perf.md README.md CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.190 : lints perf, base64 unifié, un instantané par mouvement, bilan perf"
```

---

## Self-review

- Spec § 2.1 → Task 1 ; § 2.2 → Task 2 ; § 2.3 → Task 3 ; § 2.4 → Task 4 ; § 2.5 → Tasks 5 et 6 ; § 2.6 → Task 7 ; § 2.7 ordre respecté.
- Types : `InstanceBuffer`/`Globals` (Task 4) réutilisés par Task 5 (`common.rs`) ; `PixelRect::contains_point` (Task 5) consommé par Task 6 ; `Snapshot` (rustty-vt) passé par référence en Task 7.
- Review Focus : 1, 2 → Task 2 ; 3 → Task 3 ; 4 → Task 3 (e2e) ; 5 → Task 4.
- Proportion : le plan décide les signatures, les tests et les cibles ; les corps restent au constructeur.
