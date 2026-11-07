# Hyperliens — jalon 1 « Liens déclarés » — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Les liens OSC 8 émis par les programmes (`ls --hyperlink`) sont mémorisés par cellule, soulignés au survol avec curseur main et infobulle de cible, et s'ouvrent dans le navigateur par défaut d'un ctrl+clic (ou clic simple selon `[links] click`).

**Architecture:** `rustty-vt` porte l'identifiant de lien dans chaque `Cell` (2 octets dans le bourrage actuel) et une table partagée `Arc<LinkTable>` déduplicant `(uri, id)`, bornée à 65 535 avec compactage. `rustty-render` reçoit dans `PaneFrame` la plage de cellules survolée et la souligne. Dans `rustty`, un module pur `links` calcule la plage contiguë d'un lien et décide du clic (`LinkClick`), `link_input.rs` relie survol, infobulle (extraite de `SpellPopup` en `tooltip.rs`), modificateurs et effets ; `Effect::OpenLink` lance l'ouvreur du bureau. `rustty-config` ajoute `[links]` (`click`, `schemes`).

**Tech Stack:** Rust stable, `vte` (parseur existant), `bitflags`, `winit`, `wgpu`, `toml`/`serde`. Aucune dépendance nouvelle.

**Spec:** `docs/superpowers/specs/2026-11-06-links-design.md` (§ 5.1, 5.3 à 5.7, § 6, jalon 1 du § 7). Le § 5.2 (URL en clair, `Term::logical_line`, `detect_urls`) est le jalon 2.

## Global Constraints

- Code en anglais, commentaires et messages de commit en français, TDD (test vu rouge avant le code), `cargo fmt --all`, `cargo clippy --workspace --all-targets` sans avertissement, `cargo test --workspace` depuis la racine avant chaque commit.
- Chaque commit = une version `0.1.0-alpha.N` (bump dans `Cargo.toml` racine + `cargo update -w`) et une entrée en tête de `CHANGELOG.md` au format `## 0.1.0-alpha.N — AAAA-MM-JJ · « titre »`, dans le même commit ; `git add` ciblé ; aucune mention d'assistant.
- `Cell` reste à 16 octets (`size_of::<Cell>() == 16`), `LinkId = NonZeroU16`.
- URI OSC 8 : ≤ 2 083 octets, UTF-8 valide, aucun octet `< 0x20` ni `0x7f` ; sinon la séquence vaut fermeture.
- Table : 65 535 liens, compactage quand pleine et à `ED 3` ; écran principal et alternatif partagent la table ; `RIS` la vide.
- `[links] click` ∈ {`"ctrl"` (défaut), `"plain"`} ; `schemes` défaut `["http", "https", "file", "mailto", "ftp", "ssh", "git"]`, non vide, chaque schéma `^[a-z][a-z0-9+.-]*$`, sans doublon.
- Infobulle après 400 ms de survol immobile du même lien, sous la ligne, cible tronquée au milieu à 60 cellules, suffixe ` · ctrl+clic` en mode ctrl, ` · shift+clic` / ` · shift+ctrl+clic` sous capture souris.
- Souris capturée (`MouseMode != None`) : survol et clic seulement avec shift.
- Pas de nouvelle dépendance ; pas de nouvelle action `[keys]`.

## Review Focus

1. Un programme émet `OSC 8` avec une URI de 3 000 octets puis du texte : le texte s'affiche sans lien, et le lien ouvert précédemment est fermé (pas de lien tronqué) — Task 2, `an_oversized_uri_closes_the_link`.
2. Écran alternatif (`vim` ouvre un lien, quitte) : revenir à l'écran principal garde les liens de l'historique, et les cellules de l'écran alternatif n'en polluent pas la table après compactage — Task 2, `alt_screen_shares_the_table_and_compaction_keeps_visible_links`.
3. Lien à cheval sur une ligne repliée, souris sur la seconde moitié : la plage soulignée couvre les deux lignes, et le clic ouvre l'URI entière — Task 5, `a_run_spans_wrapped_lines_from_either_half`.
4. Relâche après un glisser qui a sélectionné du texte sur le lien : copie, pas d'ouverture ; puis clic immobile : ouverture — Task 5, `a_drag_selects_and_a_still_click_opens`.
5. Modificateur relâché entre l'appui et la relâche (ctrl enfoncé à l'appui, lâché avant la relâche) : le lien s'ouvre quand même, et l'inverse (ctrl seulement à la relâche) n'ouvre pas — Task 5, `the_modifier_counts_at_press`.

---

### Task 1: `LinkId` dans la cellule et table des liens

**Files:**
- Create: `crates/rustty-vt/src/links.rs`
- Modify: `crates/rustty-vt/src/cell.rs` (`Cell`), `crates/rustty-vt/src/lib.rs` (exports)
- Test: `crates/rustty-vt/src/links.rs` (`mod tests`), `crates/rustty-vt/src/cell.rs` (`mod tests`)

**Interfaces:**
- Produces: `pub type LinkId = std::num::NonZeroU16;` ; `pub struct Link { pub uri: String, pub id_param: Option<String> }` ; `pub struct LinkTable` (`Default`, `Clone`, `Debug`, `PartialEq`, `Eq`) avec `pub fn insert(&mut self, uri: &str, id_param: Option<&str>) -> Option<LinkId>` (même paire → même id ; `None` quand pleine), `pub fn get(&self, id: LinkId) -> Option<&Link>`, `pub fn uri(&self, id: LinkId) -> Option<&str>`, `pub fn len(&self) -> usize`, `pub fn is_empty(&self) -> bool`, `pub const MAX_LINKS: usize = 65_535`, `pub fn retain_remapped(&self, used: impl Fn(LinkId) -> bool) -> (LinkTable, Vec<Option<LinkId>>)` (nouvelle table ne gardant que les ids `used`, et la correspondance ancien index → nouvel id) ; `Cell { pub c, pub style, pub link: Option<LinkId> }`, `Cell::new(c, style)` (lien `None`), `Cell::with_link(c, style, link: Option<LinkId>)`, `Cell::erased(style)` (lien `None`).

