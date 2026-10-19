# Bandeaux, glisser-déposer et menus d'onglet — plan d'implémentation (plan 9)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Réorganiser panneaux et onglets à la souris « comme Terminator » : bandeau de titre par panneau, glisser-déposer des panneaux et des onglets, menus contextuels d'onglet et de bandeau.

**Architecture:** Les opérations d'arbre (insérer une feuille d'un côté, à la racine, division parente) vont dans `rustty-layout`, pur et testé par propriétés. `Tab`/`Workspace` déplacent des `TermId` sans toucher aux PTY. Le binaire ajoute des modules purs (géométrie du bandeau, machine de glisser, menus paramétrés) testés unitairement, puis un câblage mince dans `input.rs`/`relayout.rs`/`render*.rs`. Le dessin du bandeau vit dans `rustty-render` avec une image de référence.

**Tech Stack:** Rust 1.96 (CI : clippy 1.99), wgpu 30, winit, proptest.

**Spec:** `docs/superpowers/specs/2026-10-19-terminator-design.md`

## Global Constraints

- TDD : chaque test est écrit et vu échouer avant le code.
- Code en anglais ; commentaires, messages et commits en français ; libellés d'interface en français.
- Chaque commit = une version : alpha.121 pour la tâche 1, puis la version libre suivante ; `Cargo.toml` racine (`version = "0.1.0-alpha.N"`) + `Cargo.lock` + entrée `CHANGELOG.md` `## 0.1.0-alpha.N — AAAA-MM-JJ · « titre »` dans le même commit.
- Avant chaque commit, depuis la racine : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets` (zéro avertissement ; penser au clippy 1.99 de la CI : `as_chunks` plutôt que `chunks_exact` à taille constante), `cargo test --workspace`.
- `git add` ciblé, jamais `-A`. Aucune mention d'assistant dans les commits.
- Fichiers d'environ 300 lignes au plus. Déjà à la limite : `input.rs`, `window_state.rs`, `model.rs`, `rustty-layout/src/node.rs`, `rustty-render/src/chrome/mod.rs` → n'y ajouter que des appels ou des déclarations ; la logique va dans les nouveaux modules nommés ci-dessous. Tests séparés en `*_tests.rs` (motif `#[cfg(test)] #[path = "x_tests.rs"] mod tests;`).
- Un déplacement ne tue, ne relance ni ne vide aucun shell : seuls les arbres et les tables `WindowId → TermId` changent.
- Seuil de glisser : 4 px. Délai de survol d'onglet : 500 ms. Hauteur du bandeau : `metrics.height` des métriques de base (`OsWindow::metrics()`), une rangée.
- Bandeau visible ssi l'onglet affiche au moins deux panneaux et qu'aucun n'est agrandi (zoom).
- Linux, macOS, Windows.

## Review Focus

1. Panneau à police zoomée (ctrl+molette) sous un bandeau : le bandeau garde la hauteur de base et le clic juste sous le bandeau tombe en rangée 0 du terminal → test `cell_under_header_is_row_zero_for_zoomed_font` (tâche 5).
2. Application qui capte la souris (vim, htop) : un clic sur le bandeau n'est jamais transmis ; un clic en première rangée du contenu est rapporté en rangée 1 (protocole 1-indexé) → test `header_press_is_not_reported_to_the_term` (tâche 6).
3. Shell qui se termine pendant qu'on tire son panneau, ou cible qui disparaît → le glisser s'annule sans panique → test `drag_of_a_vanished_term_is_cancelled` (tâche 9).
4. Glisser tenté pendant une confirmation de fermeture ou un renommage → refusé → test `drag_refused_while_confirming_or_renaming` (tâche 9).
5. Panneau plus petit que le bandeau (hauteur < 2 rangées) → contenu de hauteur 0 sans panique, terminal au minimum 1×1 → test `tiny_pane_keeps_a_one_row_grid` (tâche 5).

## Décisions de structure (s'appliquent à toutes les tâches)

- Un panneau déplacé reçoit un **nouveau** `WindowId` dans l'arbre d'arrivée (les ids sont propres à chaque `TabLayout`) ; on retire la feuille d'origine avec `TabLayout::close`, puis on insère. Les ids ne sont jamais réutilisés (`next_id` croissant), donc une couleur de division orpheline dans `Tab::split_accents` ne peut jamais repeindre une autre division : pas de nettoyage (le point « couleurs orphelines » de la spec est sans effet visible).
- Le mot « banner » est déjà pris (`banner.rs`, bandeau de notification) : le bandeau de panneau s'appelle **`PaneHeader`** dans le code.
- `OsWindow.pane_rects` contient désormais le rectangle **du contenu** (bandeau retiré) : tous les consommateurs actuels (taille de grille, clics, sélection, dessin) restent justes sans modification. Le rectangle complet d'un panneau = `header.rect` ∪ contenu.

---

### Task 1: Insertion latérale et division parente dans `rustty-layout`