- [ ] **Step 1: Write the failing tests**

Dans `cell.rs` :

```rust
#[test]
fn cell_stays_compact_with_its_link() {
    assert_eq!(std::mem::size_of::<Cell>(), 16);
    let c = Cell::with_link('a', Style::default(), LinkId::new(7));
    assert_eq!(c.link, LinkId::new(7));
    assert_eq!(Cell::new('a', Style::default()).link, None);
    assert_eq!(Cell::erased(Style::default()).link, None);
}
```

Dans `links.rs` :

```rust
#[test]
fn same_uri_and_id_share_one_link() {
    let mut t = LinkTable::default();
    let a = t.insert("https://a.example", None).unwrap();
    let b = t.insert("https://a.example", None).unwrap();
    let c = t.insert("https://a.example", Some("x")).unwrap();
    let d = t.insert("https://b.example", None).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
    assert_eq!(t.uri(c), Some("https://a.example"));
    assert_eq!(t.get(c).unwrap().id_param.as_deref(), Some("x"));
    assert_eq!(t.len(), 3);
}

#[test]
fn the_table_is_bounded() {
    let mut t = LinkTable::default();
    for i in 0..LinkTable::MAX_LINKS {
        assert!(t.insert(&format!("u{i}"), None).is_some());
    }
    assert_eq!(t.insert("one-more", None), None, "pleine");
    assert!(t.insert("u0", None).is_some(), "un lien connu reste utilisable");
}

#[test]
fn retain_remapped_keeps_only_used_links_and_renumbers() {
    let mut t = LinkTable::default();
    let a = t.insert("a", None).unwrap();
    let b = t.insert("b", None).unwrap();
    let c = t.insert("c", None).unwrap();
    let (kept, map) = t.retain_remapped(|id| id == b || id == c);
    assert_eq!(kept.len(), 2);
    assert_eq!(map[a.get() as usize], None);
    assert_eq!(kept.uri(map[b.get() as usize].unwrap()), Some("b"));
    assert_eq!(kept.uri(map[c.get() as usize].unwrap()), Some("c"));
    assert_eq!(kept.insert("b", None), map[b.get() as usize], "la déduplication survit");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rustty-vt -- cell_stays_compact same_uri_and_id the_table_is_bounded retain_remapped`
Expected: erreur de compilation (`link`, `with_link`, `LinkTable` inconnus).

- [ ] **Step 3: Implement `Cell.link` and `links.rs`**

`LinkTable { links: Vec<Link>, index: HashMap<(String, Option<String>), LinkId> }` ; `LinkId` = index + 1. `retain_remapped` renvoie `map` indexée par l'ancien `id.get()` (longueur `len + 1`, entrée 0 inutilisée). Exporter `pub mod links; pub use links::{Link, LinkId, LinkTable};` dans `lib.rs`. Ajouter `link: None` partout où `Cell { .. }` est construit littéralement (`grep -rn 'Cell {' crates/rustty-vt/src`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rustty-vt`
Expected: tous verts, dont les 4 nouveaux.

- [ ] **Step 5: Commit**

Bump alpha, entrée CHANGELOG « Hyperliens : identifiant de lien par cellule et table partagée ».

```bash
git add crates/rustty-vt/src/links.rs crates/rustty-vt/src/cell.rs crates/rustty-vt/src/lib.rs Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.173 : hyperliens, identifiant de lien par cellule et table partagée"
```

---

### Task 2: OSC 8 dans `Term`, snapshot, reset, compactage

**Files:**
- Modify: `crates/rustty-vt/src/term/mod.rs` (champs `links: Arc<LinkTable>`, `current_link: Option<LinkId>`), `crates/rustty-vt/src/term/osc.rs` (`set_hyperlink_from_params`), `crates/rustty-vt/src/term/perform.rs` (`b"8" => ...`), `crates/rustty-vt/src/term/print.rs:68-70` (`Cell::with_link(c, style, self.current_link)`), `crates/rustty-vt/src/term/reset.rs`, `crates/rustty-vt/src/term/edit.rs:37-40` (`ED 3`), `crates/rustty-vt/src/term/view.rs` (snapshot), `crates/rustty-vt/src/snapshot.rs`, `crates/rustty-vt/src/scrollback.rs` (`pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Line>`), `crates/rustty-vt/src/grid.rs` (`pub fn lines_mut(&mut self) -> &mut [Line]`)
- Test: `crates/rustty-vt/src/term/osc.rs` (`mod tests`), `crates/rustty-vt/src/term/resize_tests.rs`

**Interfaces:**
- Consumes: Task 1.
- Produces: `Term::current_link(&self) -> Option<LinkId>`, `Term::links(&self) -> &Arc<LinkTable>`, `Term::compact_links(&mut self)`, `Snapshot { .., pub links: Arc<LinkTable> }`, `Snapshot::link_uri(&self, id: LinkId) -> Option<&str>`. Séquence : `OSC 8 ; params ; URI ST` — `params[1]` = paramètres `clé=valeur` séparés par `:`, `params[2..]` rejoints par `;` = URI.

- [ ] **Step 1: Write the failing tests** (dans `osc.rs`, helper `fn term() -> Term { Term::new(20, 4, 100) }`)

```rust
#[test]
fn osc8_links_printed_cells_until_closed() {
    let mut t = term();
    t.input(b"\x1b]8;;https://exemple.org\x1b\\ab\x1b]8;;\x1b\\c");
    let id = t.grid().cell(0, 0).link.expect("lien sur a");
    assert_eq!(t.grid().cell(1, 0).link, Some(id));
    assert_eq!(t.grid().cell(2, 0).link, None);
    assert_eq!(t.links().uri(id), Some("https://exemple.org"));
    assert_eq!(t.snapshot().link_uri(id), Some("https://exemple.org"));
}

#[test]
fn sgr_reset_does_not_close_the_link_but_the_id_param_distinguishes() {
    let mut t = term();
    t.input(b"\x1b]8;id=1;https://x\x1b\\a\x1b[0mb\x1b]8;id=2;https://x\x1b\\c");
    let a = t.grid().cell(0, 0).link.unwrap();
    assert_eq!(t.grid().cell(1, 0).link, Some(a));
    assert_ne!(t.grid().cell(2, 0).link, Some(a));
}

#[test]
fn a_uri_with_semicolons_is_kept_whole() {
    let mut t = term();
    t.input(b"\x1b]8;;https://x/?a=1;b=2\x1b\\a");
    let id = t.grid().cell(0, 0).link.unwrap();
    assert_eq!(t.links().uri(id), Some("https://x/?a=1;b=2"));
}

#[test]
fn an_oversized_uri_closes_the_link() {
    let mut t = term();
    let long = format!("\x1b]8;;https://{}\x1b\\", "a".repeat(2100));
    t.input(b"\x1b]8;;https://ok\x1b\\a");
    t.input(long.as_bytes());
    t.input(b"b");
    assert!(t.grid().cell(0, 0).link.is_some());
    assert_eq!(t.grid().cell(1, 0).link, None);
    assert_eq!(t.links().len(), 1);
}

#[test]
fn control_bytes_or_invalid_utf8_close_the_link() {
    for bad in [&b"\x1b]8;;https://x\x01y\x1b\\"[..], b"\x1b]8;;https://\xff\x1b\\"] {
        let mut t = term();
        t.input(b"\x1b]8;;https://ok\x1b\\a");
        t.input(bad);
        t.input(b"b");
        assert_eq!(t.grid().cell(1, 0).link, None);
        assert_eq!(t.links().len(), 1);
    }
}

#[test]
fn ris_clears_the_table_and_the_current_link() {
    let mut t = term();
    t.input(b"\x1b]8;;https://x\x1b\\a\x1bcb");
    assert!(t.links().is_empty());
    assert_eq!(t.current_link(), None);
    assert_eq!(t.grid().cell(0, 0).link, None);
}

#[test]
fn erased_cells_carry_no_link() {
    let mut t = term();
    t.input(b"\x1b]8;;https://x\x1b\\abc\x1b[2D\x1b[K");
    assert_eq!(t.grid().cell(1, 0).link, None);
    assert!(t.grid().cell(0, 0).link.is_some());
}

#[test]
fn alt_screen_shares_the_table_and_compaction_keeps_visible_links() {
    let mut t = Term::new(20, 2, 2);
    t.input(b"\x1b]8;;https://main\x1b\\m\x1b]8;;\x1b\\");
    t.input(b"\x1b[?1049h\x1b]8;;https://alt\x1b\\x\x1b]8;;\x1b\\\x1b[?1049l");
    assert_eq!(t.links().len(), 2, "une table pour les deux écrans");
    // Faire sortir rien de l'écran : « main » reste visible, « alt » n'est plus référencé.
    t.compact_links();
    let id = t.grid().cell(0, 0).link.unwrap();
    assert_eq!(t.links().uri(id), Some("https://main"));
    assert_eq!(t.links().len(), 1);
}

#[test]
fn a_full_table_is_compacted_before_refusing() {
    let mut t = Term::new(4, 2, 0);
    for i in 0..LinkTable::MAX_LINKS {
        t.input(format!("\x1b]8;;u{i}\x1b\\\r").as_bytes());
    }
    // Aucune cellule ne porte de lien (rien d'imprimé) : le compactage vide tout.
    t.input(b"\x1b]8;;https://new\x1b\\z");
    let id = t.grid().cell(0, 0).link.expect("après compactage, place pour le nouveau lien");
    assert_eq!(t.links().uri(id), Some("https://new"));
    assert_eq!(t.links().len(), 1);
}

#[test]
fn ed3_compacts_links_of_the_cleared_history() {
    let mut t = Term::new(4, 1, 10);
    t.input(b"\x1b]8;;https://old\x1b\\o\x1b]8;;\x1b\\\r\nn");
    assert_eq!(t.links().len(), 1);
    t.input(b"\x1b[3J");
    assert!(t.links().is_empty());
}
```

Dans `resize_tests.rs` :

```rust
#[test]
fn resize_and_reflow_keep_links() {
    let mut t = Term::new(6, 2, 10);
    t.input(b"\x1b]8;;https://x\x1b\\abcdefgh\x1b]8;;\x1b\\");
    let id = t.grid().cell(0, 0).link.unwrap();
    t.resize(4, 2);
    let snap = t.snapshot();
    let linked = snap.lines.iter().flat_map(|l| l.cells()).filter(|c| c.link == Some(id)).count();
    assert_eq!(linked, 8);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rustty-vt -- osc8 sgr_reset_does_not a_uri_with_semicolons an_oversized control_bytes ris_clears erased_cells_carry alt_screen_shares a_full_table ed3_compacts resize_and_reflow_keep_links`
Expected: erreur de compilation (`.link` inconnu sur `Cell` déjà fait en Task 1 ; `links()`, `current_link()`, `compact_links`, `link_uri` inconnus), puis échecs d'assertion.

- [ ] **Step 3: Implement**

- `Term` : `links: Arc<LinkTable>`, `current_link: Option<LinkId>` (initialisés dans `new`, remis à zéro dans `reset`).
- `osc.rs` : `pub(crate) fn set_hyperlink_from_params(&mut self, params: &[&[u8]])` — URI = `params.get(2..)` rejoints par `b";"` ; vide → `current_link = None` ; sinon validation (longueur ≤ 2083, UTF-8, pas de `< 0x20` ni `0x7f`) sinon fermeture ; `id=` extrait de `params[1]` découpé sur `:` ; `Arc::make_mut(&mut self.links).insert(...)` ; `None` (pleine) → `self.compact_links()` puis second essai ; toujours `None` → `current_link = None`.
- `perform.rs` : `b"8" => self.set_hyperlink_from_params(params)`.
- `print.rs` : `Cell::with_link(c, style, self.current_link)` pour la cellule imprimée ; la continuation large garde `template` (sans lien).
- `compact_links` : ensemble des ids référencés dans `grid`, `alt_grid`, `scrollback` ; `retain_remapped` ; réécriture des cellules via `Grid::lines_mut` et `Scrollback::iter_mut` ; `current_link` remappé.
- `edit.rs` `ED 3` : après `scrollback.clear()`, `self.compact_links()`.
- `view.rs` / `snapshot.rs` : `links: Arc::clone(&self.links)` ; `Snapshot::link_uri`.
- Construire les `Snapshot` littéraux des tests existants avec `links: Arc::default()` (`grep -rn 'Snapshot {' crates`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --workspace`
Expected: tous verts (les tests de `rustty-render` qui construisent des `Snapshot` compilent après l'ajout du champ).

- [ ] **Step 5: Commit**

Bump alpha, CHANGELOG « Hyperliens : OSC 8 mémorisé par cellule ».

```bash
git add crates/rustty-vt/src crates/rustty-render/tests crates/rustty-render/src Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.174 : hyperliens, OSC 8 mémorisé par cellule"
```

---

### Task 3: Section `[links]` dans la configuration

**Files:**
- Modify: `crates/rustty-config/src/sections.rs` (`Links`, `LinkClick`), `crates/rustty-config/src/config.rs` (champ `links`, validation), `crates/rustty-config/src/lib.rs` (exports), `crates/rustty-config/src/example.rs` (modèle + test des clés), `docs/rustty.example.toml`, `README.md` (section Configuration)
- Test: `crates/rustty-config/src/config.rs` (`mod tests`), `crates/rustty-config/src/example.rs`

**Interfaces:**
- Produces: `pub struct Links { pub click: LinkClick, pub schemes: Vec<String> }` (`Default` : `Ctrl`, `["http","https","file","mailto","ftp","ssh","git"]`), `#[serde(rename_all = "lowercase")] pub enum LinkClick { Ctrl, Plain }`, `Config.links: Links`, `Links::scheme_allowed(&self, uri: &str) -> bool` (schéma = texte avant le premier `:`, comparé sans casse ; `false` sans `:`).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn links_section_parses_and_defaults() {
    let c = Config::from_str("[links]\nclick = \"plain\"\nschemes = [\"https\"]\n").unwrap();
    assert_eq!(c.links.click, LinkClick::Plain);
    assert_eq!(c.links.schemes, ["https"]);
    let d = Config::default().links;
    assert_eq!(d.click, LinkClick::Ctrl);
    assert_eq!(d.schemes, ["http", "https", "file", "mailto", "ftp", "ssh", "git"]);
    assert!(d.scheme_allowed("HTTPS://x"));
    assert!(d.scheme_allowed("mailto:a@b"));
    assert!(!d.scheme_allowed("javascript:alert(1)"));
    assert!(!d.scheme_allowed("pas-de-schéma"));
}

#[test]
fn links_section_is_validated() {
    for (toml, field) in [
        ("[links]\nclick = \"double\"\n", "links.click"),
        ("[links]\nschemes = []\n", "links.schemes"),
        ("[links]\nschemes = [\"Http\"]\n", "links.schemes"),
        ("[links]\nschemes = [\"1ab\"]\n", "links.schemes"),
        ("[links]\nschemes = [\"http\", \"http\"]\n", "links.schemes"),
    ] {
        let err = Config::from_str(toml).unwrap_err().to_string();
        assert!(err.contains(field), "{toml:?} → {err}");
    }
}
```

Et dans `every_section_and_key_is_documented` : ajouter `"[links]"`, `"click"`, `"schemes"`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rustty-config -- links_section every_section_and_key`
Expected: erreur de compilation (`links`, `LinkClick`), puis échec sur les clés absentes du modèle.

- [ ] **Step 3: Implement**

`click = "double"` échoue déjà à la désérialisation serde : l'erreur doit nommer `links.click` (vérifier `ConfigError::from_toml` : si le chemin n'y est pas, valider après coup n'est pas possible — accepter le message serde s'il contient `links.click`, sinon le test documente la règle à ajuster). Validation dans `Config::validate` avec `invalid("links.schemes", …)`. Modèle TOML (même texte dans `example.rs` et `docs/rustty.example.toml`) :