**Files:**
- Modify: `crates/rustty-layout/src/geometry.rs` (ajout de `Side`)
- Create: `crates/rustty-layout/src/insert.rs` (`impl Node` : insertion, division parente)
- Modify: `crates/rustty-layout/src/lib.rs`, `crates/rustty-layout/src/tab_layout.rs` (trois méthodes publiques, ≤ 30 lignes ajoutées hors tests)
- Test: `crates/rustty-layout/src/insert.rs` (unitaires) et le module de propriétés existant de `tab_layout.rs`

**Interfaces:**
- Produces:
  - `pub enum Side { Top, Bottom, Left, Right }` (réexporté à la racine comme `Axis`), avec `pub const fn axis(self) -> Axis` (Top/Bottom → `Horizontal`, Left/Right → `Vertical`) et `pub const fn new_is_first(self) -> bool` (Top/Left).
  - `TabLayout::insert(&mut self, target: WindowId, side: Side) -> Option<WindowId>` : nouvelle feuille du côté `side` de `target`, ratio 0.5, focalisée, zoom levé ; la division porte `SplitId(new.0)` (même contrat que `split`). `None` si `target` est absente.
  - `TabLayout::insert_root(&mut self, side: Side) -> WindowId` : enveloppe toute la racine (ou crée la racine si l'arbre est vide), mêmes règles.
  - `TabLayout::parent_split(&self, id: WindowId) -> Option<SplitId>` : division qui contient directement la feuille ; `None` pour une feuille racine ou absente.

- [ ] **Step 1: Tests qui échouent**

```rust
#[test]
fn insert_on_each_side_places_the_new_window() {
    for (side, new_first) in [(Side::Top, true), (Side::Left, true), (Side::Bottom, false), (Side::Right, false)] {
        let (mut l, a) = TabLayout::new();
        let b = l.insert(a, side).unwrap();
        assert_eq!(l.windows(), if new_first { vec![b, a] } else { vec![a, b] });
        assert_eq!(l.focused(), Some(b));
        assert_eq!(l.split_axis(SplitId(b.0)), Some(side.axis()));
        assert_eq!(l.parent_split(a), Some(SplitId(b.0)));
    }
}

#[test]
fn insert_root_wraps_the_whole_tree() {
    let (mut l, a) = TabLayout::new();
    let b = l.split(a, Axis::Horizontal).unwrap();
    let c = l.insert_root(Side::Right);
    assert_eq!(l.windows(), vec![a, b, c]);
    assert_eq!(l.parent_split(c), Some(SplitId(c.0)));
    assert_eq!(l.parent_split(a), Some(SplitId(b.0)));
}

#[test]
fn insert_on_missing_target_is_refused_and_root_has_no_parent() {
    let (mut l, a) = TabLayout::new();
    assert_eq!(l.insert(WindowId(99), Side::Left), None);
    assert_eq!(l.parent_split(a), None);
}
```

Plus une propriété dans le module proptest existant : après toute suite d'opérations incluant `insert(target aléatoire, side aléatoire)`, `close` puis `insert_root`, les invariants déjà vérifiés (rectangles disjoints couvrant les bornes, feuilles uniques, focus contenu) tiennent toujours.

- [ ] **Step 2:** `cargo test -p rustty-layout` → échec de compilation (`Side`, `insert` inconnus).
- [ ] **Step 3:** Implémenter `Node::insert_leaf(&mut self, target, side, new_id) -> bool` et `Node::parent_split(&self, id) -> Option<SplitId>` dans `insert.rs` ; `split_leaf` devient un appel à `insert_leaf` avec `Side::Right`/`Side::Bottom` selon l'axe (supprimer le doublon). `TabLayout` alloue l'id comme `split`.
- [ ] **Step 4:** `cargo test -p rustty-layout` → tout passe.
- [ ] **Step 5: Commit** — alpha.121 « Insertion d'un panneau d'un côté donné ».

---

### Task 2: Déplacer panneaux et onglets dans le modèle (`Tab`, `Workspace`)

**Files:**
- Modify: `crates/rustty/src/tab.rs` (insertion d'un terme existant, division parente, couleur choisie)
- Create: `crates/rustty/src/workspace_arrange.rs` (module enfant de `workspace.rs` : `#[path = "workspace_arrange.rs"] mod arrange;` pour accéder aux champs privés)
- Create: `crates/rustty/src/workspace_arrange_tests.rs`
- Modify: `crates/rustty/src/workspace.rs` (champ `pane_names`, nettoyage des noms à la fermeture)

**Interfaces:**
- Consumes: `Side`, `TabLayout::{insert, insert_root, parent_split}` (tâche 1).
- Produces:
  - `Tab::insert_term(&mut self, term: TermId, target: TermId, side: Side, accent: usize) -> bool`
  - `Tab::insert_term_at_root(&mut self, term: TermId, side: Side, accent: usize)`
  - `Tab::parent_split_of(&self, term: TermId) -> Option<SplitId>`
  - `pub color: Option<usize>` sur `Tab` (index ANSI choisi par le nuancier ; `None` = couleur de la configuration).
  - `Workspace::move_term_to_pane(&mut self, term: TermId, target: TermId, side: Side) -> bool`
  - `Workspace::move_term_to_tab(&mut self, term: TermId, tab: usize) -> bool` (à droite de tout l'onglet ; permis vers son propre onglet)
  - `Workspace::move_term_to_new_tab(&mut self, term: TermId) -> bool` (nouvel onglet juste après l'onglet d'origine)
  - `Workspace::move_tab(&mut self, from: usize, to: usize) -> bool` (`to` = index d'insertion dans `0..=len`, compté avant retrait ; l'onglet actif reste le même onglet)
  - `Workspace::close_other_tabs(&mut self, keep: usize) -> Vec<TermId>` (termes à tuer)
  - `Workspace::set_tab_color(&mut self, tab: usize, color: Option<usize>)`
  - `Workspace::rename_pane(&mut self, term: TermId, name: Option<String>)`, `Workspace::pane_name(&self, term: TermId) -> Option<&str>`
  - Toute réussite de `move_term_*` focalise le terme déplacé et active son onglet d'arrivée ; un onglet d'origine vidé est retiré et les index sont recalculés.

- [ ] **Step 1: Tests qui échouent** (dans `workspace_arrange_tests.rs`), au minimum :

```rust
#[test]
fn move_within_a_tab_keeps_every_term() {
    // onglet [t1 | t2] ; t1 déplacé sous t2
    let (mut ws, t1) = Workspace::new();
    let t2 = ws.split_focused(SplitAxis::Vertical).unwrap();
    assert!(ws.move_term_to_pane(t1, t2, Side::Bottom));
    assert_eq!(ws.active_tab().terms(), vec![t2, t1]);
    assert_eq!(ws.focused_term(), Some(t1));
}

#[test]
fn move_to_another_tab_activates_it() { /* t2 de l'onglet 0 vers l'onglet 1 : onglet 0 garde t1, onglet 1 = [t3, t2], actif = 1 */ }

#[test]
fn emptied_source_tab_is_removed_and_indices_follow() { /* onglets [a],[b,c] : move a sur c → 1 onglet, actif 0 */ }

#[test]
fn move_onto_itself_is_refused() { assert!(!ws.move_term_to_pane(t1, t1, Side::Left)); }

#[test]
fn move_to_own_tab_goes_to_the_right_edge() { /* [t1 / t2] (horizontal) ; move_term_to_tab(t1, 0) → terms() == [t2, t1], division racine verticale */ }

#[test]
fn move_to_new_tab_inserts_after_the_source() { /* onglets [a,b],[c] ; move b → [a],[b],[c], actif 1 */ }

#[test]
fn move_tab_reorders_and_active_follows() {
    // [A,B,C] actif B : move_tab(0, 3) → [B,C,A] actif reste B (index 0) ; move_tab(2, 0) → [A,B,C]
}

#[test]
fn close_other_tabs_returns_their_terms() { /* [a],[b,c],[d] keep 1 → rend {a,d}, un seul onglet, actif 0 */ }

#[test]
fn pane_name_follows_the_term_and_is_dropped_on_close() { /* rename_pane(t2, Some("logs")) ; move t2 vers autre onglet → pane_name(t2) == Some("logs") ; close_term(t2) → None ; rename_pane(t, Some("")) → None */ }
```

- [ ] **Step 2:** `cargo test -p rustty workspace_arrange` → échec de compilation.
- [ ] **Step 3:** Implémenter. Le retrait d'origine passe par `Tab::close_term` (pur, ne tue rien) ; l'accent de la nouvelle division vient de `self.picker.next()`.
- [ ] **Step 4:** `cargo test -p rustty` → tout passe.
- [ ] **Step 5: Commit** — alpha.122 « Déplacer panneaux et onglets dans le modèle ».

---

### Task 3: Couleurs — onglet choisi au nuancier, bandeau de la couleur de sa division

**Files:**
- Modify: `crates/rustty/src/appearance.rs` (+ ses tests), `crates/rustty/src/accent.rs` (noms des couleurs), l'appelant de `tab_accents` (`render_frame.rs`/`relayout.rs` selon l'usage actuel)

**Interfaces:**
- Consumes: `Tab::color`, `Tab::parent_split_of` (tâche 2).
- Produces:
  - `tab_accents(tabs_config: &Tabs, palette: &Palette, tabs: &[Tab]) -> Vec<Option<Rgba>>` : par onglet, `tab.color` s'il est choisi (même si `random = false`), sinon l'accent tiré si `random`, sinon `None` (style de la configuration).
  - `header_color(splits: &Splits, palette: &Palette, tab: &Tab, term: TermId) -> Rgba` : `divider_color` de `tab.parent_split_of(term)` ; sans division parente, `splits.color` ou `ansi[8]`.
  - `pub const ACCENT_NAMES: [&str; 12]` aligné sur `ACCENT_SLOTS` : `["Rouge", "Vert", "Jaune", "Bleu", "Magenta", "Cyan", "Rouge vif", "Vert vif", "Jaune vif", "Bleu vif", "Magenta vif", "Cyan vif"]`.

- [ ] **Step 1: Tests qui échouent** : `chosen_color_shows_even_without_random`, `default_color_follows_config` (`color = None`, `random = false` → `None`), `header_takes_its_parent_split_color` (deux divisions aux accents différents : chaque panneau prend celle de son parent ; `random_colors = false` → `splits.color`), `accent_names_match_slots` (`ACCENT_NAMES.len() == ACCENT_SLOTS.len()`).
- [ ] **Step 2:** `cargo test -p rustty appearance` → échec.
- [ ] **Step 3:** Implémenter ; adapter l'appelant de `tab_accents` (le `TabSpec.accent` reçoit directement l'`Option<Rgba>` de l'onglet).
- [ ] **Step 4:** `cargo test -p rustty` → passe.
- [ ] **Step 5: Commit** — alpha.123 « Couleur d'onglet choisie et couleur de bandeau ».

---

### Task 4: Dessin du bandeau dans `rustty-render`

**Files:**
- Create: `crates/rustty-render/src/chrome/pane_header.rs`
- Modify: `crates/rustty-render/src/chrome/mod.rs` (une ligne `mod` + réexport), `crates/rustty-render/src/lib.rs` (réexport)
- Test: `crates/rustty-render/tests/` (nouveau test hors écran) + image `tests/golden/pane_header.png`

**Interfaces:**
- Produces:
  - `pub struct PaneHeaderSpec<'a> { pub rect: PixelRect, pub title: &'a str, pub color: Rgba, pub focused: bool, pub show_close: bool, pub close_hovered: bool }`
  - `pub fn pane_header_close_rect(rect: PixelRect, metrics: CellMetrics) -> PixelRect` (bouton ✕ collé au bord droit, même largeur en cellules que celui des onglets)
  - `pub fn pane_header_chrome(headers: &[PaneHeaderSpec], style: &TabBarStyle, metrics: CellMetrics) -> Chrome` : fond `color`, titre tronqué avec « … » (réutiliser `title_fit`), texte lisible via `readable_on(color, style.light_text, style.dark_text)`, ✕ aux couleurs `close_*` du style seulement si `show_close`. Distinction visuelle du panneau focalisé laissée au constructeur (par ex. fond assombri pour les autres).

- [ ] **Step 1: Tests qui échouent** : unitaires `close_rect_is_inside_and_right_aligned`, `long_title_is_truncated_before_the_close_button` (aucun `ChromeText` ne déborde sur le rectangle du ✕), `no_close_quad_unless_show_close` ; test GPU hors écran `pane_header_golden` qui dessine deux bandeaux (un focalisé avec ✕ survolé, un non focalisé au titre long) et compare à `pane_header.png` comme les tests de `tab_bar.png` (générer l'image de référence au premier passage selon le mécanisme existant des tests de référence, puis l'inspecter visuellement).
- [ ] **Step 2:** `cargo test -p rustty-render pane_header` → échec.
- [ ] **Step 3:** Implémenter.
- [ ] **Step 4:** `cargo test -p rustty-render` → passe (y compris référence).
- [ ] **Step 5: Commit** — alpha.124 « Dessin du bandeau de panneau ».

---

### Task 5: Bandeaux dans la fenêtre — géométrie, mise en page, dessin, titres

**Files:**
- Create: `crates/rustty/src/pane_header.rs` (géométrie pure + tests)
- Modify: `crates/rustty/src/relayout.rs` (calcul des bandeaux, `pane_rects` = contenu), `crates/rustty/src/window_state.rs` (champ `headers`, résolution des titres), `crates/rustty/src/render.rs`/`render_frame.rs` (dessin des bandeaux dans la passe de chrome)

**Interfaces:**
- Consumes: `header_color` (tâche 3), `pane_header_chrome`, `pane_header_close_rect` (tâche 4), `Workspace::pane_name` (tâche 2).
- Produces:
  - `pub struct PaneHeader { pub term: TermId, pub rect: PixelRect, pub close: PixelRect }`
  - `pub fn headers_visible(tab: &Tab) -> bool`
  - `pub fn split_header(pane: PixelRect, height: u32) -> (PixelRect, PixelRect)` (bandeau, contenu ; hauteur saturée : un panneau plus bas que le bandeau donne un contenu de hauteur 0)
  - `pub enum HeaderHit { Title(TermId), Close(TermId) }`, `pub fn header_at(headers: &[PaneHeader], x: f64, y: f64) -> Option<HeaderHit>` (le ✕ prime)
  - `OsWindow.headers: Vec<PaneHeader>` (vide quand `!headers_visible`)
  - `OsWindow::pane_title(&self, term: TermId) -> &str` : `pane_name` sinon titre du shell sinon `""` ; utilisé par `tab_titles` (titre d'onglet du panneau focalisé, sauf nom d'onglet) et par les bandeaux.
  - `OsWindow::full_pane_rect(&self, term: TermId) -> Option<PixelRect>` (bandeau ∪ contenu, pour les cibles de dépôt).

- [ ] **Step 1: Tests qui échouent** (dans `pane_header.rs`) : `headers_need_two_visible_panes` (1 panneau → faux ; 2 → vrai ; 2 avec zoom → faux), `split_header_takes_one_row_from_the_top`, `tiny_pane_keeps_a_one_row_grid` (panneau de hauteur `height - 1` → contenu de hauteur 0, et `geometry::grid_size` sur ce contenu rend au moins `(1, 1)`), `close_wins_over_title_in_header_at`, `cell_under_header_is_row_zero_for_zoomed_font` (contenu issu de `split_header` + métriques d'une police plus grande : `cell_at` d'un point situé 1 px sous le bandeau + padding rend la rangée 0).
- [ ] **Step 2:** `cargo test -p rustty pane_header` → échec.
- [ ] **Step 3:** Implémenter la géométrie, puis le câblage : dans `relayout`, pour l'onglet actif, si `headers_visible`, découper chaque rectangle avec `split_header(rect, self.metrics().height)`, ranger le bandeau dans `headers` et le contenu dans `pane_rects` avant le redimensionnement des terminaux ; dessiner les bandeaux dans la passe de chrome existante : `show_close` vrai seulement pour le bandeau survolé par la souris, `close_hovered` quand elle est sur le ✕ ; un mouvement de souris qui change ce survol demande un nouveau dessin.
- [ ] **Step 4:** `cargo test --workspace` → passe ; lancer `cargo run -p rustty`, diviser, vérifier à l'œil bandeaux, couleurs et titres (noter le résultat dans le rapport).
- [ ] **Step 5: Commit** — alpha.125 « Bandeau de titre sur chaque panneau d'un onglet divisé ».

---

### Task 6: Interactions du bandeau — focus, ✕, double clic pour renommer le panneau

**Files:**
- Create: `crates/rustty/src/header_input.rs` (routage souris du bandeau)
- Modify: `crates/rustty/src/rename.rs` (cible du renommage), `crates/rustty/src/model/close.rs` (démarrage/validation selon la cible), `crates/rustty/src/input.rs` (un appel, avant le test des barres de split), le dessin du bandeau (tampon + « ▌ » pendant l'édition, comme les onglets)

**Interfaces:**
- Consumes: `header_at`, `HeaderHit` (tâche 5), `Workspace::rename_pane` (tâche 2).
- Produces:
  - `pub enum RenameTarget { Tab(usize), Pane(TermId) }` ; `Rename { pub target: RenameTarget, pub buffer: String }`, `Rename::new(target: RenameTarget, initial: &str)`.
  - `Model::start_pane_rename(&mut self, term: TermId) -> Vec<Effect>` (pré-rempli avec le nom actuel du panneau) ; `rename_key` valide vers `workspace.rename` ou `workspace.rename_pane` selon la cible.
  - `OsWindow::header_press(&mut self, button: MouseButton, x: f64, y: f64, now: Instant) -> Option<Vec<Effect>>` : `None` hors bandeau ; gauche sur ✕ → `request_close(CloseRequest::Term(term), confirm && running)` ; gauche sur titre → focus du panneau (+ amorce de glisser, câblée en tâche 10) ; double clic gauche sur titre → `start_pane_rename` ; droit → menu de bandeau (tâche 8). Jamais transmis au terminal.

- [ ] **Step 1: Tests qui échouent** : dans `rename.rs`, `pane_rename_commits_to_its_target` ; dans les tests du modèle, `pane_rename_sets_the_pane_name` et `empty_pane_name_restores_the_shell_title` ; dans `header_input.rs` (fonction pure de décision extraite, par ex. `fn header_action(hit: HeaderHit, button: MouseButton, double: bool) -> HeaderAction`), `close_button_closes_title_focuses_double_click_renames` ; `header_press_is_not_reported_to_the_term` (une pression sur le bandeau rend `Some(..)` sans `Effect` d'écriture vers le PTY, et la décision d'encodage souris n'est pas appelée — vérifier par la fonction de décision).
- [ ] **Step 2:** `cargo test -p rustty rename header` → échec.
- [ ] **Step 3:** Implémenter ; adapter les usages existants de `Rename { tab, .. }` (renommage d'onglet inchangé pour l'utilisateur).
- [ ] **Step 4:** `cargo test --workspace` → passe.
- [ ] **Step 5: Commit** — alpha.126 « Bandeau cliquable : focus, fermeture, renommage du panneau ».

---

### Task 7: Menus contextuels paramétrés (cible + entrées) et nuancier

**Files:**
- Modify: `crates/rustty/src/context_menu/mod.rs` (cible, entrées par cible), `context_menu/layout.rs`, `context_menu/chrome.rs` (pastille de couleur), `context_menu/layout_tests.rs`
- Create: `crates/rustty/src/context_menu/entries.rs` (listes d'entrées et libellés, pour garder `mod.rs` sous 300 lignes)
- Modify: `crates/rustty/src/menu_input.rs` (utiliser `menu.entries`)

**Interfaces:**
- Produces:
  - `pub enum MenuTarget { Pane(TermId), Tab(usize), Header(TermId), Palette(usize) }`
  - Nouvelles entrées de `MenuItem` : `CloseTab`, `CloseOtherTabs`, `ChangeColor`, `MoveToNewTab`, `RenamePane`, `Color(usize)` (index ANSI), `DefaultColor`. `MenuItem::action(self) -> Option<Action>` (les nouvelles rendent `None`). `MenuItem::swatch(self) -> Option<usize>`.
  - `pub fn entries(target: &MenuTarget) -> Vec<MenuEntry>` :
    - Pane : liste actuelle inchangée.
    - Tab : `RenameTab`, `ChangeColor`, `NewTab`, séparateur, `CloseTab`, `CloseOtherTabs` — libellés « Renommer », « Changer la couleur », « Nouvel onglet », « Fermer l'onglet », « Fermer les autres onglets ».
    - Header : `SplitHorizontal`, `SplitVertical`, `ToggleZoom`, séparateur, `MoveToNewTab`, `RenamePane`, séparateur, `ClosePane` — libellés « Diviser horizontalement », « Diviser verticalement », « Agrandir le panneau », « Déplacer dans un nouvel onglet », « Renommer le panneau », « Fermer le panneau ».
    - Palette : les 12 `Color(ACCENT_SLOTS[i])` avec `ACCENT_NAMES[i]`, séparateur, `DefaultColor` « Couleur par défaut ».
  - `ContextMenu { pub target: MenuTarget, pub entries: Vec<MenuEntry>, .. }` remplace le champ `term` et la constante `ENTRIES` ; constructeurs `ContextMenu::pane(term, x, y, can_copy, zoomed, keys)` (l'actuel `new`), `ContextMenu::tab(index, x, y, can_close_others: bool, keys)`, `ContextMenu::header(term, x, y, keys)`, `ContextMenu::palette(tab, x, y)`. `CloseOtherTabs` désactivé quand `!can_close_others` (un seul onglet).
  - `menu_chrome` dessine une pastille carrée de `accent_color(palette, slot)` avant le libellé des entrées `swatch()`.

- [ ] **Step 1: Tests qui échouent** : `pane_menu_is_unchanged` (adapter le test existant « 8 entrées »), `tab_menu_lists_its_five_items`, `header_menu_lists_its_six_items`, `palette_lists_twelve_colors_and_default`, `close_others_disabled_with_a_single_tab`, `swatch_only_on_colors` ; dans `layout_tests.rs`, le placement/hit-test fonctionne avec une liste de 14 entrées (nuancier).
- [ ] **Step 2:** `cargo test -p rustty context_menu` → échec.
- [ ] **Step 3:** Implémenter ; le menu de panneau existant se comporte exactement comme avant.
- [ ] **Step 4:** `cargo test --workspace` → passe.
- [ ] **Step 5: Commit** — alpha.127 « Menus contextuels paramétrés et nuancier ».

---

### Task 8: Ouvrir et exécuter les menus d'onglet, de bandeau et le nuancier

**Files:**
- Create: `crates/rustty/src/model/menu.rs` (exécution des entrées selon la cible) + tests dans `model_tests.rs` ou `model/menu_tests.rs`
- Modify: `crates/rustty/src/model.rs` (`CloseRequest::OtherTabs(usize)`, une ligne `mod`), `crates/rustty/src/model/close.rs` (exécution de `OtherTabs`), `crates/rustty/src/menu_input.rs` (ouverture sur onglet/bandeau, `ChangeColor` remplace le menu par le nuancier au même point, exécution via `Model::menu_command`), `crates/rustty/src/input.rs` / `header_input.rs` (clic droit sur un onglet et sur un bandeau)

**Interfaces:**
- Consumes: tâches 2, 6, 7.
- Produces:
  - `CloseRequest::OtherTabs(usize)` : confirmation si un programme tourne dans l'un des onglets fermés (comme `Tab`).
  - `Model::menu_command(&mut self, target: MenuTarget, item: MenuItem, confirm_close: bool, running: &dyn Fn(TermId) -> bool) -> Vec<Effect>` :
    - `Pane(term)` : comportement actuel (focus puis `apply(item.action())`).
    - `Tab(i)` : `RenameTab` → `start_rename(i)` ; `NewTab` → active `i` puis `Action::NewTab` (le nouvel onglet suit l'onglet cliqué) ; `CloseTab` → `request_close(Tab(i))` ; `CloseOtherTabs` → `request_close(OtherTabs(i))`.
    - `Header(term)` : focus de `term`, puis splits/zoom via `apply` (les divisions héritent du dossier, alpha.120), `MoveToNewTab` → `workspace.move_term_to_new_tab`, `RenamePane` → `start_pane_rename`, `ClosePane` → `request_close(Term(term))`.
    - `Palette(i)` : `Color(c)` → `set_tab_color(i, Some(c))`, `DefaultColor` → `set_tab_color(i, None)` ; rend `[Relayout]`.
  - Clic droit sur un onglet de la barre → `ContextMenu::tab` ; sur un bandeau → `ContextMenu::header` ; refusé pendant confirmation ou renommage (comme le menu actuel).

- [ ] **Step 1: Tests qui échouent** (modèle) : `tab_menu_targets_the_clicked_tab_not_the_active_one` (renommer l'onglet 2 alors que l'actif est 0), `new_tab_from_tab_menu_follows_the_clicked_tab`, `close_other_tabs_asks_when_something_runs` et `..._closes_directly_otherwise`, `header_split_targets_its_pane` (le `SpawnTerm` porte `cwd_of: Some(term)` du bandeau, pas du panneau focalisé avant), `header_move_to_new_tab_creates_a_tab_after`, `palette_sets_and_resets_the_tab_color`.
- [ ] **Step 2:** `cargo test -p rustty model` → échec.
- [ ] **Step 3:** Implémenter ; puis brancher l'ouverture (clic droit) et l'exécution dans `menu_input.rs`.
- [ ] **Step 4:** `cargo test --workspace` → passe ; `cargo run -p rustty` : ouvrir les trois menus et le nuancier, vérifier à l'œil (noter dans le rapport).
- [ ] **Step 5: Commit** — alpha.128 « Menus d'onglet et de bandeau ».

---

### Task 9: Machine de glisser pure (panneaux et onglets)

**Files:**
- Create: `crates/rustty/src/drag.rs`, `crates/rustty/src/drag_tests.rs`
- Modify: `crates/rustty/src/main.rs` (une ligne `mod`), `crates/rustty/src/model.rs` + `crates/rustty/src/model/arrange.rs` (exécution des dépôts)

**Interfaces:**
- Consumes: `Side` (tâche 1), `Workspace::move_*` (tâche 2), `PaneHeader` (tâche 5), `TabRect` (`rustty_render`).
- Produces (dans `drag.rs`) :
  - `pub const DRAG_THRESHOLD: f64 = 4.0;`, `pub const TAB_HOVER_DELAY: Duration = Duration::from_millis(500);`
  - `pub enum DragSource { Pane(TermId), Tab(usize) }`
  - `pub enum DropTarget { None, Pane { term: TermId, side: Side }, OnTab(usize), TabSlot(usize) }`
  - `pub fn drop_side(rect: PixelRect, x: f64, y: f64) -> Side` (zones délimitées par les deux diagonales du rectangle)
  - `pub fn tab_slot(tabs: &[TabRect], x: f64) -> usize` (index d'insertion d'après le milieu de chaque onglet)
  - `pub struct Drag` avec : `Drag::press(source: DragSource, start_tab: usize, x: f64, y: f64) -> Self` (état « amorcé ») ; `fn moved(&mut self, x: f64, y: f64) -> bool` (vrai quand le seuil vient d'être franchi) ; `fn is_active(&self) -> bool` ; `fn set_target(&mut self, target: DropTarget, now: Instant)` (un `OnTab(i)` démarre le compteur de survol, toute autre cible le remet à zéro) ; `fn due_tab_switch(&self, now: Instant) -> Option<usize>` (onglet à afficher quand le survol a duré `TAB_HOVER_DELAY` ; une seule fois par survol) ; `fn deadline(&self) -> Option<Instant>` (pour `about_to_wait`) ; `fn finish(self) -> Option<(DragSource, DropTarget)>` (`None` si jamais activé) ; `fn start_tab(&self) -> usize`.
  - `pub enum PaneDrop { Pane { target: TermId, side: Side }, Tab(usize), NewTab }`
  - `Model::move_pane(&mut self, term: TermId, drop: PaneDrop) -> Vec<Effect>` et `Model::move_tab(&mut self, from: usize, to: usize) -> Vec<Effect>` (`[Relayout]` en cas de changement, sinon rien ; un renommage en cours est abandonné).
  - `Model::accepts_drag(&self) -> bool` (ni confirmation ni renommage en cours).

- [ ] **Step 1: Tests qui échouent** (`drag_tests.rs` et tests du modèle) :

```rust
#[test]
fn drop_side_follows_the_diagonals() {
    let r = PixelRect { x: 0, y: 0, width: 200, height: 100 };
    assert_eq!(drop_side(r, 100.0, 10.0), Side::Top);
    assert_eq!(drop_side(r, 100.0, 90.0), Side::Bottom);
    assert_eq!(drop_side(r, 10.0, 50.0), Side::Left);
    assert_eq!(drop_side(r, 190.0, 50.0), Side::Right);
}

#[test]
fn small_moves_stay_a_click() {
    let mut d = Drag::press(DragSource::Tab(0), 0, 10.0, 10.0);
    assert!(!d.moved(13.0, 10.0));
    assert!(!d.is_active());
    assert!(d.moved(15.0, 10.0));
    assert_eq!(Drag::press(DragSource::Tab(0), 0, 0.0, 0.0).finish(), None);
}

#[test]
fn hovering_a_tab_half_a_second_switches_once() {
    let t0 = Instant::now();
    let mut d = Drag::press(DragSource::Pane(TermId(1)), 0, 0.0, 0.0);
    d.moved(50.0, 0.0);
    d.set_target(DropTarget::OnTab(2), t0);
    assert_eq!(d.due_tab_switch(t0 + Duration::from_millis(499)), None);
    assert_eq!(d.due_tab_switch(t0 + TAB_HOVER_DELAY), Some(2));
    assert_eq!(d.deadline(), Some(t0 + TAB_HOVER_DELAY));
}

#[test]
fn tab_slot_uses_tab_midpoints() { /* 3 onglets de 100 px : x=40 → 0, x=60 → 1, x=290 → 3 */ }
```

Modèle : `pane_drop_moves_without_spawning` (aucun `SpawnTerm`/`CloseTerm` dans les effets), `drop_on_itself_changes_nothing`, `drag_refused_while_confirming_or_renaming` (`accepts_drag` faux), `drag_of_a_vanished_term_is_cancelled` (`move_pane` d'un terme absent → aucun effet, aucune panique), `move_tab_effects`.
- [ ] **Step 2:** `cargo test -p rustty drag` → échec.
- [ ] **Step 3:** Implémenter.
- [ ] **Step 4:** `cargo test -p rustty` → passe.
- [ ] **Step 5: Commit** — alpha.129 « Machine de glisser-déposer ».

---

### Task 10: Glisser-déposer dans la fenêtre — câblage, aperçu, minuterie, Échap

**Files:**
- Create: `crates/rustty/src/drag_input.rs` (pression/mouvement/relâchement/Échap pendant un glisser, choix de la cible), `crates/rustty/src/drag_preview.rs` (quads d'aperçu, pur + tests)
- Modify: `crates/rustty/src/window_state.rs` (champ `drag: Option<Drag>` à côté du `drag` de barre existant — renommer ce dernier en `divider_drag` pour lever l'ambiguïté), `crates/rustty/src/input.rs` (appels), `crates/rustty/src/app.rs` (`about_to_wait` : échéance de survol), le rendu (aperçu dessiné au-dessus des panneaux, sous le menu), `crates/rustty/src/relayout.rs` (annuler un glisser dont la source a disparu)

**Interfaces:**
- Consumes: tâche 9, `OsWindow::full_pane_rect`, `headers`, `tab_bar` (`TabBarLayout`).
- Produces:
  - Pression gauche sur un titre de bandeau → `Drag::press(Pane(term))` ; sur un onglet → `Drag::press(Tab(i))` (le clic simple garde son effet actuel au relâchement s'il n'y a pas eu de glisser).
  - Pendant un glisser actif : cible = onglet survolé (`OnTab` pour un panneau, `TabSlot` pour un onglet), sinon panneau sous le curseur avec `drop_side` sur son rectangle complet (pas de cible sur le panneau tiré lui-même) ; curseur `CursorIcon::Grabbing`.
  - `about_to_wait` : si `drag.deadline()` existe, `ControlFlow::WaitUntil` au plus tôt de cette échéance ; au réveil, `due_tab_switch` → `go_to_tab` + relayout.
  - Relâchement : `Model::move_pane` / `Model::move_tab` selon la source et la cible ; cible `None` ou Échap → aucun changement d'arbre et retour à `start_tab` si un survol a changé d'onglet.
  - `pub fn drop_preview(target: &DropTarget, rects: ...) -> Vec<ChromeQuad>` : moitié éclairée (translucide) pour `Pane`, contour de l'onglet pour `OnTab`, trait vertical d'insertion pour `TabSlot` ; apparence laissée au constructeur.

- [ ] **Step 1: Tests qui échouent** (`drag_preview.rs`) : `pane_preview_covers_the_chosen_half` (côté `Left` → quad de la moitié gauche du rectangle complet), `tab_slot_preview_is_a_vertical_bar_between_tabs`, `no_preview_without_target` ; (`drag_input.rs`, logique extraite en fonction pure `fn pick_target(source, x, y, headers, panes, tab_bar) -> DropTarget`) `pane_is_not_a_target_for_itself`, `tab_bar_wins_over_panes`, `tab_drag_targets_slots_only`.
- [ ] **Step 2:** `cargo test -p rustty drag_` → échec.
- [ ] **Step 3:** Implémenter et câbler.
- [ ] **Step 4:** `cargo test --workspace` → passe ; `cargo run -p rustty` : tirer un panneau sur les quatre moitiés d'un autre, vers un autre onglet par survol, sur un onglet, réordonner les onglets, Échap en plein glisser ; noter le résultat dans le rapport.
- [ ] **Step 5: Commit** — alpha.130 « Glisser-déposer des panneaux et des onglets ».

---

### Task 11: Documentation et parcours manuel

**Files:**
- Modify: `README.md` (fonctionnalités : bandeaux, glisser-déposer, menus d'onglet et de bandeau, nuancier ; phrase sur le clic droit), `docs/e2e.md` (nouvelle section « Bandeaux et glisser-déposer » : chaque comportement de la spec en une ligne vérifiable, dont les cinq points de la Review Focus)

- [ ] **Step 1:** Rédiger ; relire le README pour qu'aucune phrase existante ne contredise le nouveau comportement (menu au clic droit, titres d'onglet).
- [ ] **Step 2:** Suite complète depuis la racine : fmt, clippy, tests → propres.
- [ ] **Step 3: Commit** — alpha.131 « Documentation des bandeaux et du glisser-déposer ».