```toml
[links]
# Geste qui ouvre un lien (OSC 8 des programmes comme `ls --hyperlink`) dans
# le navigateur par défaut : "ctrl" (ctrl+clic) ou "plain" (clic simple).
click = "ctrl"
# Schémas d'URL reconnus et ouverts ; tout autre lien reste du texte.
schemes = ["http", "https", "file", "mailto", "ftp", "ssh", "git"]
```

README, section Configuration : une phrase sur `[links]` (`click`, `schemes`) à la suite de `[spellcheck]`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rustty-config`
Expected: verts.

- [ ] **Step 5: Commit**

```bash
git add crates/rustty-config/src docs/rustty.example.toml README.md Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.175 : hyperliens, section [links]"
```

---

### Task 4: Soulignement de la plage survolée dans le renderer

**Files:**
- Modify: `crates/rustty-render/src/frame.rs` (`CellRange`, `PaneFrame.link_hover`), `crates/rustty-render/src/grid.rs:103-123` (décoration), `crates/rustty-render/src/lib.rs` (export `CellRange`), `crates/rustty-render/tests/offscreen.rs` et `crates/rustty/src/render_frame.rs` (constructions de `PaneFrame` : `link_hover: None`)
- Test: `crates/rustty-render/src/grid.rs` (`mod tests`)

**Interfaces:**
- Produces: `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct CellRange { pub start: (usize, usize), pub end: (usize, usize) }` (`(row, col)`, bornes incluses, `start ≤ end` en ordre ligne puis colonne) avec `pub fn contains(&self, row: usize, col: usize) -> bool` ; `PaneFrame { .., pub link_hover: Option<CellRange> }`.

- [ ] **Step 1: Write the failing test** (dans `grid.rs`, à côté des tests existants de `pane_instances`)

```rust
#[test]
fn a_hovered_link_range_is_underlined_across_rows() {
    let mut term = Term::new(4, 2, 0);
    term.input(b"abcdef");
    let snapshot = term.snapshot();
    let metrics = test_metrics();
    let plain = pane_instances(&pane(&snapshot, None), metrics, &Palette::default(), 0);
    let hovered = pane_instances(
        &pane(&snapshot, Some(CellRange { start: (0, 2), end: (1, 1) })),
        metrics,
        &Palette::default(),
        0,
    );
    assert_eq!(hovered.decorations.len(), plain.decorations.len() + 4, "c d e f");
    let ys: Vec<f32> = hovered.decorations[plain.decorations.len()..].iter().map(|q| q.y).collect();
    assert_eq!(ys.iter().filter(|&&y| y == metrics.underline_y as f32).count(), 2);
}
```

(`pane(snapshot, link_hover)` et `test_metrics()` : petits helpers du module de tests, à créer s'ils n'existent pas ; `CellRange::contains` testé par `assert!(CellRange { start: (0, 2), end: (1, 1) }.contains(0, 3) && !…contains(1, 2))`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustty-render -- a_hovered_link_range`
Expected: erreur de compilation (`CellRange`, `link_hover`).

- [ ] **Step 3: Implement**

Dans la boucle de `pane_instances`, après le soulignement `SGR 4` : `if pane.link_hover.is_some_and(|r| r.contains(row, col)) && !cell.style.attrs.contains(Attrs::UNDERLINE)` → même quad (`underline_y`, `underline_thickness`, couleur `fg`). Les continuations larges sont déjà sautées.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --workspace`
Expected: verts (images de référence inchangées, `link_hover: None` partout).

- [ ] **Step 5: Commit**

```bash
git add crates/rustty-render/src crates/rustty-render/tests crates/rustty/src/render_frame.rs Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.176 : hyperliens, soulignement de la plage survolée"
```

---

### Task 5: Module pur `links` : plage d'un lien, clic, texte d'infobulle

**Files:**
- Create: `crates/rustty/src/links.rs`, `crates/rustty/src/links_tests.rs`
- Modify: `crates/rustty/src/main.rs` (`mod links;`)

**Interfaces:**
- Consumes: `rustty_vt::{Line, LinkId, Snapshot}`, `rustty_render::CellRange`, `rustty_config::LinkClick`, `crate::mouse::click::DOUBLE_CLICK`, `Mods` (`crate::keyboard`).
- Produces:
  - `pub fn link_run(lines: &[Line], row: usize, col: usize) -> Option<(LinkId, CellRange)>` : cellules contiguës portant le même `link` que `(row, col)` ; vers l'arrière jusqu'à une cellule différente, en remontant à la fin de la ligne précédente quand `lines[row-1].wrapped` et que la colonne 0 est atteinte ; vers l'avant symétriquement.
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct LinkGesture { pub click: LinkClick, pub captured: bool }` et `pub fn gesture_armed(g: LinkGesture, mods: Mods) -> bool` : `captured` exige `SHIFT` ; `Ctrl` exige `CTRL` ; `Plain` n'exige rien de plus.
  - `#[derive(Debug, Default)] pub struct LinkClick` (machine d'états) : `pub fn press(&mut self, run: Option<(TermId, LinkId, CellRange)>, armed: bool) -> bool` (vrai si l'appui est capté : lien présent et armé en mode ctrl — pas de sélection), `pub fn release(&mut self, run: Option<(TermId, LinkId, CellRange)>, selection_empty: bool, now: Instant) -> Option<LinkId>` (l'id à ouvrir si appui et relâche sur la même `(term, id, range)`, armé à l'appui, sélection vide, et pas une seconde relâche sur le même lien dans `DOUBLE_CLICK`), `pub fn reset(&mut self)`.
  - `pub fn tooltip_text(uri: &str, hint: Option<&str>) -> String` : `uri` tronquée au milieu à 60 cellules (`…`), puis ` · ` + `hint` s'il y en a un.
  - `pub fn tooltip_hint(click: LinkClick, captured: bool) -> Option<&'static str>` : `None` pour `Plain` non capturé, `"ctrl+clic"`, `"shift+clic"`, `"shift+ctrl+clic"`.

- [ ] **Step 1: Write the failing tests** (`links_tests.rs`, `use super::*;`, lignes construites avec `Term::new(cols, rows, 0)` + `input` OSC 8)

```rust
fn lines(cols: usize, bytes: &[u8]) -> Vec<Line> {
    let mut t = Term::new(cols, 4, 0);
    t.input(bytes);
    t.snapshot().lines
}

#[test]
fn a_run_spans_wrapped_lines_from_either_half() {
    let l = lines(4, b"xx\x1b]8;;https://a\x1b\\abcd\x1b]8;;\x1b\\yy");
    let expected = CellRange { start: (0, 2), end: (1, 1) };
    assert_eq!(link_run(&l, 0, 3).map(|(_, r)| r), Some(expected));
    assert_eq!(link_run(&l, 1, 0).map(|(_, r)| r), Some(expected));
    assert_eq!(link_run(&l, 1, 2), None);
}

#[test]
fn a_run_stops_at_a_different_link_and_at_a_hard_newline() {
    let l = lines(8, b"\x1b]8;;https://a\x1b\\ab\x1b]8;;https://b\x1b\\cd\x1b]8;;\x1b\\\r\n\x1b]8;;https://b\x1b\\e");
    assert_eq!(link_run(&l, 0, 1).map(|(_, r)| r), Some(CellRange { start: (0, 0), end: (0, 1) }));
    assert_eq!(link_run(&l, 0, 3).map(|(_, r)| r), Some(CellRange { start: (0, 2), end: (0, 3) }));
    assert_eq!(link_run(&l, 1, 0).map(|(_, r)| r), Some(CellRange { start: (1, 0), end: (1, 0) }), "pas de wrapped : la ligne 0 ne se joint pas");
}

#[test]
fn the_gesture_needs_ctrl_or_shift_as_configured() {
    let g = |click, captured| LinkGesture { click, captured };
    assert!(gesture_armed(g(LinkClick::Plain, false), Mods::empty()));
    assert!(!gesture_armed(g(LinkClick::Ctrl, false), Mods::empty()));
    assert!(gesture_armed(g(LinkClick::Ctrl, false), Mods::CTRL));
    assert!(!gesture_armed(g(LinkClick::Plain, true), Mods::empty()));
    assert!(gesture_armed(g(LinkClick::Plain, true), Mods::SHIFT));
    assert!(!gesture_armed(g(LinkClick::Ctrl, true), Mods::CTRL));
    assert!(gesture_armed(g(LinkClick::Ctrl, true), Mods::CTRL | Mods::SHIFT));
}

#[test]
fn a_drag_selects_and_a_still_click_opens() {
    let run = Some((TermId(1), LinkId::new(1).unwrap(), CellRange { start: (0, 0), end: (0, 3) }));
    let mut c = LinkClick::default();
    let t0 = Instant::now();
    assert!(!c.press(run, true), "mode plain : la sélection démarre");
    assert_eq!(c.release(run, false, t0), None, "glisser = copie");
    c.press(run, true);
    assert_eq!(c.release(run, true, t0 + DOUBLE_CLICK * 2), LinkId::new(1));
}

#[test]
fn the_modifier_counts_at_press() {
    let run = Some((TermId(1), LinkId::new(1).unwrap(), CellRange { start: (0, 0), end: (0, 3) }));
    let mut c = LinkClick::default();
    c.press(run, false);
    assert_eq!(c.release(run, true, Instant::now()), None, "armé seulement à la relâche : rien");
    assert!(c.press(run, true), "mode ctrl armé : l'appui est capté");
    assert_eq!(c.release(run, true, Instant::now()), LinkId::new(1));
}

#[test]
fn a_release_elsewhere_or_a_second_click_does_not_open() {
    let run = Some((TermId(1), LinkId::new(1).unwrap(), CellRange { start: (0, 0), end: (0, 3) }));
    let other = Some((TermId(1), LinkId::new(2).unwrap(), CellRange { start: (1, 0), end: (1, 3) }));
    let mut c = LinkClick::default();
    let t0 = Instant::now();
    c.press(run, true);
    assert_eq!(c.release(other, true, t0), None);
    c.press(run, true);
    assert_eq!(c.release(run, true, t0), LinkId::new(1));
    c.press(run, true);
    assert_eq!(c.release(run, true, t0 + DOUBLE_CLICK / 2), None, "double clic : une seule ouverture");
    c.press(run, true);
    assert_eq!(c.release(run, true, t0 + DOUBLE_CLICK * 2), LinkId::new(1));
}

#[test]
fn tooltip_text_truncates_in_the_middle_and_adds_the_hint() {
    let long = format!("https://exemple.org/{}/fin.html", "a".repeat(80));
    let text = tooltip_text(&long, Some("ctrl+clic"));
    assert!(text.starts_with("https://exemple.org/"));
    assert!(text.ends_with("/fin.html · ctrl+clic"));
    assert!(text.contains('…'));
    assert_eq!(text.split(" · ").next().unwrap().chars().count(), 60);
    assert_eq!(tooltip_text("https://x", None), "https://x");
    assert_eq!(tooltip_hint(LinkClick::Plain, false), None);
    assert_eq!(tooltip_hint(LinkClick::Ctrl, false), Some("ctrl+clic"));
    assert_eq!(tooltip_hint(LinkClick::Plain, true), Some("shift+clic"));
    assert_eq!(tooltip_hint(LinkClick::Ctrl, true), Some("shift+ctrl+clic"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rustty -- links::tests`
Expected: erreur de compilation (module absent).

- [ ] **Step 3: Implement `links.rs`**

`LinkClick { armed: Option<(TermId, LinkId, CellRange)>, last_open: Option<(TermId, LinkId, Instant)> }`. Attention au nom : la machine d'états `links::LinkClick` et l'énumération `rustty_config::LinkClick` cohabitent ; importer la seconde sous `use rustty_config::LinkClick as ClickMode` dans ce module et dans `link_input.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rustty -- links::tests`
Expected: 7 verts.

- [ ] **Step 5: Commit**

```bash
git add crates/rustty/src/links.rs crates/rustty/src/links_tests.rs crates/rustty/src/main.rs Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.177 : hyperliens, plage, geste et infobulle (logique pure)"
```

---

### Task 6: Infobulle générique extraite du correcteur

**Files:**
- Create: `crates/rustty/src/tooltip.rs`
- Modify: `crates/rustty/src/spell/popup.rs` (`layout` et `chrome` délèguent le cadre et le message à `tooltip`), `crates/rustty/src/main.rs` (`mod tooltip;`)
- Test: `crates/rustty/src/tooltip.rs` (`mod tests`), tests existants de `spell/popup.rs` inchangés

**Interfaces:**
- Produces: `pub struct Tooltip { pub anchor: PixelRect, pub message: String }`, `pub fn layout(tip: &Tooltip, rows: u32, viewport: (u32, u32), metrics: CellMetrics) -> PopupLayout` (même algorithme que `spell::popup::layout` : largeur `clamp(12, 60) + 2` cellules sur le plus large de message et rangées — ici `rows` rangées de même largeur que le message, hauteur `2*PADDING + row_height*(rows+1)`, au-dessus de l'ancre si la place existe sinon dessous ; `PopupLayout` déménage dans `tooltip.rs`, réexporté par `spell`), `pub fn chrome(tip: &Tooltip, layout: &PopupLayout, palette: &Palette, metrics: CellMetrics) -> Chrome` (cadre `ansi[8]`, fond `background`, message grisé) ; `pub const PADDING: u32 = 4; pub const ROW_PADDING: u32 = 3;`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_tooltip_without_rows_is_one_message_row_placed_under_the_anchor_when_room_above_is_missing() {
    let tip = Tooltip { anchor: PixelRect::new(10, 0, 8, 16), message: "https://x".into() };
    let l = layout(&tip, 0, (800, 600), test_metrics());
    assert_eq!(l.rows.len(), 0);
    assert_eq!(l.rect.y, 16, "sous l'ancre : pas de place au-dessus");
    assert_eq!(l.rect.height, 2 * PADDING + test_metrics().height + 2 * ROW_PADDING);
    let c = chrome(&tip, &l, &Palette::default(), test_metrics());
    assert_eq!(c.texts.len(), 1);
    assert_eq!(c.texts[0].text, "https://x");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustty -- tooltip::tests`
Expected: erreur de compilation.

- [ ] **Step 3: Implement and refactor `spell/popup.rs` onto it**

`spell::popup::layout` = `tooltip::layout(&Tooltip { anchor, message }, replacements.len(), …)` avec la largeur calculée sur le plus large des remplacements et du message (passer `widest_cells: usize` en paramètre de `tooltip::layout` plutôt que de recalculer : signature finale `layout(tip, rows, widest_cells, viewport, metrics)` — adapter le test ci-dessus avec `widest_cells = tip.message.width()`). `spell::popup::chrome` = `tooltip::chrome(...)` + les rangées de remplacement.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rustty`
Expected: verts, tests de `spell::popup` inchangés.

- [ ] **Step 5: Commit**

```bash
git add crates/rustty/src/tooltip.rs crates/rustty/src/spell/popup.rs crates/rustty/src/spell/mod.rs crates/rustty/src/main.rs Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.178 : infobulle générique extraite du correcteur"
```

---

### Task 7: Survol, infobulle, clic et ouverture dans la fenêtre

**Files:**
- Create: `crates/rustty/src/link_input.rs`
- Modify: `crates/rustty/src/window_state.rs` (champs `link_hover: Option<LinkHover>`, `link_click: links::LinkClick`), `crates/rustty/src/input.rs` (`on_cursor_moved`, `on_cursor_left`, `on_modifiers`, `on_press`, `on_release`), `crates/rustty/src/divider_drag.rs:46-70` (`update_cursor_icon` : `CursorIcon::Pointer` sur un lien survolé armé), `crates/rustty/src/render_frame.rs` (`PaneView.link_hover`), `crates/rustty/src/render.rs` (chrome de l'infobulle après le correcteur), `crates/rustty/src/app.rs:168` (échéance : `min` de `drag` et `link_deadline`), `crates/rustty/src/model.rs` (`Effect::OpenLink(String)`), `crates/rustty/src/effects.rs` (`open_link`), `crates/rustty/src/external.rs:106` (`spawn_detached(command, target: &OsStr)`), `crates/rustty/src/main.rs`
- Test: `crates/rustty/src/link_input.rs` (`mod tests` sur les fonctions pures `LinkHover::tooltip_due`), `crates/rustty/src/external_tests.rs`, `docs/e2e.md`

**Interfaces:**
- Consumes: Tasks 2 à 6.
- Produces (sur `OsWindow`) : `pub(crate) fn link_hover(&mut self, term: TermId, cell: Option<(usize, usize)>) -> bool` (recalcule `self.link_hover` depuis `tw.snapshot()` et `links::link_run` ; rien si le schéma n'est pas autorisé ou si le geste n'est pas armé sous capture ; vrai si redessin), `pub(crate) fn leave_link(&mut self) -> bool`, `pub(crate) fn link_deadline(&self) -> Option<Instant>` (`since + 400 ms` tant que l'infobulle n'est pas ouverte), `pub(crate) fn link_tick(&mut self, now: Instant) -> bool` (ouvre l'infobulle quand l'échéance est passée ; appelé au réveil `NewEvents`/`AboutToWait` comme le glisser), `pub(crate) fn link_chrome(&self) -> Option<Chrome>`, `pub(crate) fn link_range_of(&self, term: TermId) -> Option<CellRange>`, `pub(crate) fn link_press(&mut self, term: TermId, cell: Option<(usize, usize)>, captured: bool) -> bool`, `pub(crate) fn link_release(&mut self, term: TermId, cell: Option<(usize, usize)>) -> Option<Effect>` ; `pub struct LinkHover { pub term: TermId, pub id: LinkId, pub range: CellRange, pub uri: String, pub since: Instant, pub tooltip: bool }` avec `pub const TOOLTIP_DELAY: Duration = Duration::from_millis(400)` et `pub fn tooltip_due(&self, now: Instant) -> bool`.
- `Effect::OpenLink(String)` → `effects.rs::open_link(uri)` : `config.links.scheme_allowed(&uri)` sinon bandeau « ouvrir le lien : schéma non autorisé » ; `external::spawn_detached(&external::opener(), OsStr::new(&uri))` ; `report_external("ouvrir le lien", result)`.

- [ ] **Step 1: Write the failing tests**

`link_input.rs` :

```rust
#[test]
fn the_tooltip_is_due_after_the_delay() {
    let t0 = Instant::now();
    let h = LinkHover { term: TermId(1), id: LinkId::new(1).unwrap(), range: CellRange { start: (0, 0), end: (0, 1) }, uri: "https://x".into(), since: t0, tooltip: false };
    assert!(!h.tooltip_due(t0 + TOOLTIP_DELAY / 2));
    assert!(h.tooltip_due(t0 + TOOLTIP_DELAY));
}
```

`external_tests.rs` :

```rust
#[test]
fn spawn_detached_takes_any_os_string_target() {
    // Un programme qui n'existe pas : l'erreur vient du lancement, pas du type de cible.
    let err = spawn_detached(&["rustty-programme-inexistant".to_string()], std::ffi::OsStr::new("https://x")).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rustty -- the_tooltip_is_due spawn_detached_takes_any`
Expected: erreur de compilation.

- [ ] **Step 3: Implement**

Câblage dans `input.rs` :
- `on_cursor_moved` : après `spell_hover`, `if self.link_hover(term, cell) { redraw }` ; dans la branche « pas de panneau » et dans `on_cursor_left` : `leave_link()`.
- `on_modifiers` : si `self.cursor_inside`, réévaluer `link_hover` sur la cellule courante (`pane_under` + `cell_under`) — les modificateurs changent l'armement sous capture et le texte de l'infobulle.
- `on_press` (branche panneau, avant l'envoi à l'application) : `let captured = modes.mouse != MouseMode::None && !shift` ; `if self.link_press(term, cell, captured) { return effects }` (appui capté : ni sélection ni envoi). Sous capture avec shift : la règle existante continue (sélection), `link_press` reçoit `captured = modes.mouse != MouseMode::None`.
- `on_release` : avant le bloc `self.dragging`, `if let Some(effect) = self.link_release(term_sous_la_souris, cell) { self.dragging = false; return vec![effect] }` — la sélection vide est connue via `self.selection.as_ref().is_none_or(|(_, s)| s.is_empty())`.
- `update_cursor_icon` : `CursorIcon::Pointer` quand `self.link_hover.is_some()`, avant les curseurs de redimensionnement de panneau.
- `render_frame::PaneView.link_hover` rempli par `link_range_of(term)` ; `render.rs` : `link_chrome()` ajouté à `menu_chrome` après le correcteur (le correcteur garde la priorité : pas d'infobulle de lien si `spell.popup` est ouverte).
- `app.rs` : `deadline = [drag.deadline(), w.link_deadline()].into_iter().flatten().min()` ; au réveil, `if w.link_tick(now) { request_redraw }`.
- L'infobulle se ferme (`leave_link`) à l'appui, à la molette (`wheel_input.rs`), à la frappe (`on_key`) et à la perte de focus.
- `docs/e2e.md` : section `## Liens` avant `## Robustesse` avec les cas de la spec § 6 propres à OSC 8 (`ls --hyperlink=always`, `printf '\e]8;;https://exemple.org\e\\texte\e]8;;\e\\'`, lien coupé par un retour automatique, ctrl+clic et clic simple selon le TOML, `vim` avec et sans shift, infobulle après 400 ms avec suffixe, schéma non autorisé `printf '\e]8;;javascript:alert(1)\e\\x\e]8;;\e\\'` = texte ordinaire, double clic = une seule ouverture).
- README, section « Souris » : un paragraphe « Liens » (OSC 8, survol, ctrl+clic, `[links]`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets`
Expected: verts, 0 avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/rustty/src docs/e2e.md README.md Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "rustty 0.1.0-alpha.179 : hyperliens, survol, infobulle et ouverture"
```

---

## Self-review

- Spec § 5.1 : Tasks 1-2 (cellule 16 octets, table, bornes, compactage, `RIS`, `ED 3`, effacements, reflow, snapshot). § 5.3 : Tasks 4, 5, 7. § 5.4 : Tasks 6, 7. § 5.5 : Tasks 5, 7 (`Effect::OpenLink`, bandeau, double clic, capture). § 5.6 : Task 3 (`detect_urls` au jalon 2). § 5.7 : Task 3 (`scheme_allowed`), Task 2 (URI bornée), Task 7 (argument unique). § 6 : les tests nommés ; l'image de référence `link_hover` est au jalon 2 avec la détection.
- Types : `CellRange` vit dans `rustty-render` et est consommé par `rustty::links` ; `rustty_config::LinkClick` (énumération) vs `links::LinkClick` (machine) : alias `ClickMode` imposé en Task 5.
- Review Focus : 1 → Task 2, 2 → Task 2, 3 → Task 5, 4 → Task 5, 5 → Task 5.
