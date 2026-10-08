# Plan d'implémentation — Fondations et `rustty-vt`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mettre en place le workspace Cargo, la CI trois OS, et livrer la crate `rustty-vt` : une émulation de terminal pure (grille, scrollback, curseur, modes, SGR, OSC, jeux de caractères, caractères larges, resize, instantané) pilotée par `vte`, sans aucune dépendance système.

**Architecture:** `rustty-vt` expose `Term`, qui consomme des octets via `Term::input(&[u8])` et expose son état via `Term::snapshot()`. Le parseur `vte::Parser` est sorti de `Term` le temps d'un `advance` pour que `Term` implémente `vte::Perform` directement. La grille est un `Vec<Line>`, le scrollback un anneau borné de `Line`, les graphèmes à largeur nulle vivent dans une table annexe par ligne.

**Tech Stack:** Rust 1.96 (edition 2024), `vte`, `unicode-width`, `bitflags`, `base64`; tests avec `cargo test`, `insta` pour les snapshots de grille.

**Spec:** `docs/superpowers/specs/2026-10-08-rustty-design.md` (sections 2.2, 3.1, 6, 7).

## Global Constraints

- Code et identifiants en **anglais** ; commentaires, messages de commit, CHANGELOG en **français**. Aucune mention d'assistant dans les commits.
- Chaque commit bumpe la version du workspace. Pendant le chantier v0.1 la version est une pré-release SemVer `0.1.0-alpha.N` ; chaque tâche incrémente `N` (tâche 1 → `alpha.1`, tâche 2 → `alpha.2`, …) dans `[workspace.package] version` du `Cargo.toml` racine, et ajoute une entrée en tête de `CHANGELOG.md` au format `## 0.1.0-alpha.N — 2026-10-08 · « titre »`.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace` doivent passer avant chaque commit, lancés depuis la racine.
- `git add` ciblé, jamais `-A`.
- `rustty-vt` n'a aucune dépendance système ni I/O : pas de `std::fs`, pas de thread.
- Les séquences inconnues sont ignorées silencieusement (journalisation `debug` ajoutée dans un plan ultérieur), jamais de `panic!` sur une entrée.
- Toutes les positions sont en cellules, `col` puis `row`, indexées à 0 en interne ; les paramètres CSI sont indexés à 1 et convertis à l'entrée.

## Review Focus

Entrées que la spec implique sans les nommer, et le comportement attendu. Chaque ligne a son test dans la tâche indiquée.

1. **Paramètres CSI absurdes** (`CSI 0 A`, `CSI 99999 C`, `CSI ; ; H`) : 0 vaut 1 pour les mouvements, les déplacements sont bornés à la grille, les paramètres vides valent défaut. Test dans la tâche 7 (`csi_zero_and_huge_params_are_clamped`).
2. **Caractère large en dernière colonne** : il passe à la ligne suivante, la dernière cellule de la ligne devient un blanc, jamais une moitié orpheline. Test dans la tâche 13 (`wide_char_at_last_column_wraps`) ; grille d'une seule colonne couverte par `garbage_input_does_not_panic` (tâche 15) et le garde-fou de `put_char`.
3. **Resize vers une grille plus petite que la position du curseur** : le curseur est ramené dans la grille, la région de scroll est réinitialisée, aucun index hors borne. Test dans la tâche 14 (`resize_shrink_clamps_cursor_and_region`).
4. **Région de scroll inversée ou hors borne** (`CSI 10;5 r`, `CSI 1;999 r`) : ignorée ou bornée, jamais de panique. Test dans la tâche 7 (`decstbm_invalid_region_is_ignored`).
5. **OSC 52 avec base64 invalide** : l'événement presse-papiers n'est pas émis, le reste du flux continue d'être interprété. Test dans la tâche 11 (`osc52_invalid_base64_is_ignored`).

---

### Task 1 : Workspace Cargo, crate vide et CI

**Files:**
- Create: `Cargo.toml`
- Create: `crates/rustty-vt/Cargo.toml`
- Create: `crates/rustty-vt/src/lib.rs`
- Create: `crates/rustty-vt/tests/smoke.rs`
- Create: `.github/workflows/ci.yml`
- Modify: `CHANGELOG.md`
- Modify: `.gitignore`

**Interfaces:**
- Produces: crate `rustty_vt` avec `pub const VERSION: &str` ; le workspace où toutes les crates héritent `version`, `edition`, `license`.

- [ ] **Step 1: Écrire le test de fumée (qui échoue car la crate n'existe pas)**

`crates/rustty-vt/tests/smoke.rs` :

```rust
//! Test de fumée : la crate se compile et expose sa version.

#[test]
fn version_is_the_workspace_prerelease() {
    assert!(rustty_vt::VERSION.starts_with("0.1.0-alpha."), "got {}", rustty_vt::VERSION);
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cd /home/seb/Dev/rustty && cargo test --workspace`
Expected: erreur `could not find Cargo.toml`.

- [ ] **Step 3: Créer le workspace et la crate**

`Cargo.toml` (racine) :

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
version = "0.1.0-alpha.1"
edition = "2024"
license = "GPL-3.0-only"
repository = "https://github.com/yrbane/rustty"
rust-version = "1.96"

[workspace.dependencies]
vte = "0.15"
unicode-width = "0.2"
bitflags = "2"
base64 = "0.22"
insta = "1"

[workspace.lints.clippy]
all = "warn"
```

Si `cargo` refuse `vte = "0.15"` (version inexistante), lancer `cargo add vte --package rustty-vt` pour obtenir la dernière 0.x, reporter cette version dans `[workspace.dependencies]` et remettre `vte.workspace = true` dans la crate. Même procédure pour les autres dépendances.

`crates/rustty-vt/Cargo.toml` :

```toml
[package]
name = "rustty-vt"
description = "Émulation de terminal pure : grille, scrollback, curseur, modes"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]

[dev-dependencies]

[lints]
workspace = true
```

`crates/rustty-vt/src/lib.rs` :

```rust
//! Émulation de terminal pure, sans I/O : transforme un flux d'octets en état
//! de grille. Consommée par le renderer et le binaire `rustty`.

/// Version de la crate, héritée du workspace. Affichée dans l'interface.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

`.github/workflows/ci.yml` :

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
jobs:
  check:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
```

Ajouter à `.gitignore` la ligne `Cargo.lock` **n'est pas** souhaité : on versionne `Cargo.lock` (binaire final). Ne rien changer au `.gitignore` sauf si `cargo` crée des fichiers parasites.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cd /home/seb/Dev/rustty && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: `test version_is_the_workspace_prerelease ... ok`.

- [ ] **Step 5: CHANGELOG et commit**

Ajouter en tête de `CHANGELOG.md`, sous le titre et l'intro :

```markdown
## 0.1.0-alpha.1 — 2026-10-08 · « Workspace et CI »

- Workspace Cargo (edition 2024, version unique), crate `rustty-vt` vide exposant `VERSION`.
- CI GitHub Actions : fmt, clippy en mode strict, tests sur Linux, macOS et Windows.
```

```bash
cd /home/seb/Dev/rustty
git add Cargo.toml Cargo.lock crates/rustty-vt/Cargo.toml crates/rustty-vt/src/lib.rs crates/rustty-vt/tests/smoke.rs .github/workflows/ci.yml CHANGELOG.md
git commit -m "Workspace Cargo, crate rustty-vt et CI trois OS (0.1.0-alpha.1)"
```

---

### Task 2 : Couleurs, attributs, style et cellule

**Files:**
- Create: `crates/rustty-vt/src/color.rs`
- Create: `crates/rustty-vt/src/cell.rs`
- Modify: `crates/rustty-vt/src/lib.rs`
- Modify: `crates/rustty-vt/Cargo.toml` (dépendance `bitflags`)

**Interfaces:**
- Produces:
  - `pub enum Color { Default, Indexed(u8), Rgb(u8, u8, u8) }` (Copy, Default = `Default`)
  - `pub struct Attrs: u16` (bitflags) : `BOLD, DIM, ITALIC, UNDERLINE, BLINK, INVERSE, HIDDEN, STRIKETHROUGH, WIDE, WIDE_CONTINUATION`
  - `pub struct Style { pub fg: Color, pub bg: Color, pub attrs: Attrs }` (Copy, Default)
  - `pub struct Cell { pub c: char, pub style: Style }` (Copy, Default = espace, style par défaut)
  - `Cell::new(c: char, style: Style) -> Cell`, `Cell::erased(style: Style) -> Cell` (espace, `fg`/`bg` du style, attrs vides), `Cell::is_wide(&self) -> bool`, `Cell::is_wide_continuation(&self) -> bool`.

- [ ] **Step 1: Écrire les tests**

Dans `crates/rustty-vt/src/cell.rs`, en bas :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    #[test]
    fn default_cell_is_a_blank_space_with_default_style() {
        let c = Cell::default();
        assert_eq!(c.c, ' ');
        assert_eq!(c.style, Style::default());
        assert_eq!(c.style.fg, Color::Default);
        assert_eq!(c.style.bg, Color::Default);
        assert!(c.style.attrs.is_empty());
    }

    #[test]
    fn erased_cell_keeps_colors_but_drops_attributes() {
        let style = Style { fg: Color::Indexed(1), bg: Color::Rgb(1, 2, 3), attrs: Attrs::BOLD | Attrs::WIDE };
        let c = Cell::erased(style);
        assert_eq!(c.c, ' ');
        assert_eq!(c.style.fg, Color::Indexed(1));
        assert_eq!(c.style.bg, Color::Rgb(1, 2, 3));
        assert!(c.style.attrs.is_empty());
    }

    #[test]
    fn wide_flags_are_reported() {
        let wide = Cell::new('漢', Style { attrs: Attrs::WIDE, ..Style::default() });
        let cont = Cell::new(' ', Style { attrs: Attrs::WIDE_CONTINUATION, ..Style::default() });
        assert!(wide.is_wide() && !wide.is_wide_continuation());
        assert!(cont.is_wide_continuation() && !cont.is_wide());
        assert!(!Cell::default().is_wide());
    }

    #[test]
    fn cell_stays_compact() {
        // Garde-fou : la grille peut contenir des millions de cellules.
        assert!(std::mem::size_of::<Cell>() <= 16, "Cell = {} octets", std::mem::size_of::<Cell>());
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: erreurs de compilation `unresolved import`.

- [ ] **Step 3: Implémenter**

`crates/rustty-vt/Cargo.toml`, section `[dependencies]` :

```toml
bitflags.workspace = true
```

`crates/rustty-vt/src/color.rs` :

```rust
//! Couleurs de cellule telles que vues par l'émulation : la résolution en RGB
//! réel (palette, couleurs par défaut) appartient au renderer.

/// Couleur d'avant-plan ou d'arrière-plan d'une cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Color {
    /// Couleur par défaut du terminal (configurable).
    #[default]
    Default,
    /// Index 0..=255 dans la palette.
    Indexed(u8),
    /// Couleur vraie.
    Rgb(u8, u8, u8),
}
```

`crates/rustty-vt/src/cell.rs` :

```rust
//! Cellule de la grille : un caractère de base et son style. Les caractères
//! combinants sont stockés à part, dans la ligne (voir `line.rs`).

use bitflags::bitflags;

use crate::color::Color;

bitflags! {
    /// Attributs de rendu et drapeaux de largeur d'une cellule.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Attrs: u16 {
        const BOLD = 1 << 0;
        const DIM = 1 << 1;
        const ITALIC = 1 << 2;
        const UNDERLINE = 1 << 3;
        const BLINK = 1 << 4;
        const INVERSE = 1 << 5;
        const HIDDEN = 1 << 6;
        const STRIKETHROUGH = 1 << 7;
        /// Première moitié d'un caractère large.
        const WIDE = 1 << 8;
        /// Seconde moitié d'un caractère large : ne se dessine pas.
        const WIDE_CONTINUATION = 1 << 9;
    }
}

/// Style courant (état SGR) ou style d'une cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

/// Une cellule de la grille.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cell {
    pub c: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self::new(' ', Style::default())
    }
}

impl Cell {
    pub const fn new(c: char, style: Style) -> Self {
        Self { c, style }
    }

    /// Cellule effacée : conserve les couleurs courantes (comportement « bce »
    /// de xterm) mais aucun attribut.
    pub const fn erased(style: Style) -> Self {
        Self::new(' ', Style { fg: style.fg, bg: style.bg, attrs: Attrs::empty() })
    }

    pub const fn is_wide(&self) -> bool {
        self.style.attrs.contains(Attrs::WIDE)
    }

    pub const fn is_wide_continuation(&self) -> bool {
        self.style.attrs.contains(Attrs::WIDE_CONTINUATION)
    }
}
```

`crates/rustty-vt/src/lib.rs`, ajouter :

```rust
pub mod cell;
pub mod color;

pub use cell::{Attrs, Cell, Style};
pub use color::Color;
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: 5 tests `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`Cargo.toml` racine : `version = "0.1.0-alpha.2"`. En tête de `CHANGELOG.md` :

```markdown
## 0.1.0-alpha.2 — 2026-10-08 · « Cellule et style »

- `rustty-vt` : types `Color`, `Attrs`, `Style`, `Cell` ; cellule effacée conservant les couleurs.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-vt/Cargo.toml crates/rustty-vt/src/lib.rs crates/rustty-vt/src/color.rs crates/rustty-vt/src/cell.rs CHANGELOG.md
git commit -m "rustty-vt : couleurs, attributs, style et cellule (0.1.0-alpha.2)"
```

---

### Task 3 : Ligne avec table annexe des caractères à largeur nulle

**Files:**
- Create: `crates/rustty-vt/src/line.rs`
- Modify: `crates/rustty-vt/src/lib.rs`

**Interfaces:**
- Consumes: `Cell`, `Style` (tâche 2).
- Produces:
  - `pub struct Line` (Clone, Debug, PartialEq, Eq) avec `pub wrapped: bool` (la ligne continue sur la suivante, pour la copie et le futur rewrap).
  - `Line::new(cols: usize) -> Line`, `Line::filled(cols, template: Cell) -> Line`
  - `len() -> usize`, `get(col) -> &Cell`, `get_mut(col) -> &mut Cell`, `cells() -> &[Cell]`
  - `reset(&mut self, template: Cell)` (efface aussi la table annexe et `wrapped`)
  - `set(&mut self, col, cell: Cell)` (efface les combinants de `col`)
  - `push_zerowidth(&mut self, col, ch: char)`, `zerowidth(&self, col) -> Option<&str>`
  - `erase_range(&mut self, range: Range<usize>, template: Cell)`
  - `insert_blank(&mut self, col, n, template)` (décale à droite, tronque), `delete(&mut self, col, n, template)` (décale à gauche, complète)
  - `resize(&mut self, cols, template)`
  - `text(&self) -> String` (caractères de base + combinants, continuations ignorées ; usage tests et copie).

- [ ] **Step 1: Écrire les tests**

Bas de `crates/rustty-vt/src/line.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{Attrs, Style};

    fn styled(c: char) -> Cell {
        Cell::new(c, Style { attrs: Attrs::BOLD, ..Style::default() })
    }

    #[test]
    fn new_line_is_blank() {
        let l = Line::new(4);
        assert_eq!(l.len(), 4);
        assert_eq!(l.text(), "    ");
        assert!(!l.wrapped);
    }

    #[test]
    fn set_and_get_cells() {
        let mut l = Line::new(3);
        l.set(1, styled('a'));
        assert_eq!(l.get(1).c, 'a');
        assert_eq!(l.text(), " a ");
    }

    #[test]
    fn zerowidth_chars_attach_to_a_column_and_follow_text() {
        let mut l = Line::new(3);
        l.set(0, styled('e'));
        l.push_zerowidth(0, '\u{301}');
        assert_eq!(l.zerowidth(0), Some("\u{301}"));
        assert_eq!(l.text(), "e\u{301}  ");
        l.set(0, styled('x'));
        assert_eq!(l.zerowidth(0), None, "écraser la cellule efface ses combinants");
    }

    #[test]
    fn text_skips_wide_continuations() {
        let mut l = Line::new(3);
        l.set(0, Cell::new('漢', Style { attrs: Attrs::WIDE, ..Style::default() }));
        l.set(1, Cell::new(' ', Style { attrs: Attrs::WIDE_CONTINUATION, ..Style::default() }));
        assert_eq!(l.text(), "漢 ");
    }

    #[test]
    fn insert_blank_shifts_right_and_truncates() {
        let mut l = Line::new(4);
        for (i, c) in "abcd".chars().enumerate() {
            l.set(i, styled(c));
        }
        l.push_zerowidth(3, '\u{301}');
        l.insert_blank(1, 2, Cell::default());
        assert_eq!(l.text(), "a  b");
        assert_eq!(l.zerowidth(3), None, "le d et son combinant sont sortis de la ligne");
    }

    #[test]
    fn delete_shifts_left_and_pads_with_template() {
        let mut l = Line::new(4);
        for (i, c) in "abcd".chars().enumerate() {
            l.set(i, styled(c));
        }
        l.push_zerowidth(2, '\u{301}');
        l.delete(1, 2, Cell::default());
        assert_eq!(l.text(), "ad  ");
        assert_eq!(l.zerowidth(2), None);
    }

    #[test]
    fn erase_range_uses_template_and_clears_zerowidth() {
        let mut l = Line::new(4);
        l.set(2, styled('c'));
        l.push_zerowidth(2, '\u{301}');
        let tpl = Cell::erased(Style { bg: crate::Color::Indexed(4), ..Style::default() });
        l.erase_range(1..3, tpl);
        assert_eq!(l.get(2).style.bg, crate::Color::Indexed(4));
        assert_eq!(l.zerowidth(2), None);
        assert_eq!(l.text(), "    ");
    }

    #[test]
    fn resize_truncates_or_pads() {
        let mut l = Line::new(3);
        l.set(2, styled('c'));
        l.push_zerowidth(2, '\u{301}');
        l.resize(2, Cell::default());
        assert_eq!(l.len(), 2);
        assert_eq!(l.zerowidth(2), None);
        l.resize(5, Cell::default());
        assert_eq!(l.text(), "     ");
    }

    #[test]
    fn reset_clears_everything() {
        let mut l = Line::new(2);
        l.set(0, styled('a'));
        l.push_zerowidth(0, '\u{301}');
        l.wrapped = true;
        l.reset(Cell::default());
        assert_eq!(l, Line::new(2));
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: erreur `unresolved import` / module `line` introuvable.

- [ ] **Step 3: Implémenter**

`crates/rustty-vt/src/line.rs` :

```rust
//! Une ligne de la grille : des cellules de largeur fixe plus une table annexe
//! pour les caractères combinants, qui n'occupent aucune cellule.

use std::collections::BTreeMap;
use std::ops::Range;

use crate::cell::Cell;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    cells: Vec<Cell>,
    /// Caractères à largeur nulle attachés à une colonne, dans l'ordre d'arrivée.
    zerowidth: BTreeMap<usize, String>,
    /// Vrai si la ligne a débordé sur la suivante (saut de ligne implicite).
    pub wrapped: bool,
}

impl Line {
    pub fn new(cols: usize) -> Self {
        Self::filled(cols, Cell::default())
    }

    pub fn filled(cols: usize, template: Cell) -> Self {
        Self { cells: vec![template; cols], zerowidth: BTreeMap::new(), wrapped: false }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn get(&self, col: usize) -> &Cell {
        &self.cells[col]
    }

    pub fn get_mut(&mut self, col: usize) -> &mut Cell {
        self.zerowidth.remove(&col);
        &mut self.cells[col]
    }

    pub fn set(&mut self, col: usize, cell: Cell) {
        self.zerowidth.remove(&col);
        self.cells[col] = cell;
    }

    pub fn push_zerowidth(&mut self, col: usize, ch: char) {
        self.zerowidth.entry(col).or_default().push(ch);
    }

    pub fn zerowidth(&self, col: usize) -> Option<&str> {
        self.zerowidth.get(&col).map(String::as_str)
    }

    pub fn reset(&mut self, template: Cell) {
        self.cells.fill(template);
        self.zerowidth.clear();
        self.wrapped = false;
    }

    pub fn erase_range(&mut self, range: Range<usize>, template: Cell) {
        let range = range.start.min(self.len())..range.end.min(self.len());
        self.zerowidth.retain(|col, _| !range.contains(col));
        self.cells[range].fill(template);
    }

    /// Insère `n` cellules `template` en `col`, décale le reste à droite et
    /// tronque ce qui dépasse.
    pub fn insert_blank(&mut self, col: usize, n: usize, template: Cell) {
        let cols = self.len();
        if col >= cols {
            return;
        }
        let n = n.min(cols - col);
        self.cells[col..].rotate_right(n);
        self.cells[col..col + n].fill(template);
        let moved: Vec<(usize, String)> = self
            .zerowidth
            .range(col..)
            .filter(|(c, _)| **c + n < cols)
            .map(|(c, s)| (c + n, s.clone()))
            .collect();
        self.zerowidth.retain(|c, _| *c < col);
        self.zerowidth.extend(moved);
    }

    /// Supprime `n` cellules en `col`, décale le reste à gauche et complète
    /// avec `template`.
    pub fn delete(&mut self, col: usize, n: usize, template: Cell) {
        let cols = self.len();
        if col >= cols {
            return;
        }
        let n = n.min(cols - col);
        self.cells[col..].rotate_left(n);
        self.cells[cols - n..].fill(template);
        let moved: Vec<(usize, String)> =
            self.zerowidth.range(col + n..).map(|(c, s)| (c - n, s.clone())).collect();
        self.zerowidth.retain(|c, _| *c < col);
        self.zerowidth.extend(moved);
    }

    pub fn resize(&mut self, cols: usize, template: Cell) {
        self.cells.resize(cols, template);
        self.zerowidth.retain(|c, _| *c < cols);
    }

    /// Texte de la ligne, combinants inclus, continuations de caractères larges exclues.
    pub fn text(&self) -> String {
        let mut out = String::with_capacity(self.len());
        for (col, cell) in self.cells.iter().enumerate() {
            if cell.is_wide_continuation() {
                continue;
            }
            out.push(cell.c);
            if let Some(zw) = self.zerowidth.get(&col) {
                out.push_str(zw);
            }
        }
        out
    }
}
```

`lib.rs` : ajouter `pub mod line;` et `pub use line::Line;`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Si clippy signale `len_without_is_empty`, `is_empty` est déjà fourni.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.3"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.3 — 2026-10-08 · « Ligne de grille »

- `rustty-vt` : `Line` avec insertion, suppression, effacement, redimensionnement et table annexe des caractères combinants.
```

```bash
git add Cargo.toml crates/rustty-vt/src/lib.rs crates/rustty-vt/src/line.rs CHANGELOG.md
git commit -m "rustty-vt : ligne de grille avec caractères combinants (0.1.0-alpha.3)"
```

---

### Task 4 : Grille avec défilement par région

**Files:**
- Create: `crates/rustty-vt/src/grid.rs`
- Modify: `crates/rustty-vt/src/lib.rs`

**Interfaces:**
- Consumes: `Line`, `Cell`.
- Produces:
  - `pub struct Grid` (Clone, Debug, PartialEq, Eq)
  - `Grid::new(cols, rows) -> Grid`, `cols()`, `rows()`
  - `line(row) -> &Line`, `line_mut(row) -> &mut Line`, `lines() -> &[Line]`
  - `cell(col, row) -> &Cell`, `set_cell(col, row, cell)`
  - `scroll_up(&mut self, top, bottom, n, template) -> Vec<Line>` : fait monter les lignes `top..=bottom` de `n`, remplit le bas avec `template`, **retourne les lignes sorties par le haut dans l'ordre** (le `Term` les pousse dans le scrollback si `top == 0`).
  - `scroll_down(&mut self, top, bottom, n, template)` : fait descendre, remplit le haut.
  - `clear(&mut self, template)`, `resize(&mut self, cols, rows, template)`
  - `text(&self) -> Vec<String>` : une entrée par ligne, blancs de fin supprimés (tests).

- [ ] **Step 1: Écrire les tests**

Bas de `crates/rustty-vt/src/grid.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn grid_with_letters(rows: usize) -> Grid {
        let mut g = Grid::new(3, rows);
        for r in 0..rows {
            let c = char::from(b'a' + u8::try_from(r).unwrap());
            g.set_cell(0, r, Cell::new(c, Default::default()));
        }
        g
    }

    #[test]
    fn new_grid_has_requested_size_and_is_blank() {
        let g = Grid::new(4, 2);
        assert_eq!((g.cols(), g.rows()), (4, 2));
        assert_eq!(g.text(), vec!["", ""]);
    }

    #[test]
    fn scroll_up_whole_screen_returns_evicted_lines() {
        let mut g = grid_with_letters(4);
        let evicted = g.scroll_up(0, 3, 2, Cell::default());
        assert_eq!(evicted.iter().map(Line::text).collect::<Vec<_>>(), vec!["a  ", "b  "]);
        assert_eq!(g.text(), vec!["c", "d", "", ""]);
    }

    #[test]
    fn scroll_up_inside_region_leaves_outside_rows_alone() {
        let mut g = grid_with_letters(5);
        let evicted = g.scroll_up(1, 3, 1, Cell::default());
        assert_eq!(evicted.len(), 1);
        assert_eq!(g.text(), vec!["a", "c", "d", "", "e"]);
    }

    #[test]
    fn scroll_down_inside_region() {
        let mut g = grid_with_letters(5);
        g.scroll_down(1, 3, 1, Cell::default());
        assert_eq!(g.text(), vec!["a", "", "b", "c", "e"]);
    }

    #[test]
    fn scrolling_more_than_region_height_just_clears_it() {
        let mut g = grid_with_letters(3);
        let evicted = g.scroll_up(0, 2, 10, Cell::default());
        assert_eq!(evicted.len(), 3);
        assert_eq!(g.text(), vec!["", "", ""]);
    }

    #[test]
    fn resize_pads_and_truncates_lines_and_rows() {
        let mut g = grid_with_letters(2);
        g.resize(5, 3, Cell::default());
        assert_eq!((g.cols(), g.rows()), (5, 3));
        assert_eq!(g.text(), vec!["a", "b", ""]);
        g.resize(1, 1, Cell::default());
        assert_eq!(g.text(), vec!["a"]);
    }

    #[test]
    fn clear_resets_all_lines() {
        let mut g = grid_with_letters(2);
        g.clear(Cell::default());
        assert_eq!(g, Grid::new(3, 2));
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: erreur de compilation, module `grid` introuvable.

- [ ] **Step 3: Implémenter**

`crates/rustty-vt/src/grid.rs` :

```rust
//! Grille d'écran : `rows` lignes de `cols` cellules. Ne connaît pas le
//! curseur ni le scrollback ; le `Term` orchestre.

use crate::cell::Cell;
use crate::line::Line;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    cols: usize,
    rows: usize,
    lines: Vec<Line>,
}

impl Grid {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows, lines: (0..rows).map(|_| Line::new(cols)).collect() }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub fn line(&self, row: usize) -> &Line {
        &self.lines[row]
    }

    pub fn line_mut(&mut self, row: usize) -> &mut Line {
        &mut self.lines[row]
    }

    pub fn cell(&self, col: usize, row: usize) -> &Cell {
        self.lines[row].get(col)
    }

    pub fn set_cell(&mut self, col: usize, row: usize, cell: Cell) {
        self.lines[row].set(col, cell);
    }

    /// Fait monter `top..=bottom` de `n` lignes. Retourne les lignes sorties
    /// par le haut, de la plus ancienne à la plus récente.
    pub fn scroll_up(&mut self, top: usize, bottom: usize, n: usize, template: Cell) -> Vec<Line> {
        let (top, bottom) = self.clamp_region(top, bottom);
        let height = bottom - top + 1;
        let n = n.min(height);
        if n == 0 {
            return Vec::new();
        }
        let evicted: Vec<Line> = self.lines[top..top + n].to_vec();
        self.lines[top..=bottom].rotate_left(n);
        for line in &mut self.lines[bottom + 1 - n..=bottom] {
            line.reset(template);
        }
        evicted
    }

    /// Fait descendre `top..=bottom` de `n` lignes, remplit le haut avec `template`.
    pub fn scroll_down(&mut self, top: usize, bottom: usize, n: usize, template: Cell) {
        let (top, bottom) = self.clamp_region(top, bottom);
        let height = bottom - top + 1;
        let n = n.min(height);
        if n == 0 {
            return;
        }
        self.lines[top..=bottom].rotate_right(n);
        for line in &mut self.lines[top..top + n] {
            line.reset(template);
        }
    }

    pub fn clear(&mut self, template: Cell) {
        for line in &mut self.lines {
            line.reset(template);
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize, template: Cell) {
        for line in &mut self.lines {
            line.resize(cols, template);
        }
        self.lines.resize_with(rows, || Line::filled(cols, template));
        self.cols = cols;
        self.rows = rows;
    }

    /// Texte de chaque ligne, blancs de fin supprimés. Pour les tests.
    pub fn text(&self) -> Vec<String> {
        self.lines.iter().map(|l| l.text().trim_end().to_string()).collect()
    }

    fn clamp_region(&self, top: usize, bottom: usize) -> (usize, usize) {
        let bottom = bottom.min(self.rows.saturating_sub(1));
        (top.min(bottom), bottom)
    }
}
```

`lib.rs` : `pub mod grid;` et `pub use grid::Grid;`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.4"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.4 — 2026-10-08 · « Grille »

- `rustty-vt` : `Grid` avec défilement par région dans les deux sens, effacement et redimensionnement.
```

```bash
git add Cargo.toml crates/rustty-vt/src/lib.rs crates/rustty-vt/src/grid.rs CHANGELOG.md
git commit -m "rustty-vt : grille avec défilement par région (0.1.0-alpha.4)"
```

---

### Task 5 : Scrollback borné

**Files:**
- Create: `crates/rustty-vt/src/scrollback.rs`
- Modify: `crates/rustty-vt/src/lib.rs`

**Interfaces:**
- Consumes: `Line`.
- Produces:
  - `pub struct Scrollback` ; `Scrollback::new(max_lines: usize)`
  - `push(&mut self, line: Line)` (évince la plus ancienne au-delà de `max_lines` ; `max_lines == 0` ⇒ rien n'est conservé)
  - `extend(&mut self, lines: Vec<Line>)`
  - `len() -> usize`, `is_empty()`, `max_lines() -> usize`
  - `get(&self, idx_from_newest: usize) -> Option<&Line>` : `0` = la ligne la plus récente (juste au-dessus de l'écran)
  - `clear(&mut self)`, `resize_lines(&mut self, cols, template)` (applique `Line::resize` à toutes les lignes).

- [ ] **Step 1: Écrire les tests**

Bas de `crates/rustty-vt/src/scrollback.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;

    fn line(c: char) -> Line {
        let mut l = Line::new(2);
        l.set(0, Cell::new(c, Default::default()));
        l
    }

    #[test]
    fn newest_line_is_index_zero() {
        let mut sb = Scrollback::new(10);
        sb.push(line('a'));
        sb.push(line('b'));
        assert_eq!(sb.len(), 2);
        assert_eq!(sb.get(0).unwrap().text(), "b ");
        assert_eq!(sb.get(1).unwrap().text(), "a ");
        assert!(sb.get(2).is_none());
    }

    #[test]
    fn oldest_lines_are_evicted_beyond_capacity() {
        let mut sb = Scrollback::new(2);
        sb.extend(vec![line('a'), line('b'), line('c')]);
        assert_eq!(sb.len(), 2);
        assert_eq!(sb.get(1).unwrap().text(), "b ");
    }

    #[test]
    fn zero_capacity_keeps_nothing() {
        let mut sb = Scrollback::new(0);
        sb.push(line('a'));
        assert!(sb.is_empty());
    }

    #[test]
    fn resize_lines_applies_to_history() {
        let mut sb = Scrollback::new(5);
        sb.push(line('a'));
        sb.resize_lines(4, Cell::default());
        assert_eq!(sb.get(0).unwrap().len(), 4);
    }

    #[test]
    fn clear_empties_history() {
        let mut sb = Scrollback::new(5);
        sb.push(line('a'));
        sb.clear();
        assert!(sb.is_empty());
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: module `scrollback` introuvable.

- [ ] **Step 3: Implémenter**

`crates/rustty-vt/src/scrollback.rs` :

```rust
//! Historique des lignes sorties par le haut de l'écran principal. Anneau
//! borné ; l'écran alternatif n'en a pas.

use std::collections::VecDeque;

use crate::cell::Cell;
use crate::line::Line;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scrollback {
    lines: VecDeque<Line>,
    max_lines: usize,
}

impl Scrollback {
    pub fn new(max_lines: usize) -> Self {
        Self { lines: VecDeque::new(), max_lines }
    }

    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn push(&mut self, line: Line) {
        if self.max_lines == 0 {
            return;
        }
        if self.lines.len() == self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn extend(&mut self, lines: Vec<Line>) {
        for line in lines {
            self.push(line);
        }
    }

    /// `0` est la ligne la plus récente, c'est-à-dire celle juste au-dessus de l'écran.
    pub fn get(&self, idx_from_newest: usize) -> Option<&Line> {
        let len = self.lines.len();
        if idx_from_newest >= len {
            return None;
        }
        self.lines.get(len - 1 - idx_from_newest)
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn resize_lines(&mut self, cols: usize, template: Cell) {
        for line in &mut self.lines {
            line.resize(cols, template);
        }
    }
}
```

`lib.rs` : `pub mod scrollback;` et `pub use scrollback::Scrollback;`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.5"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.5 — 2026-10-08 · « Scrollback »

- `rustty-vt` : `Scrollback` borné, indexé depuis la ligne la plus récente.
```

```bash
git add Cargo.toml crates/rustty-vt/src/lib.rs crates/rustty-vt/src/scrollback.rs CHANGELOG.md
git commit -m "rustty-vt : scrollback borné (0.1.0-alpha.5)"
```

---

### Task 6 : `Term` — squelette, impression de texte et contrôles C0

**Files:**
- Create: `crates/rustty-vt/src/cursor.rs`
- Create: `crates/rustty-vt/src/modes.rs`
- Create: `crates/rustty-vt/src/charset.rs`
- Create: `crates/rustty-vt/src/term.rs`
- Modify: `crates/rustty-vt/src/lib.rs`
- Modify: `crates/rustty-vt/Cargo.toml` (dépendances `vte`, `unicode-width`)

**Interfaces:**
- Consumes: `Grid`, `Scrollback`, `Line`, `Cell`, `Style`.
- Produces:
  - `pub struct Cursor { pub col: usize, pub row: usize, pub style: Style, pub pending_wrap: bool }`
  - `pub enum CursorShape { Block, Underline, Beam }` (Default = Block)
  - `pub enum MouseMode { None, X10, Normal, ButtonEvent, AnyEvent }`
  - `pub struct Modes { pub app_cursor_keys, pub autowrap, pub cursor_visible, pub cursor_blink, pub origin, pub insert, pub bracketed_paste, pub alt_screen, pub focus_events, pub line_feed_new_line: bool, pub mouse: MouseMode, pub mouse_sgr: bool }` (Default : `autowrap`, `cursor_visible`, `cursor_blink` à `true`, le reste `false`/`None`)
  - `pub enum Charset { Ascii, DecSpecialGraphics }` (Default = Ascii) ; `Charset::map(self, c: char) -> char` (identité pour l'instant, table en tâche 12)
  - `pub enum TermEvent { Title(String), Bell, SetClipboard(String) }`
  - `pub struct Term` avec : `Term::new(cols, rows, scrollback_lines) -> Term`, `input(&mut self, bytes: &[u8])`, `grid() -> &Grid`, `cursor() -> Cursor`, `modes() -> &Modes`, `title() -> &str`, `scrollback() -> &Scrollback`, `drain_events(&mut self) -> Vec<TermEvent>`, `drain_responses(&mut self) -> Vec<u8>`, `text(&self) -> Vec<String>`.
  - En interne : `erase_template(&self) -> Cell`, `linefeed(&mut self)`, `carriage_return`, `backspace`, `horizontal_tab`, `scroll_up_region(n)`, `scroll_down_region(n)`, `active_grid()/active_grid_mut()`.

- [ ] **Step 1: Écrire les tests**

Bas de `crates/rustty-vt/src/term.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn term(cols: usize, rows: usize) -> Term {
        Term::new(cols, rows, 100)
    }

    pub(crate) fn feed(t: &mut Term, s: &str) {
        t.input(s.as_bytes());
    }

    #[test]
    fn prints_text_and_advances_cursor() {
        let mut t = term(10, 2);
        feed(&mut t, "hello");
        assert_eq!(t.text(), vec!["hello", ""]);
        assert_eq!((t.cursor().col, t.cursor().row), (5, 0));
    }

    #[test]
    fn carriage_return_and_line_feed() {
        let mut t = term(10, 3);
        feed(&mut t, "ab\r\ncd\n");
        assert_eq!(t.text(), vec!["ab", "cd", ""]);
        assert_eq!((t.cursor().col, t.cursor().row), (2, 2), "LF seul garde la colonne");
    }

    #[test]
    fn autowrap_marks_line_and_continues_on_next_row() {
        let mut t = term(5, 2);
        feed(&mut t, "abcde");
        assert_eq!((t.cursor().col, t.cursor().row), (4, 0));
        assert!(t.cursor().pending_wrap);
        feed(&mut t, "f");
        assert_eq!(t.text(), vec!["abcde", "f"]);
        assert!(t.grid().line(0).wrapped);
        assert!(!t.cursor().pending_wrap);
    }

    #[test]
    fn carriage_return_cancels_pending_wrap() {
        let mut t = term(5, 2);
        feed(&mut t, "abcde\r\nx");
        assert_eq!(t.text(), vec!["abcde", "x"]);
        assert!(!t.grid().line(0).wrapped);
    }

    #[test]
    fn line_feed_at_bottom_scrolls_into_scrollback() {
        let mut t = term(3, 2);
        feed(&mut t, "a\r\nb\r\nc");
        assert_eq!(t.text(), vec!["b", "c"]);
        assert_eq!(t.scrollback().len(), 1);
        assert_eq!(t.scrollback().get(0).unwrap().text(), "a  ");
    }

    #[test]
    fn backspace_stops_at_first_column() {
        let mut t = term(5, 1);
        feed(&mut t, "ab\x08\x08\x08x");
        assert_eq!(t.text(), vec!["xb"]);
    }

    #[test]
    fn horizontal_tab_goes_to_next_stop_and_stays_at_edge() {
        let mut t = term(20, 1);
        feed(&mut t, "\t");
        assert_eq!(t.cursor().col, 8);
        feed(&mut t, "\t\t\t");
        assert_eq!(t.cursor().col, 19);
    }

    #[test]
    fn bell_emits_an_event() {
        let mut t = term(5, 1);
        feed(&mut t, "\x07");
        assert_eq!(t.drain_events(), vec![TermEvent::Bell]);
        assert!(t.drain_events().is_empty(), "drain vide la file");
    }

    #[test]
    fn autowrap_off_overwrites_last_column() {
        let mut t = term(3, 1);
        t.modes.autowrap = false;
        feed(&mut t, "abcdef");
        assert_eq!(t.text(), vec!["abf"]);
        assert_eq!(t.cursor().col, 2);
    }

    #[test]
    fn combining_char_attaches_to_previous_cell() {
        let mut t = term(5, 1);
        feed(&mut t, "e\u{301}x");
        assert_eq!(t.grid().line(0).zerowidth(0), Some("\u{301}"));
        assert_eq!(t.text(), vec!["e\u{301}x"]);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: module `term` introuvable.

- [ ] **Step 3: Implémenter**

`crates/rustty-vt/Cargo.toml`, `[dependencies]` :

```toml
bitflags.workspace = true
unicode-width.workspace = true
vte.workspace = true
```

`crates/rustty-vt/src/cursor.rs` :

```rust
//! Position et style du curseur d'écriture.

use crate::cell::Style;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    pub col: usize,
    pub row: usize,
    /// État SGR courant, appliqué aux cellules écrites.
    pub style: Style,
    /// Le curseur est « au-delà » de la dernière colonne : le prochain
    /// caractère imprimable passe à la ligne (si autowrap).
    pub pending_wrap: bool,
}

/// Forme demandée par DECSCUSR ; le renderer décide de l'apparence exacte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorShape {
    #[default]
    Block,
    Underline,
    Beam,
}
```

`crates/rustty-vt/src/modes.rs` :

```rust
//! Modes DEC privés et ANSI qui changent l'interprétation du flux ou ce que
//! l'hôte doit envoyer (touches curseur, souris, collage).

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MouseMode {
    #[default]
    None,
    /// 9 : clics seulement, sans relâchement.
    X10,
    /// 1000 : pressions et relâchements.
    Normal,
    /// 1002 : plus les mouvements bouton enfoncé.
    ButtonEvent,
    /// 1003 : tous les mouvements.
    AnyEvent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modes {
    /// DECCKM (1) : flèches en mode application.
    pub app_cursor_keys: bool,
    /// DECAWM (7).
    pub autowrap: bool,
    /// DECTCEM (25).
    pub cursor_visible: bool,
    /// 12.
    pub cursor_blink: bool,
    /// DECOM (6) : positions relatives à la région de défilement.
    pub origin: bool,
    /// IRM (ANSI 4) : insertion au lieu d'écrasement.
    pub insert: bool,
    /// 2004.
    pub bracketed_paste: bool,
    /// 47 / 1047 / 1049.
    pub alt_screen: bool,
    /// 1004.
    pub focus_events: bool,
    /// LNM (ANSI 20) : LF implique CR.
    pub line_feed_new_line: bool,
    pub mouse: MouseMode,
    /// 1006 : encodage SGR des événements souris.
    pub mouse_sgr: bool,
}

impl Default for Modes {
    fn default() -> Self {
        Self {
            app_cursor_keys: false,
            autowrap: true,
            cursor_visible: true,
            cursor_blink: true,
            origin: false,
            insert: false,
            bracketed_paste: false,
            alt_screen: false,
            focus_events: false,
            line_feed_new_line: false,
            mouse: MouseMode::None,
            mouse_sgr: false,
        }
    }
}
```

`crates/rustty-vt/src/charset.rs` :

```rust
//! Jeux de caractères G0/G1. Seul le jeu graphique DEC (lignes de boîte) est
//! pris en charge, c'est celui qu'utilisent encore les applications.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Charset {
    #[default]
    Ascii,
    DecSpecialGraphics,
}

impl Charset {
    /// Traduit un caractère selon le jeu. Table remplie en tâche 12.
    pub fn map(self, c: char) -> char {
        match self {
            Self::Ascii | Self::DecSpecialGraphics => c,
        }
    }
}
```

`crates/rustty-vt/src/term.rs` :

```rust
//! L'état complet d'un terminal et l'interprétation du flux d'octets.
//! `Term` implémente `vte::Perform` ; le parseur est sorti de la structure
//! pendant `input` pour satisfaire l'emprunteur.

use unicode_width::UnicodeWidthChar;
use vte::{Params, Parser, Perform};

use crate::cell::Cell;
use crate::charset::Charset;
use crate::cursor::{Cursor, CursorShape};
use crate::grid::Grid;
use crate::modes::Modes;
use crate::scrollback::Scrollback;

/// Ce que l'interface doit savoir et que l'état seul ne dit pas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermEvent {
    Title(String),
    Bell,
    /// OSC 52 : l'application demande à écrire dans le presse-papiers.
    SetClipboard(String),
}

/// Curseur sauvegardé par DECSC / CSI s, avec le contexte qui l'accompagne.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SavedCursor {
    pub cursor: Cursor,
    pub origin: bool,
    pub charsets: [Charset; 2],
    pub active_charset: usize,
}

pub struct Term {
    pub(crate) grid: Grid,
    pub(crate) alt_grid: Grid,
    pub(crate) scrollback: Scrollback,
    pub(crate) cursor: Cursor,
    pub(crate) saved_cursor: SavedCursor,
    pub(crate) saved_cursor_alt: SavedCursor,
    pub(crate) modes: Modes,
    pub(crate) cursor_shape: CursorShape,
    /// Région de défilement, bornes incluses, indexée à 0.
    pub(crate) scroll_top: usize,
    pub(crate) scroll_bottom: usize,
    pub(crate) tabs: Vec<bool>,
    pub(crate) charsets: [Charset; 2],
    pub(crate) active_charset: usize,
    pub(crate) title: String,
    /// Décalage d'affichage dans le scrollback : 0 = écran vivant.
    pub(crate) display_offset: usize,
    parser: Parser,
    responses: Vec<u8>,
    events: Vec<TermEvent>,
}

impl Term {
    pub fn new(cols: usize, rows: usize, scrollback_lines: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            grid: Grid::new(cols, rows),
            alt_grid: Grid::new(cols, rows),
            scrollback: Scrollback::new(scrollback_lines),
            cursor: Cursor::default(),
            saved_cursor: SavedCursor::default(),
            saved_cursor_alt: SavedCursor::default(),
            modes: Modes::default(),
            cursor_shape: CursorShape::default(),
            scroll_top: 0,
            scroll_bottom: rows - 1,
            tabs: Self::default_tabs(cols),
            charsets: [Charset::Ascii; 2],
            active_charset: 0,
            title: String::new(),
            display_offset: 0,
            parser: Parser::new(),
            responses: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Interprète des octets venus du PTY.
    pub fn input(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::take(&mut self.parser);
        parser.advance(self, bytes);
        self.parser = parser;
    }

    pub fn grid(&self) -> &Grid {
        self.active_grid()
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn cursor_shape(&self) -> CursorShape {
        self.cursor_shape
    }

    pub fn modes(&self) -> &Modes {
        &self.modes
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    pub fn drain_events(&mut self) -> Vec<TermEvent> {
        std::mem::take(&mut self.events)
    }

    /// Octets à renvoyer à l'application (réponses aux requêtes).
    pub fn drain_responses(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.responses)
    }

    /// Texte de l'écran actif, pour les tests.
    pub fn text(&self) -> Vec<String> {
        self.active_grid().text()
    }

    // ----- internes -----

    pub(crate) fn default_tabs(cols: usize) -> Vec<bool> {
        (0..cols).map(|c| c % 8 == 0).collect()
    }

    pub(crate) fn active_grid(&self) -> &Grid {
        if self.modes.alt_screen { &self.alt_grid } else { &self.grid }
    }

    pub(crate) fn active_grid_mut(&mut self) -> &mut Grid {
        if self.modes.alt_screen { &mut self.alt_grid } else { &mut self.grid }
    }

    pub(crate) fn cols(&self) -> usize {
        self.grid.cols()
    }

    pub(crate) fn rows(&self) -> usize {
        self.grid.rows()
    }

    /// Cellule utilisée pour effacer : couleurs courantes, pas d'attribut.
    pub(crate) fn erase_template(&self) -> Cell {
        Cell::erased(self.cursor.style)
    }

    pub(crate) fn push_event(&mut self, ev: TermEvent) {
        self.events.push(ev);
    }

    pub(crate) fn scroll_up_region(&mut self, n: usize) {
        let template = self.erase_template();
        let (top, bottom) = (self.scroll_top, self.scroll_bottom);
        let keep_history = top == 0 && !self.modes.alt_screen;
        let evicted = self.active_grid_mut().scroll_up(top, bottom, n, template);
        if keep_history {
            self.scrollback.extend(evicted);
        }
    }

    pub(crate) fn scroll_down_region(&mut self, n: usize) {
        let template = self.erase_template();
        let (top, bottom) = (self.scroll_top, self.scroll_bottom);
        self.active_grid_mut().scroll_down(top, bottom, n, template);
    }

    pub(crate) fn linefeed(&mut self) {
        self.cursor.pending_wrap = false;
        if self.cursor.row == self.scroll_bottom {
            self.scroll_up_region(1);
        } else if self.cursor.row + 1 < self.rows() {
            self.cursor.row += 1;
        }
        if self.modes.line_feed_new_line {
            self.cursor.col = 0;
        }
    }

    pub(crate) fn carriage_return(&mut self) {
        self.cursor.col = 0;
        self.cursor.pending_wrap = false;
    }

    pub(crate) fn backspace(&mut self) {
        self.cursor.pending_wrap = false;
        self.cursor.col = self.cursor.col.saturating_sub(1);
    }

    pub(crate) fn horizontal_tab(&mut self) {
        self.cursor.pending_wrap = false;
        let cols = self.cols();
        let next = (self.cursor.col + 1..cols).find(|&c| self.tabs[c]);
        self.cursor.col = next.unwrap_or(cols - 1);
    }

    /// Écrit un caractère de largeur 1 à la position du curseur, en gérant
    /// le retour à la ligne différé et le mode insertion.
    fn put_char(&mut self, c: char) {
        let cols = self.cols();
        if self.cursor.pending_wrap {
            if self.modes.autowrap {
                let row = self.cursor.row;
                self.active_grid_mut().line_mut(row).wrapped = true;
                self.carriage_return();
                self.linefeed();
            } else {
                self.cursor.pending_wrap = false;
            }
        }
        let (col, row) = (self.cursor.col, self.cursor.row);
        let cell = Cell::new(c, self.cursor.style);
        let template = self.erase_template();
        let line = self.active_grid_mut().line_mut(row);
        if self.modes.insert {
            line.insert_blank(col, 1, template);
        }
        line.set(col, cell);
        if col + 1 < cols {
            self.cursor.col += 1;
        } else {
            self.cursor.pending_wrap = true;
        }
    }

    /// Attache un caractère combinant à la dernière cellule écrite.
    fn put_zerowidth(&mut self, c: char) {
        let (col, row) = (self.cursor.col, self.cursor.row);
        // Après un caractère en dernière colonne le curseur n'a pas avancé :
        // la cible est la cellule sous le curseur, sinon celle juste avant.
        let target = if self.cursor.pending_wrap || col == 0 { col } else { col - 1 };
        let line = self.active_grid_mut().line_mut(row);
        let target = if line.get(target).is_wide_continuation() { target.saturating_sub(1) } else { target };
        line.push_zerowidth(target, c);
    }
}

impl Perform for Term {
    fn print(&mut self, c: char) {
        let c = self.charsets[self.active_charset].map(c);
        match c.width().unwrap_or(1) {
            0 => self.put_zerowidth(c),
            _ => self.put_char(c),
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x07 => self.push_event(TermEvent::Bell),
            0x08 => self.backspace(),
            0x09 => self.horizontal_tab(),
            0x0A..=0x0C => self.linefeed(),
            0x0D => self.carriage_return(),
            0x0E => self.active_charset = 1,
            0x0F => self.active_charset = 0,
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}
}
```

Les tests du module utilisent `Style` et `Attrs` à partir de la tâche 9 : les importer alors dans `mod tests` (`use crate::cell::{Attrs, Style};`). Si `Parser` n'implémente pas `Default` dans la version de `vte` résolue, remplacer le champ par `parser: Option<Parser>` et faire `self.parser.take().unwrap_or_default()` puis `self.parser = Some(parser)`.

`lib.rs` : ajouter

```rust
pub mod charset;
pub mod cursor;
pub mod modes;
pub mod term;

pub use charset::Charset;
pub use cursor::{Cursor, CursorShape};
pub use modes::{Modes, MouseMode};
pub use term::{Term, TermEvent};
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Si clippy se plaint de `new_without_default`, c'est normal : `Term::new` a des paramètres, pas d'impl `Default` attendue.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.6"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.6 — 2026-10-08 · « Cœur du terminal »

- `rustty-vt` : `Term` piloté par `vte`, impression de texte avec retour à la ligne différé, contrôles C0 (BEL, BS, HT, LF, CR, SO, SI), défilement vers le scrollback, caractères combinants.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-vt/Cargo.toml crates/rustty-vt/src/lib.rs crates/rustty-vt/src/cursor.rs crates/rustty-vt/src/modes.rs crates/rustty-vt/src/charset.rs crates/rustty-vt/src/term.rs CHANGELOG.md
git commit -m "rustty-vt : cœur du terminal, impression et contrôles C0 (0.1.0-alpha.6)"
```

---

### Task 7 : Déplacements du curseur, région de défilement, sauvegarde du curseur

**Files:**
- Create: `crates/rustty-vt/src/csi.rs` (helpers de paramètres)
- Modify: `crates/rustty-vt/src/term.rs` (`csi_dispatch`, `esc_dispatch`, nouvelles méthodes)

**Interfaces:**
- Produces:
  - `pub(crate) fn args(params: &Params) -> Vec<u16>` ; `pub(crate) fn arg_or(p: &[u16], i: usize, default: u16) -> u16` (absent **ou 0** ⇒ `default`) ; `pub(crate) fn raw(p: &[u16], i: usize) -> u16` (absent ⇒ 0).
  - Méthodes `Term` : `cursor_up(n)`, `cursor_down(n)`, `cursor_forward(n)`, `cursor_back(n)`, `cursor_to(col, row)` (absolu, 0-indexé, origin-aware), `set_scroll_region(top, bottom)` (1-indexés, 0 = défaut), `save_cursor()`, `restore_cursor()`.
  - Le `match` de `csi_dispatch` et `esc_dispatch` que les tâches suivantes étendent.

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `term.rs` :

```rust
    #[test]
    fn cup_is_one_indexed_and_clamped() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[3;4H");
        assert_eq!((t.cursor().col, t.cursor().row), (3, 2));
        feed(&mut t, "\x1b[99;99H");
        assert_eq!((t.cursor().col, t.cursor().row), (9, 4));
        feed(&mut t, "\x1b[H");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 0));
    }

    #[test]
    fn csi_zero_and_huge_params_are_clamped() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[2;2H\x1b[0A");
        assert_eq!(t.cursor().row, 0, "0 vaut 1");
        feed(&mut t, "\x1b[99999C");
        assert_eq!(t.cursor().col, 9);
        feed(&mut t, "\x1b[;;H");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 0), "paramètres vides = défauts");
    }

    #[test]
    fn relative_moves() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[3;3H\x1b[B\x1b[2C\x1b[A\x1b[D");
        assert_eq!((t.cursor().col, t.cursor().row), (3, 2));
        feed(&mut t, "\x1b[5G\x1b[2d");
        assert_eq!((t.cursor().col, t.cursor().row), (4, 1), "CHA et VPA");
        feed(&mut t, "\x1b[E\x1b[F");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 1), "CNL puis CPL");
    }

    #[test]
    fn cursor_up_stops_at_scroll_region_top_when_inside() {
        let mut t = term(10, 6);
        feed(&mut t, "\x1b[2;5r\x1b[3;1H\x1b[9A");
        assert_eq!(t.cursor().row, 1);
        feed(&mut t, "\x1b[1;1H\x1b[9B");
        assert_eq!(t.cursor().row, 4, "depuis le dessus de la région, on s'arrête à son bas (xterm)");
        feed(&mut t, "\x1b[6;1H\x1b[9A");
        assert_eq!(t.cursor().row, 1, "depuis le dessous, on s'arrête à son haut");
    }

    #[test]
    fn decstbm_moves_cursor_home_and_scrolls_within_region() {
        let mut t = term(3, 4);
        feed(&mut t, "a\r\nb\r\nc\r\nd\x1b[2;3r");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 0));
        feed(&mut t, "\x1b[3;1H\n");
        assert_eq!(t.text(), vec!["a", "c", "", "d"]);
        assert_eq!(t.scrollback().len(), 0, "une région qui ne touche pas le haut n'alimente pas l'historique");
    }

    #[test]
    fn decstbm_invalid_region_is_ignored() {
        let mut t = term(3, 4);
        feed(&mut t, "\x1b[10;5r");
        assert_eq!((t.scroll_top, t.scroll_bottom), (0, 3));
        feed(&mut t, "\x1b[1;999r");
        assert_eq!((t.scroll_top, t.scroll_bottom), (0, 3), "bas hors écran : borné à l'écran");
        feed(&mut t, "\x1b[3;3r");
        assert_eq!((t.scroll_top, t.scroll_bottom), (0, 3), "région d'une ligne : ignorée");
        feed(&mut t, "\x1b[r");
        assert_eq!((t.scroll_top, t.scroll_bottom), (0, 3));
    }

    #[test]
    fn origin_mode_makes_cup_relative_to_region() {
        let mut t = term(5, 6);
        t.modes.origin = true;
        feed(&mut t, "\x1b[3;5r\x1b[1;1H");
        assert_eq!(t.cursor().row, 2);
        feed(&mut t, "\x1b[99;1H");
        assert_eq!(t.cursor().row, 4, "borné à la région");
    }

    #[test]
    fn save_and_restore_cursor_with_style() {
        let mut t = term(10, 3);
        feed(&mut t, "\x1b[2;3H\x1b7\x1b[1;1Hzz\x1b8");
        assert_eq!((t.cursor().col, t.cursor().row), (2, 1));
        feed(&mut t, "\x1b[s\x1b[3;9H\x1b[u");
        assert_eq!((t.cursor().col, t.cursor().row), (2, 1), "CSI s/u équivalents");
    }

    #[test]
    fn moves_cancel_pending_wrap() {
        let mut t = term(3, 2);
        feed(&mut t, "abc\x1b[Dx");
        assert_eq!(t.text(), vec!["axc", ""]);
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: les nouveaux tests échouent (le curseur ne bouge pas) ; `scroll_top` est accessible car `pub(crate)`.

- [ ] **Step 3: Implémenter**

`crates/rustty-vt/src/csi.rs` :

```rust
//! Lecture des paramètres CSI. Les séquences sont indexées à 1 et « 0 » vaut
//! presque toujours « défaut », sauf pour ED/EL/SGR qui lisent la valeur brute.

use vte::Params;

pub(crate) fn args(params: &Params) -> Vec<u16> {
    params.iter().map(|sub| sub.first().copied().unwrap_or(0)).collect()
}

/// Paramètre `i`, ou `default` s'il est absent ou nul.
pub(crate) fn arg_or(p: &[u16], i: usize, default: u16) -> u16 {
    match p.get(i) {
        Some(&v) if v != 0 => v,
        _ => default,
    }
}

/// Paramètre `i` tel quel, 0 s'il est absent.
pub(crate) fn raw(p: &[u16], i: usize) -> u16 {
    p.get(i).copied().unwrap_or(0)
}
```

Dans `term.rs`, ajouter `use crate::csi::{arg_or, args, raw};` puis les méthodes dans `impl Term` :

```rust
    /// Borne haute pour un déplacement vertical : la région si le curseur y est.
    fn upper_bound(&self) -> usize {
        if self.cursor.row >= self.scroll_top { self.scroll_top } else { 0 }
    }

    fn lower_bound(&self) -> usize {
        if self.cursor.row <= self.scroll_bottom { self.scroll_bottom } else { self.rows() - 1 }
    }

    pub(crate) fn cursor_up(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        let bound = self.upper_bound();
        self.cursor.row = self.cursor.row.saturating_sub(n).max(bound);
    }

    pub(crate) fn cursor_down(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        let bound = self.lower_bound();
        self.cursor.row = self.cursor.row.saturating_add(n).min(bound);
    }

    pub(crate) fn cursor_forward(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        self.cursor.col = self.cursor.col.saturating_add(n).min(self.cols() - 1);
    }

    pub(crate) fn cursor_back(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    /// Position absolue 0-indexée ; en mode origine, `row` est relatif à la
    /// région et y reste borné.
    pub(crate) fn cursor_to(&mut self, col: usize, row: usize) {
        self.cursor.pending_wrap = false;
        let (min_row, max_row) =
            if self.modes.origin { (self.scroll_top, self.scroll_bottom) } else { (0, self.rows() - 1) };
        self.cursor.row = (min_row + row).min(max_row);
        self.cursor.col = col.min(self.cols() - 1);
    }

    /// DECSTBM, paramètres 1-indexés, 0 = défaut. Région invalide : ignorée.
    pub(crate) fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        let rows = self.rows();
        let top = if top == 0 { 1 } else { usize::from(top) };
        let bottom = if bottom == 0 { rows } else { usize::from(bottom).min(rows) };
        if top >= bottom {
            return;
        }
        self.scroll_top = top - 1;
        self.scroll_bottom = bottom - 1;
        self.cursor_to(0, 0);
    }

    pub(crate) fn save_cursor(&mut self) {
        let saved = SavedCursor {
            cursor: self.cursor,
            origin: self.modes.origin,
            charsets: self.charsets,
            active_charset: self.active_charset,
        };
        if self.modes.alt_screen { self.saved_cursor_alt = saved } else { self.saved_cursor = saved }
    }

    pub(crate) fn restore_cursor(&mut self) {
        let saved = if self.modes.alt_screen { self.saved_cursor_alt } else { self.saved_cursor };
        self.cursor = saved.cursor;
        self.cursor.pending_wrap = false;
        self.modes.origin = saved.origin;
        self.charsets = saved.charsets;
        self.active_charset = saved.active_charset;
        self.cursor.row = self.cursor.row.min(self.rows() - 1);
        self.cursor.col = self.cursor.col.min(self.cols() - 1);
    }
```

Remplacer les méthodes vides `csi_dispatch` et `esc_dispatch` de `impl Perform for Term` :

```rust
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        let p = args(params);
        let n = |i: usize| usize::from(arg_or(&p, i, 1));
        match (intermediates, action) {
            ([], 'A') => self.cursor_up(n(0)),
            ([], 'B') | ([], 'e') => self.cursor_down(n(0)),
            ([], 'C') | ([], 'a') => self.cursor_forward(n(0)),
            ([], 'D') => self.cursor_back(n(0)),
            ([], 'E') => {
                self.cursor_down(n(0));
                self.cursor.col = 0;
            }
            ([], 'F') => {
                self.cursor_up(n(0));
                self.cursor.col = 0;
            }
            ([], 'G') | ([], '`') => {
                let row = self.cursor.row;
                self.cursor.pending_wrap = false;
                self.cursor.col = (n(0) - 1).min(self.cols() - 1);
                self.cursor.row = row;
            }
            ([], 'H') | ([], 'f') => self.cursor_to(n(1) - 1, n(0) - 1),
            ([], 'd') => {
                let col = self.cursor.col;
                self.cursor_to(col, n(0) - 1);
            }
            ([], 'r') => self.set_scroll_region(raw(&p, 0), raw(&p, 1)),
            ([], 's') => self.save_cursor(),
            ([], 'u') => self.restore_cursor(),
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (intermediates, byte) {
            ([], b'7') => self.save_cursor(),
            ([], b'8') => self.restore_cursor(),
            _ => {}
        }
    }
```

`lib.rs` : ajouter `mod csi;` (privé).

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.7"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.7 — 2026-10-08 · « Déplacements du curseur »

- `rustty-vt` : CUU/CUD/CUF/CUB/CNL/CPL/CHA/VPA/CUP/HVP, région de défilement DECSTBM, mode origine, sauvegarde et restauration du curseur (DECSC/DECRC, CSI s/u).
```

```bash
git add Cargo.toml crates/rustty-vt/src/lib.rs crates/rustty-vt/src/csi.rs crates/rustty-vt/src/term.rs CHANGELOG.md
git commit -m "rustty-vt : déplacements du curseur et région de défilement (0.1.0-alpha.7)"
```

---

### Task 8 : Effacement et édition (ED, EL, ECH, ICH, DCH, IL, DL, SU, SD)

**Files:**
- Modify: `crates/rustty-vt/src/term.rs`

**Interfaces:**
- Produces: méthodes `Term` : `erase_in_line(mode: u16)`, `erase_in_display(mode: u16)`, `erase_chars(n)`, `insert_blank_chars(n)`, `delete_chars(n)`, `insert_lines(n)`, `delete_lines(n)`. Les arms `K`, `J`, `X`, `@`, `P`, `L`, `M`, `S`, `T` dans `csi_dispatch`.

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `term.rs` :

```rust
    #[test]
    fn erase_in_line_modes() {
        let mut t = term(5, 3);
        feed(&mut t, "abcde\x1b[1;3H\x1b[K");
        assert_eq!(t.text()[0], "ab");
        feed(&mut t, "\x1b[2;1Habcde\x1b[2;3H\x1b[1K");
        assert_eq!(t.text()[1], "   de");
        feed(&mut t, "\x1b[3;1Habcde\x1b[2K");
        assert_eq!(t.text()[2], "");
        assert_eq!(t.cursor().row, 2, "EL ne déplace pas le curseur");
    }

    #[test]
    fn erase_in_display_modes() {
        let mut t = term(3, 3);
        feed(&mut t, "aaa\r\nbbb\r\nccc\x1b[2;2H\x1b[J");
        assert_eq!(t.text(), vec!["aaa", "b", ""]);
        feed(&mut t, "\x1b[1;1Haaa\r\nbbb\r\nccc\x1b[2;2H\x1b[1J");
        assert_eq!(t.text(), vec!["", "  b", "ccc"]);
        feed(&mut t, "\x1b[2J");
        assert_eq!(t.text(), vec!["", "", ""]);
        assert_eq!(t.cursor().row, 1, "ED ne déplace pas le curseur");
    }

    #[test]
    fn erase_in_display_3_clears_scrollback() {
        let mut t = term(3, 1);
        feed(&mut t, "a\r\nb\r\nc");
        assert_eq!(t.scrollback().len(), 2);
        feed(&mut t, "\x1b[3J");
        assert!(t.scrollback().is_empty());
        assert_eq!(t.text(), vec!["c"], "l'écran est intact");
    }

    #[test]
    fn erase_uses_current_background() {
        let mut t = term(3, 1);
        feed(&mut t, "abc\x1b[44m\x1b[2K");
        assert_eq!(t.grid().cell(1, 0).style.bg, crate::Color::Indexed(4));
        assert!(t.grid().cell(1, 0).style.attrs.is_empty());
    }

    #[test]
    fn erase_chars_does_not_shift() {
        let mut t = term(5, 1);
        feed(&mut t, "abcde\x1b[1;2H\x1b[2X");
        assert_eq!(t.text(), vec!["a  de"]);
    }

    #[test]
    fn insert_and_delete_chars_shift_within_line() {
        let mut t = term(5, 1);
        feed(&mut t, "abcde\x1b[1;2H\x1b[2@");
        assert_eq!(t.text(), vec!["a  bc"]);
        feed(&mut t, "\x1b[3P");
        assert_eq!(t.text(), vec!["ac"]);
    }

    #[test]
    fn insert_and_delete_lines_respect_scroll_region() {
        let mut t = term(1, 5);
        feed(&mut t, "a\r\nb\r\nc\r\nd\r\ne\x1b[2;4r\x1b[3;1H\x1b[L");
        assert_eq!(t.text(), vec!["a", "b", "", "c", "e"]);
        feed(&mut t, "\x1b[2M");
        assert_eq!(t.text(), vec!["a", "b", "", "", "e"]);
        assert!(t.scrollback().is_empty(), "DL n'alimente jamais l'historique");
    }

    #[test]
    fn insert_lines_outside_region_is_ignored() {
        let mut t = term(1, 4);
        feed(&mut t, "a\r\nb\r\nc\r\nd\x1b[2;3r\x1b[4;1H\x1b[L");
        assert_eq!(t.text(), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn scroll_up_and_down_commands() {
        let mut t = term(1, 3);
        feed(&mut t, "a\r\nb\r\nc\x1b[S");
        assert_eq!(t.text(), vec!["b", "c", ""]);
        assert_eq!(t.scrollback().len(), 1, "SU sur tout l'écran alimente l'historique");
        feed(&mut t, "\x1b[2T");
        assert_eq!(t.text(), vec!["", "", "b"]);
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: les nouveaux tests échouent (séquences ignorées).

- [ ] **Step 3: Implémenter**

Dans `impl Term` de `term.rs` :

```rust
    pub(crate) fn erase_in_line(&mut self, mode: u16) {
        let (col, row, cols) = (self.cursor.col, self.cursor.row, self.cols());
        let template = self.erase_template();
        let range = match mode {
            0 => col..cols,
            1 => 0..col + 1,
            2 => 0..cols,
            _ => return,
        };
        self.active_grid_mut().line_mut(row).erase_range(range, template);
    }

    pub(crate) fn erase_in_display(&mut self, mode: u16) {
        let (row, rows) = (self.cursor.row, self.rows());
        let template = self.erase_template();
        match mode {
            0 => {
                self.erase_in_line(0);
                for r in row + 1..rows {
                    self.active_grid_mut().line_mut(r).reset(template);
                }
            }
            1 => {
                for r in 0..row {
                    self.active_grid_mut().line_mut(r).reset(template);
                }
                self.erase_in_line(1);
            }
            2 => self.active_grid_mut().clear(template),
            3 => self.scrollback.clear(),
            _ => {}
        }
    }

    pub(crate) fn erase_chars(&mut self, n: usize) {
        let (col, row, cols) = (self.cursor.col, self.cursor.row, self.cols());
        let template = self.erase_template();
        self.active_grid_mut().line_mut(row).erase_range(col..(col + n).min(cols), template);
    }

    pub(crate) fn insert_blank_chars(&mut self, n: usize) {
        let (col, row) = (self.cursor.col, self.cursor.row);
        let template = self.erase_template();
        self.active_grid_mut().line_mut(row).insert_blank(col, n, template);
    }

    pub(crate) fn delete_chars(&mut self, n: usize) {
        let (col, row) = (self.cursor.col, self.cursor.row);
        let template = self.erase_template();
        self.active_grid_mut().line_mut(row).delete(col, n, template);
    }

    fn cursor_in_scroll_region(&self) -> bool {
        (self.scroll_top..=self.scroll_bottom).contains(&self.cursor.row)
    }

    pub(crate) fn insert_lines(&mut self, n: usize) {
        if !self.cursor_in_scroll_region() {
            return;
        }
        let (row, bottom) = (self.cursor.row, self.scroll_bottom);
        let template = self.erase_template();
        self.active_grid_mut().scroll_down(row, bottom, n, template);
    }

    pub(crate) fn delete_lines(&mut self, n: usize) {
        if !self.cursor_in_scroll_region() {
            return;
        }
        let (row, bottom) = (self.cursor.row, self.scroll_bottom);
        let template = self.erase_template();
        // Les lignes supprimées ne vont jamais dans l'historique.
        let _ = self.active_grid_mut().scroll_up(row, bottom, n, template);
    }
```

Dans `csi_dispatch`, ajouter avant `_ => {}` :

```rust
            ([], 'K') => self.erase_in_line(raw(&p, 0)),
            ([], 'J') => self.erase_in_display(raw(&p, 0)),
            ([], 'X') => self.erase_chars(n(0)),
            ([], '@') => self.insert_blank_chars(n(0)),
            ([], 'P') => self.delete_chars(n(0)),
            ([], 'L') => self.insert_lines(n(0)),
            ([], 'M') => self.delete_lines(n(0)),
            ([], 'S') => self.scroll_up_region(n(0)),
            ([], 'T') => self.scroll_down_region(n(0)),
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.8"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.8 — 2026-10-08 · « Effacement et édition »

- `rustty-vt` : ED (dont effacement de l'historique), EL, ECH, ICH, DCH, IL, DL, SU, SD, avec respect de la région de défilement et de la couleur de fond courante.
```

```bash
git add Cargo.toml crates/rustty-vt/src/term.rs CHANGELOG.md
git commit -m "rustty-vt : effacement et édition de lignes et caractères (0.1.0-alpha.8)"
```

---

### Task 9 : Attributs SGR

**Files:**
- Create: `crates/rustty-vt/src/sgr.rs`
- Modify: `crates/rustty-vt/src/term.rs`
- Modify: `crates/rustty-vt/src/lib.rs`

**Interfaces:**
- Produces: `pub(crate) fn apply_sgr(style: &mut Style, params: &Params)` ; arm `m` dans `csi_dispatch`.

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `term.rs` :

```rust
    fn style_at(t: &Term, col: usize) -> Style {
        t.grid().cell(col, 0).style
    }

    #[test]
    fn sgr_basic_attributes_and_reset() {
        let mut t = term(10, 1);
        feed(&mut t, "\x1b[1;3;4;5;7;8;9ma\x1b[0mb\x1b[2mc");
        let a = style_at(&t, 0).attrs;
        for f in [Attrs::BOLD, Attrs::ITALIC, Attrs::UNDERLINE, Attrs::BLINK, Attrs::INVERSE, Attrs::HIDDEN, Attrs::STRIKETHROUGH] {
            assert!(a.contains(f), "{f:?} manquant");
        }
        assert!(style_at(&t, 1).attrs.is_empty());
        assert_eq!(style_at(&t, 2).attrs, Attrs::DIM);
    }

    #[test]
    fn sgr_empty_is_reset() {
        let mut t = term(4, 1);
        feed(&mut t, "\x1b[1;31ma\x1b[mb");
        assert_eq!(style_at(&t, 1), Style::default());
    }

    #[test]
    fn sgr_individual_resets() {
        let mut t = term(10, 1);
        feed(&mut t, "\x1b[1;2;3;4;5;7;8;9m\x1b[22;23;24;25;27;28;29ma");
        assert!(style_at(&t, 0).attrs.is_empty(), "22 retire gras et atténué");
    }

    #[test]
    fn sgr_indexed_colors() {
        let mut t = term(10, 1);
        feed(&mut t, "\x1b[31;42ma\x1b[94;105mb\x1b[39;49mc");
        assert_eq!((style_at(&t, 0).fg, style_at(&t, 0).bg), (crate::Color::Indexed(1), crate::Color::Indexed(2)));
        assert_eq!((style_at(&t, 1).fg, style_at(&t, 1).bg), (crate::Color::Indexed(12), crate::Color::Indexed(13)));
        assert_eq!(style_at(&t, 2), Style::default());
    }

    #[test]
    fn sgr_256_and_truecolor_with_semicolons_and_colons() {
        let mut t = term(10, 1);
        feed(&mut t, "\x1b[38;5;200;48;2;10;20;30ma\x1b[0m\x1b[38:2::1:2:3;48:5:7mb");
        assert_eq!(style_at(&t, 0).fg, crate::Color::Indexed(200));
        assert_eq!(style_at(&t, 0).bg, crate::Color::Rgb(10, 20, 30));
        assert_eq!(style_at(&t, 1).fg, crate::Color::Rgb(1, 2, 3), "forme 38:2::r:g:b avec espace colorimétrique vide");
        assert_eq!(style_at(&t, 1).bg, crate::Color::Indexed(7));
    }

    #[test]
    fn sgr_truncated_extended_color_is_ignored_but_rest_applies() {
        let mut t = term(10, 1);
        feed(&mut t, "\x1b[38;2;10m\x1b[1ma");
        assert_eq!(style_at(&t, 0).fg, crate::Color::Default);
        assert!(style_at(&t, 0).attrs.contains(Attrs::BOLD));
    }

    #[test]
    fn sgr_underline_styles_via_subparams() {
        let mut t = term(10, 1);
        feed(&mut t, "\x1b[4:3ma\x1b[4:0mb");
        assert!(style_at(&t, 0).attrs.contains(Attrs::UNDERLINE));
        assert!(!style_at(&t, 1).attrs.contains(Attrs::UNDERLINE));
    }
```

Ajouter `use crate::cell::{Attrs, Style};` en tête du module de tests si nécessaire (déjà importés dans `term.rs` via `super::*`).

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: les tests SGR échouent (style par défaut partout).

- [ ] **Step 3: Implémenter**

`crates/rustty-vt/src/sgr.rs` :

```rust
//! Select Graphic Rendition : met à jour le style courant. Gère les deux
//! écritures des couleurs étendues (`38;5;n` et `38:5:n`, idem `2`).

use vte::Params;

use crate::cell::{Attrs, Style};
use crate::color::Color;

pub(crate) fn apply_sgr(style: &mut Style, params: &Params) {
    let groups: Vec<&[u16]> = params.iter().collect();
    if groups.is_empty() {
        *style = Style::default();
        return;
    }
    let mut i = 0;
    while i < groups.len() {
        let g = groups[i];
        let code = g.first().copied().unwrap_or(0);
        match code {
            0 => *style = Style::default(),
            1 => style.attrs.insert(Attrs::BOLD),
            2 => style.attrs.insert(Attrs::DIM),
            3 => style.attrs.insert(Attrs::ITALIC),
            4 => match g.get(1) {
                Some(0) => style.attrs.remove(Attrs::UNDERLINE),
                _ => style.attrs.insert(Attrs::UNDERLINE),
            },
            5 | 6 => style.attrs.insert(Attrs::BLINK),
            7 => style.attrs.insert(Attrs::INVERSE),
            8 => style.attrs.insert(Attrs::HIDDEN),
            9 => style.attrs.insert(Attrs::STRIKETHROUGH),
            22 => style.attrs.remove(Attrs::BOLD | Attrs::DIM),
            23 => style.attrs.remove(Attrs::ITALIC),
            24 => style.attrs.remove(Attrs::UNDERLINE),
            25 => style.attrs.remove(Attrs::BLINK),
            27 => style.attrs.remove(Attrs::INVERSE),
            28 => style.attrs.remove(Attrs::HIDDEN),
            29 => style.attrs.remove(Attrs::STRIKETHROUGH),
            30..=37 => style.fg = Color::Indexed(as_u8(code - 30)),
            39 => style.fg = Color::Default,
            40..=47 => style.bg = Color::Indexed(as_u8(code - 40)),
            49 => style.bg = Color::Default,
            90..=97 => style.fg = Color::Indexed(as_u8(code - 90 + 8)),
            100..=107 => style.bg = Color::Indexed(as_u8(code - 100 + 8)),
            38 | 48 => {
                let (color, consumed) = if g.len() > 1 {
                    // Forme `38:2::r:g:b` ou `38:5:n` : tout est dans le groupe.
                    (extended_color(&g[1..], true), 0)
                } else {
                    // Forme `38;2;r;g;b` : les groupes suivants portent la suite.
                    let rest: Vec<u16> = groups[i + 1..].iter().map(|s| s.first().copied().unwrap_or(0)).collect();
                    let (c, used) = match extended_color(&rest, false) {
                        Some(c) => (Some(c), extended_len(&rest)),
                        None => (None, 0),
                    };
                    (c, used)
                };
                if let Some(c) = color {
                    if code == 38 { style.fg = c } else { style.bg = c }
                }
                i += consumed;
            }
            _ => {}
        }
        i += 1;
    }
}

fn as_u8(v: u16) -> u8 {
    u8::try_from(v.min(255)).unwrap_or(255)
}

/// Longueur consommée par une couleur étendue en forme `;` : `5;n` = 2, `2;r;g;b` = 4.
fn extended_len(items: &[u16]) -> usize {
    match items.first() {
        Some(5) => 2,
        Some(2) => 4,
        _ => 0,
    }
}

/// `items` commence au sélecteur (2 ou 5). En forme `:`, `2` peut être suivi
/// d'un identifiant d'espace colorimétrique vide : `2::r:g:b`.
fn extended_color(items: &[u16], colon_form: bool) -> Option<Color> {
    match items.first()? {
        5 => items.get(1).map(|&n| Color::Indexed(as_u8(n))),
        2 => {
            let rgb = if colon_form && items.len() >= 5 { &items[2..5] } else { items.get(1..4)? };
            Some(Color::Rgb(as_u8(rgb[0]), as_u8(rgb[1]), as_u8(rgb[2])))
        }
        _ => None,
    }
}
```

Dans `term.rs`, `csi_dispatch`, avant `_ => {}` :

```rust
            ([], 'm') => crate::sgr::apply_sgr(&mut self.cursor.style, params),
```

`lib.rs` : `mod sgr;`. Dans `mod tests` de `term.rs`, ajouter `use crate::cell::{Attrs, Style};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.9"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.9 — 2026-10-08 · « Attributs SGR »

- `rustty-vt` : gras, atténué, italique, souligné (avec sous-paramètres), clignotant, inversé, caché, barré, couleurs 16, 256 et vraies couleurs dans les deux syntaxes.
```

```bash
git add Cargo.toml crates/rustty-vt/src/lib.rs crates/rustty-vt/src/sgr.rs crates/rustty-vt/src/term.rs CHANGELOG.md
git commit -m "rustty-vt : attributs et couleurs SGR (0.1.0-alpha.9)"
```

---

### Task 10 : Modes DEC et ANSI, écran alternatif, forme du curseur

**Files:**
- Modify: `crates/rustty-vt/src/term.rs`

**Interfaces:**
- Produces: méthodes `Term` : `set_dec_mode(mode: u16, on: bool)`, `set_ansi_mode(mode: u16, on: bool)`, `enter_alt_screen(save_cursor: bool, clear: bool)`, `leave_alt_screen(restore_cursor: bool)`, `set_cursor_shape(param: u16)`. Arms `?h`, `?l`, `h`, `l`, ` q` dans `csi_dispatch`.

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `term.rs` :

```rust
    #[test]
    fn mode_1049_saves_cursor_switches_and_restores() {
        let mut t = term(5, 2);
        feed(&mut t, "main\x1b[?1049h");
        assert!(t.modes().alt_screen);
        assert_eq!(t.text(), vec!["", ""], "l'écran alternatif démarre vide");
        assert_eq!(t.cursor().col, 4, "la position est conservée à l'entrée");
        feed(&mut t, "\x1b[Halt\x1b[?1049l");
        assert!(!t.modes().alt_screen);
        assert_eq!(t.text(), vec!["main", ""]);
        assert_eq!(t.cursor().col, 4, "le curseur est restauré à la sortie");
        feed(&mut t, "\x1b[?1049h");
        assert_eq!(t.text(), vec!["", ""], "une nouvelle entrée repart d'un écran vide");
    }

    #[test]
    fn mode_47_switches_without_saving_cursor() {
        let mut t = term(5, 2);
        feed(&mut t, "ab\x1b[?47h\x1b[2;3Hx\x1b[?47l");
        assert_eq!((t.cursor().col, t.cursor().row), (3, 1));
        assert_eq!(t.text(), vec!["ab", ""]);
    }

    #[test]
    fn alt_screen_never_feeds_scrollback() {
        let mut t = term(1, 1);
        feed(&mut t, "\x1b[?1049ha\r\nb\r\nc\x1b[?1049l");
        assert!(t.scrollback().is_empty());
    }

    #[test]
    fn dec_private_flags() {
        let mut t = term(5, 2);
        feed(&mut t, "\x1b[?1h\x1b[?7l\x1b[?25l\x1b[?12l\x1b[?1004h\x1b[?2004h");
        let m = *t.modes();
        assert!(m.app_cursor_keys && !m.autowrap && !m.cursor_visible && !m.cursor_blink && m.focus_events && m.bracketed_paste);
        feed(&mut t, "\x1b[?1l\x1b[?7h\x1b[?25h\x1b[?12h\x1b[?1004l\x1b[?2004l");
        assert_eq!(*t.modes(), Modes::default());
    }

    #[test]
    fn several_modes_in_one_sequence() {
        let mut t = term(5, 2);
        feed(&mut t, "\x1b[?1;25;2004h");
        assert!(t.modes().app_cursor_keys && t.modes().bracketed_paste && t.modes().cursor_visible);
    }

    #[test]
    fn origin_mode_homes_cursor_inside_region() {
        let mut t = term(5, 5);
        feed(&mut t, "\x1b[2;4r\x1b[4;4H\x1b[?6h");
        assert!(t.modes().origin);
        assert_eq!((t.cursor().col, t.cursor().row), (0, 1));
        feed(&mut t, "\x1b[?6l");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 0));
    }

    #[test]
    fn mouse_modes() {
        let mut t = term(5, 1);
        for (seq, expected) in [
            ("\x1b[?9h", MouseMode::X10),
            ("\x1b[?1000h", MouseMode::Normal),
            ("\x1b[?1002h", MouseMode::ButtonEvent),
            ("\x1b[?1003h", MouseMode::AnyEvent),
            ("\x1b[?1003l", MouseMode::None),
        ] {
            feed(&mut t, seq);
            assert_eq!(t.modes().mouse, expected, "{seq:?}");
        }
        feed(&mut t, "\x1b[?1006h");
        assert!(t.modes().mouse_sgr);
    }

    #[test]
    fn ansi_insert_mode_and_line_feed_new_line() {
        let mut t = term(5, 2);
        feed(&mut t, "abc\x1b[1;1H\x1b[4hX\x1b[4l");
        assert_eq!(t.text(), vec!["Xabc", ""]);
        feed(&mut t, "\x1b[20h\x1b[1;3H\n");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 1), "LNM : LF implique CR");
        feed(&mut t, "\x1b[20l");
        assert!(!t.modes().line_feed_new_line);
    }

    #[test]
    fn cursor_shape_via_decscusr() {
        let mut t = term(5, 1);
        for (seq, shape) in [
            ("\x1b[3 q", CursorShape::Underline),
            ("\x1b[6 q", CursorShape::Beam),
            ("\x1b[1 q", CursorShape::Block),
            ("\x1b[0 q", CursorShape::Block),
            ("\x1b[4 q", CursorShape::Underline),
        ] {
            feed(&mut t, seq);
            assert_eq!(t.cursor_shape(), shape, "{seq:?}");
        }
    }
```

Ajouter `use crate::modes::MouseMode;` et `use crate::cursor::CursorShape;` dans le module de tests si `super::*` ne les apporte pas (ils sont importés en tête de `term.rs`, donc `super::*` suffit).

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: les nouveaux tests échouent.

- [ ] **Step 3: Implémenter**

Dans `impl Term` :

```rust
    pub(crate) fn enter_alt_screen(&mut self, save_cursor: bool, clear: bool) {
        if self.modes.alt_screen {
            return;
        }
        if save_cursor {
            self.save_cursor();
        }
        self.modes.alt_screen = true;
        if clear {
            let template = self.erase_template();
            self.alt_grid.clear(template);
        }
    }

    pub(crate) fn leave_alt_screen(&mut self, restore_cursor: bool) {
        if !self.modes.alt_screen {
            return;
        }
        self.modes.alt_screen = false;
        if restore_cursor {
            self.restore_cursor();
        }
    }

    pub(crate) fn set_dec_mode(&mut self, mode: u16, on: bool) {
        match mode {
            1 => self.modes.app_cursor_keys = on,
            6 => {
                self.modes.origin = on;
                self.cursor_to(0, 0);
            }
            7 => self.modes.autowrap = on,
            9 => self.modes.mouse = if on { MouseMode::X10 } else { MouseMode::None },
            12 => self.modes.cursor_blink = on,
            25 => self.modes.cursor_visible = on,
            47 => {
                if on { self.enter_alt_screen(false, false) } else { self.leave_alt_screen(false) }
            }
            1000 => self.modes.mouse = if on { MouseMode::Normal } else { MouseMode::None },
            1002 => self.modes.mouse = if on { MouseMode::ButtonEvent } else { MouseMode::None },
            1003 => self.modes.mouse = if on { MouseMode::AnyEvent } else { MouseMode::None },
            1004 => self.modes.focus_events = on,
            1006 => self.modes.mouse_sgr = on,
            1047 => {
                if on {
                    self.enter_alt_screen(false, false);
                } else {
                    let template = self.erase_template();
                    self.alt_grid.clear(template);
                    self.leave_alt_screen(false);
                }
            }
            1049 => {
                if on { self.enter_alt_screen(true, true) } else { self.leave_alt_screen(true) }
            }
            2004 => self.modes.bracketed_paste = on,
            _ => {}
        }
    }

    pub(crate) fn set_ansi_mode(&mut self, mode: u16, on: bool) {
        match mode {
            4 => self.modes.insert = on,
            20 => self.modes.line_feed_new_line = on,
            _ => {}
        }
    }

    pub(crate) fn set_cursor_shape(&mut self, param: u16) {
        self.cursor_shape = match param {
            3 | 4 => CursorShape::Underline,
            5 | 6 => CursorShape::Beam,
            _ => CursorShape::Block,
        };
    }
```

Ajouter `use crate::modes::{Modes, MouseMode};` en remplacement de l'import de `Modes`. Dans `csi_dispatch`, avant `_ => {}` :

```rust
            ([b'?'], 'h') => p.iter().for_each(|&m| self.set_dec_mode(m, true)),
            ([b'?'], 'l') => p.iter().for_each(|&m| self.set_dec_mode(m, false)),
            ([], 'h') => p.iter().for_each(|&m| self.set_ansi_mode(m, true)),
            ([], 'l') => p.iter().for_each(|&m| self.set_ansi_mode(m, false)),
            ([b' '], 'q') => self.set_cursor_shape(raw(&p, 0)),
```

Si l'emprunteur refuse `p.iter().for_each(|&m| self.set_dec_mode(...))` (fermeture empruntant `self`), écrire une boucle `for &m in &p { self.set_dec_mode(m, true); }` ; `p` est un `Vec` local, pas un emprunt de `self`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.10"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.10 — 2026-10-08 · « Modes et écran alternatif »

- `rustty-vt` : modes DEC (touches application, origine, autowrap, visibilité et clignotement du curseur, souris X10/normal/bouton/tout, SGR souris, focus, collage encadré), écrans alternatifs 47/1047/1049, modes ANSI insertion et LNM, forme du curseur DECSCUSR.
```

```bash
git add Cargo.toml crates/rustty-vt/src/term.rs CHANGELOG.md
git commit -m "rustty-vt : modes DEC et ANSI, écran alternatif, forme du curseur (0.1.0-alpha.10)"
```

---

### Task 11 : OSC (titre, presse-papiers) et réponses aux requêtes (DA, DSR)

**Files:**
- Modify: `crates/rustty-vt/src/term.rs`
- Modify: `crates/rustty-vt/Cargo.toml` (dépendance `base64`)

**Interfaces:**
- Produces: `osc_dispatch` complet pour 0, 2, 52 ; méthodes `Term` : `set_title(String)`, `respond(&[u8])`, `device_attributes()`, `device_status_report(mode: u16, private: bool)`. Arms `c`, `>c`, `n`, `?n` dans `csi_dispatch`.

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `term.rs` :

```rust
    #[test]
    fn osc_title_with_bel_and_st_terminators() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]0;Hello\x07");
        assert_eq!(t.title(), "Hello");
        feed(&mut t, "\x1b]2;World\x1b\\");
        assert_eq!(t.title(), "World");
        assert_eq!(t.drain_events(), vec![TermEvent::Title("Hello".into()), TermEvent::Title("World".into())]);
    }

    #[test]
    fn osc52_sets_clipboard_from_base64() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]52;c;aGVsbG8=\x07");
        assert_eq!(t.drain_events(), vec![TermEvent::SetClipboard("hello".into())]);
    }

    #[test]
    fn osc52_invalid_base64_is_ignored() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]52;c;!!!not-base64!!!\x07ok");
        assert!(t.drain_events().is_empty());
        assert_eq!(t.text(), vec!["ok"], "le flux continue d'être interprété");
    }

    #[test]
    fn osc52_query_is_not_answered() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]52;c;?\x07");
        assert!(t.drain_events().is_empty());
        assert!(t.drain_responses().is_empty(), "ne jamais divulguer le presse-papiers");
    }

    #[test]
    fn primary_and_secondary_device_attributes() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b[c");
        assert_eq!(t.drain_responses(), b"\x1b[?62;22c".to_vec());
        feed(&mut t, "\x1b[>c");
        assert_eq!(t.drain_responses(), b"\x1b[>1;10;0c".to_vec());
    }

    #[test]
    fn device_status_reports() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[5n");
        assert_eq!(t.drain_responses(), b"\x1b[0n".to_vec());
        feed(&mut t, "\x1b[3;4H\x1b[6n");
        assert_eq!(t.drain_responses(), b"\x1b[3;4R".to_vec());
        feed(&mut t, "\x1b[2;5r\x1b[?6h\x1b[2;1H\x1b[6n");
        assert_eq!(t.drain_responses(), b"\x1b[2;1R".to_vec(), "relatif à la région en mode origine");
        feed(&mut t, "\x1b[?6n");
        assert_eq!(t.drain_responses(), b"\x1b[?2;1R".to_vec());
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: échecs (titre vide, réponses vides).

- [ ] **Step 3: Implémenter**

`crates/rustty-vt/Cargo.toml`, `[dependencies]` : ajouter `base64.workspace = true`.

Dans `impl Term` :

```rust
    pub(crate) fn set_title(&mut self, title: String) {
        self.title.clone_from(&title);
        self.push_event(TermEvent::Title(title));
    }

    pub(crate) fn respond(&mut self, bytes: &[u8]) {
        self.responses.extend_from_slice(bytes);
    }

    /// DA1 : VT220 avec couleurs ANSI (22).
    pub(crate) fn device_attributes(&mut self) {
        self.respond(b"\x1b[?62;22c");
    }

    pub(crate) fn secondary_device_attributes(&mut self) {
        self.respond(b"\x1b[>1;10;0c");
    }

    pub(crate) fn device_status_report(&mut self, mode: u16, private: bool) {
        match mode {
            5 => self.respond(b"\x1b[0n"),
            6 => {
                let row = if self.modes.origin { self.cursor.row.saturating_sub(self.scroll_top) } else { self.cursor.row };
                let prefix = if private { "?" } else { "" };
                let reply = format!("\x1b[{prefix}{};{}R", row + 1, self.cursor.col + 1);
                self.respond(reply.as_bytes());
            }
            _ => {}
        }
    }

    fn handle_osc52(&mut self, params: &[&[u8]]) {
        use base64::Engine as _;
        let Some(data) = params.get(2) else { return };
        if data == b"?" {
            return; // requête de lecture : jamais honorée
        }
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) else { return };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        self.push_event(TermEvent::SetClipboard(text));
    }
```

Remplacer `osc_dispatch` dans `impl Perform for Term` :

```rust
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let Some(code) = params.first() else { return };
        match *code {
            b"0" | b"2" => {
                let title = params.get(1).map(|t| String::from_utf8_lossy(t).into_owned()).unwrap_or_default();
                self.set_title(title);
            }
            b"52" => self.handle_osc52(params),
            _ => {}
        }
    }
```

Dans `csi_dispatch`, avant `_ => {}` :

```rust
            ([], 'c') => self.device_attributes(),
            ([b'>'], 'c') => self.secondary_device_attributes(),
            ([], 'n') => self.device_status_report(raw(&p, 0), false),
            ([b'?'], 'n') => self.device_status_report(raw(&p, 0), true),
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.11"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.11 — 2026-10-08 · « Titre, presse-papiers et requêtes »

- `rustty-vt` : titre de fenêtre (OSC 0/2), écriture du presse-papiers (OSC 52, lecture refusée), réponses DA1, DA2, DSR 5 et 6 (dont la forme DEC privée).
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-vt/Cargo.toml crates/rustty-vt/src/term.rs CHANGELOG.md
git commit -m "rustty-vt : OSC titre et presse-papiers, réponses DA et DSR (0.1.0-alpha.11)"
```

---

### Task 12 : Jeux de caractères, tabulations et séquences ESC (IND, RI, NEL, HTS, RIS)

**Files:**
- Modify: `crates/rustty-vt/src/charset.rs`
- Modify: `crates/rustty-vt/src/term.rs`

**Interfaces:**
- Produces: `Charset::map` avec la table DEC Special Graphics ; méthodes `Term` : `reverse_index()`, `set_tab_stop()`, `clear_tab_stops(mode: u16)`, `reset()`. Arms `esc_dispatch` : `D`, `E`, `H`, `M`, `c`, `( 0`, `( B`, `) 0`, `) B` ; arm `csi_dispatch` : `g`.

- [ ] **Step 1: Écrire les tests**

Dans `charset.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_identity() {
        assert_eq!(Charset::Ascii.map('q'), 'q');
    }

    #[test]
    fn dec_special_graphics_maps_line_drawing() {
        let g = Charset::DecSpecialGraphics;
        assert_eq!(g.map('q'), '─');
        assert_eq!(g.map('x'), '│');
        assert_eq!(g.map('l'), '┌');
        assert_eq!(g.map('k'), '┐');
        assert_eq!(g.map('m'), '└');
        assert_eq!(g.map('j'), '┘');
        assert_eq!(g.map('n'), '┼');
        assert_eq!(g.map('a'), '▒');
        assert_eq!(g.map('A'), 'A', "les lettres hors table sont inchangées");
    }
}
```

Dans `mod tests` de `term.rs` :

```rust
    #[test]
    fn g0_dec_graphics_via_esc_paren_zero() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b(0qx\x1b(Bq");
        assert_eq!(t.text(), vec!["─│q"]);
    }

    #[test]
    fn g1_selected_with_shift_out() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b)0q\x0eq\x0fq");
        assert_eq!(t.text(), vec!["q─q"]);
    }

    #[test]
    fn index_and_reverse_index_scroll_at_region_edges() {
        let mut t = term(1, 3);
        feed(&mut t, "a\r\nb\r\nc\x1bD");
        assert_eq!(t.text(), vec!["b", "c", ""]);
        feed(&mut t, "\x1b[1;1H\x1bM");
        assert_eq!(t.text(), vec!["", "b", "c"]);
    }

    #[test]
    fn next_line_moves_to_first_column_of_next_row() {
        let mut t = term(5, 2);
        feed(&mut t, "abc\x1bEx");
        assert_eq!(t.text(), vec!["abc", "x"]);
    }

    #[test]
    fn tab_stops_can_be_set_and_cleared() {
        let mut t = term(20, 1);
        feed(&mut t, "\x1b[1;4H\x1bH\x1b[1;1H\t");
        assert_eq!(t.cursor().col, 3, "HTS pose un taquet en colonne 4");
        feed(&mut t, "\x1b[g\x1b[1;1H\t");
        assert_eq!(t.cursor().col, 8, "TBC 0 retire le taquet courant seulement");
        feed(&mut t, "\x1b[3g\x1b[1;1H\t");
        assert_eq!(t.cursor().col, 19, "TBC 3 retire tous les taquets");
    }

    #[test]
    fn ris_resets_everything_but_keeps_size() {
        let mut t = term(5, 2);
        feed(&mut t, "abc\x1b[?25l\x1b[31m\x1b[2;2r\x1b]0;T\x07\x1bc");
        assert_eq!(t.text(), vec!["", ""]);
        assert_eq!(*t.modes(), Modes::default());
        assert_eq!(t.cursor(), Cursor::default());
        assert_eq!((t.scroll_top, t.scroll_bottom), (0, 1));
        assert_eq!(t.title(), "");
        assert_eq!((t.grid().cols(), t.grid().rows()), (5, 2));
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: échecs.

- [ ] **Step 3: Implémenter**

`charset.rs`, remplacer `map` :

```rust
    pub fn map(self, c: char) -> char {
        match self {
            Self::Ascii => c,
            Self::DecSpecialGraphics => match c {
                '`' => '◆',
                'a' => '▒',
                'b' => '␉',
                'c' => '␌',
                'd' => '␍',
                'e' => '␊',
                'f' => '°',
                'g' => '±',
                'h' => '␤',
                'i' => '␋',
                'j' => '┘',
                'k' => '┐',
                'l' => '┌',
                'm' => '└',
                'n' => '┼',
                'o' => '⎺',
                'p' => '⎻',
                'q' => '─',
                'r' => '⎼',
                's' => '⎽',
                't' => '├',
                'u' => '┤',
                'v' => '┴',
                'w' => '┬',
                'x' => '│',
                'y' => '≤',
                'z' => '≥',
                '{' => 'π',
                '|' => '≠',
                '}' => '£',
                '~' => '·',
                _ => c,
            },
        }
    }
```

Dans `impl Term` de `term.rs` :

```rust
    /// RI : monte d'une ligne, fait défiler vers le bas en haut de région.
    pub(crate) fn reverse_index(&mut self) {
        self.cursor.pending_wrap = false;
        if self.cursor.row == self.scroll_top {
            self.scroll_down_region(1);
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
    }

    pub(crate) fn set_tab_stop(&mut self) {
        let col = self.cursor.col;
        self.tabs[col] = true;
    }

    pub(crate) fn clear_tab_stops(&mut self, mode: u16) {
        match mode {
            0 => {
                let col = self.cursor.col;
                self.tabs[col] = false;
            }
            3 => self.tabs.fill(false),
            _ => {}
        }
    }

    /// RIS : état initial, taille conservée. Le scrollback est conservé aussi
    /// (xterm le garde ; `CSI 3 J` existe pour l'effacer).
    pub(crate) fn reset(&mut self) {
        let (cols, rows) = (self.cols(), self.rows());
        self.grid = Grid::new(cols, rows);
        self.alt_grid = Grid::new(cols, rows);
        self.cursor = Cursor::default();
        self.saved_cursor = SavedCursor::default();
        self.saved_cursor_alt = SavedCursor::default();
        self.modes = Modes::default();
        self.cursor_shape = CursorShape::default();
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;
        self.tabs = Self::default_tabs(cols);
        self.charsets = [Charset::Ascii; 2];
        self.active_charset = 0;
        self.title.clear();
        self.display_offset = 0;
    }
```

Remplacer `esc_dispatch` :

```rust
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (intermediates, byte) {
            ([], b'7') => self.save_cursor(),
            ([], b'8') => self.restore_cursor(),
            ([], b'D') => self.linefeed(),
            ([], b'E') => {
                self.linefeed();
                self.carriage_return();
            }
            ([], b'H') => self.set_tab_stop(),
            ([], b'M') => self.reverse_index(),
            ([], b'c') => self.reset(),
            ([b'('], b'0') => self.charsets[0] = Charset::DecSpecialGraphics,
            ([b'('], b'B') => self.charsets[0] = Charset::Ascii,
            ([b')'], b'0') => self.charsets[1] = Charset::DecSpecialGraphics,
            ([b')'], b'B') => self.charsets[1] = Charset::Ascii,
            _ => {}
        }
    }
```

Dans `csi_dispatch`, avant `_ => {}` :

```rust
            ([], 'g') => self.clear_tab_stops(raw(&p, 0)),
```

Attention : `linefeed` applique LNM ; pour `ESC D` (IND) ce n'est pas strictement conforme mais sans effet pratique. Si un test futur l'exige, extraire une fonction `index()` sans le traitement LNM.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.12"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.12 — 2026-10-08 · « Jeux de caractères et tabulations »

- `rustty-vt` : jeu graphique DEC pour les lignes de boîte (G0/G1, SI/SO), IND, RI, NEL, taquets de tabulation HTS/TBC, réinitialisation RIS.
```

```bash
git add Cargo.toml crates/rustty-vt/src/charset.rs crates/rustty-vt/src/term.rs CHANGELOG.md
git commit -m "rustty-vt : jeux de caractères, tabulations et séquences ESC (0.1.0-alpha.12)"
```

---

### Task 13 : Caractères larges

**Files:**
- Modify: `crates/rustty-vt/src/term.rs` (`print`, `put_char`)

**Interfaces:**
- Produces: `put_char(c, width: usize)` gère `width == 2` : pose `WIDE` + `WIDE_CONTINUATION`, passe à la ligne si une seule colonne reste, et nettoie la moitié orpheline quand on écrase une moitié de caractère large.

- [ ] **Step 1: Écrire les tests**

Dans `mod tests` de `term.rs` :

```rust
    #[test]
    fn wide_char_takes_two_cells() {
        let mut t = term(5, 1);
        feed(&mut t, "漢a");
        assert!(t.grid().cell(0, 0).is_wide());
        assert!(t.grid().cell(1, 0).is_wide_continuation());
        assert_eq!(t.grid().cell(2, 0).c, 'a');
        assert_eq!(t.text(), vec!["漢a"]);
        assert_eq!(t.cursor().col, 3);
    }

    #[test]
    fn wide_char_at_last_column_wraps() {
        let mut t = term(4, 2);
        feed(&mut t, "abc漢");
        assert_eq!(t.text(), vec!["abc", "漢"]);
        assert_eq!(t.grid().cell(3, 0).c, ' ', "la dernière cellule de la ligne 1 est un blanc");
        assert!(t.grid().line(0).wrapped);
        assert_eq!(t.cursor().col, 2);
    }

    #[test]
    fn wide_char_at_last_column_without_autowrap_is_dropped() {
        let mut t = term(4, 1);
        feed(&mut t, "\x1b[?7labc漢x");
        assert_eq!(t.text(), vec!["abcx"], "le large n'a pas tenu et est abandonné, x prend la dernière colonne");
    }

    #[test]
    fn overwriting_half_of_a_wide_char_clears_the_other_half() {
        let mut t = term(5, 1);
        feed(&mut t, "漢漢\x1b[1;2Hx");
        assert_eq!(t.text(), vec![" x漢"]);
        assert!(!t.grid().cell(0, 0).is_wide());
        feed(&mut t, "\x1b[1;3Hy");
        assert_eq!(t.text(), vec![" xy"]);
        assert!(!t.grid().cell(3, 0).is_wide_continuation());
    }

    #[test]
    fn wide_char_fills_exactly_the_last_two_columns() {
        let mut t = term(4, 1);
        feed(&mut t, "ab漢");
        assert_eq!(t.text(), vec!["ab漢"]);
        assert!(t.cursor().pending_wrap);
    }

    #[test]
    fn combining_char_after_wide_char_attaches_to_its_first_half() {
        let mut t = term(5, 1);
        feed(&mut t, "漢\u{301}");
        assert_eq!(t.grid().line(0).zerowidth(0), Some("\u{301}"));
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: échecs (le large est traité comme étroit).

- [ ] **Step 3: Implémenter**

Remplacer `put_char` et `print` dans `term.rs` :

```rust
    /// Si la cellule `col` est une moitié de caractère large, efface l'autre moitié.
    fn clear_wide_partner(&mut self, col: usize, row: usize) {
        let template = self.erase_template();
        let line = self.active_grid_mut().line_mut(row);
        if line.get(col).is_wide() && col + 1 < line.len() {
            line.set(col + 1, template);
        } else if line.get(col).is_wide_continuation() && col > 0 {
            line.set(col - 1, template);
        }
    }

    /// Écrit un caractère de largeur `width` (1 ou 2) sous le curseur.
    fn put_char(&mut self, c: char, width: usize) {
        let cols = self.cols();
        // Une grille d'une colonne ne peut pas contenir un large : on le traite
        // comme étroit plutôt que d'indexer hors de la ligne.
        let width = if width == 2 && cols < 2 { 1 } else { width };
        if self.cursor.pending_wrap {
            if self.modes.autowrap {
                self.wrap_line();
            } else {
                self.cursor.pending_wrap = false;
            }
        }
        if width == 2 && self.cursor.col + 1 >= cols {
            if !self.modes.autowrap {
                return; // pas de place et pas de retour à la ligne : abandonné
            }
            // Une seule colonne libre : on la blanchit et on passe à la ligne.
            let (col, row) = (self.cursor.col, self.cursor.row);
            let template = self.erase_template();
            self.active_grid_mut().line_mut(row).set(col, template);
            self.wrap_line();
        }
        let (col, row) = (self.cursor.col, self.cursor.row);
        let template = self.erase_template();
        if self.modes.insert {
            self.active_grid_mut().line_mut(row).insert_blank(col, width, template);
        }
        self.clear_wide_partner(col, row);
        if width == 2 {
            self.clear_wide_partner(col + 1, row);
        }
        let mut style = self.cursor.style;
        if width == 2 {
            style.attrs.insert(Attrs::WIDE);
        }
        let line = self.active_grid_mut().line_mut(row);
        line.set(col, Cell::new(c, style));
        if width == 2 {
            let mut cont = Cell::erased(self.cursor.style);
            cont.style.attrs.insert(Attrs::WIDE_CONTINUATION);
            line.set(col + 1, cont);
        }
        if col + width < cols {
            self.cursor.col += width;
        } else {
            self.cursor.col = cols - 1;
            self.cursor.pending_wrap = true;
        }
    }

    /// Retour à la ligne implicite : marque la ligne et descend.
    fn wrap_line(&mut self) {
        let row = self.cursor.row;
        self.active_grid_mut().line_mut(row).wrapped = true;
        self.carriage_return();
        self.linefeed();
    }
```

Et dans `impl Perform for Term` :

```rust
    fn print(&mut self, c: char) {
        let c = self.charsets[self.active_charset].map(c);
        match c.width().unwrap_or(1) {
            0 => self.put_zerowidth(c),
            2 => self.put_char(c, 2),
            _ => self.put_char(c, 1),
        }
    }
```

En tête de `term.rs`, remplacer `use crate::cell::Cell;` par `use crate::cell::{Attrs, Cell};`.

Vérifier que `put_zerowidth` (tâche 6) cible bien la première moitié d'un large : après `漢` en colonnes 0-1 le curseur est en 2, `col - 1 = 1` est une continuation, donc on remonte à 0. C'est le comportement attendu par le dernier test.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`, y compris les anciens tests de la tâche 6.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.13"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.13 — 2026-10-08 · « Caractères larges »

- `rustty-vt` : caractères CJK et emojis sur deux cellules, retour à la ligne en fin de ligne, nettoyage des moitiés orphelines, combinants attachés à la première moitié.
```

```bash
git add Cargo.toml crates/rustty-vt/src/term.rs CHANGELOG.md
git commit -m "rustty-vt : caractères larges sur deux cellules (0.1.0-alpha.13)"
```

---

### Task 14 : Redimensionnement

**Files:**
- Modify: `crates/rustty-vt/src/term.rs`

**Interfaces:**
- Produces: `Term::resize(&mut self, cols: usize, rows: usize)` : redimensionne les deux grilles et l'historique, borne le curseur et les curseurs sauvegardés, réinitialise la région de défilement, recalcule les taquets, annule `pending_wrap` et le décalage d'affichage. Pas de rewrap (v0.2).

- [ ] **Step 1: Écrire les tests**

Dans `mod tests` de `term.rs` :

```rust
    #[test]
    fn resize_grow_keeps_content_and_cursor() {
        let mut t = term(3, 2);
        feed(&mut t, "abc\r\nd");
        t.resize(5, 4);
        assert_eq!(t.text(), vec!["abc", "d", "", ""]);
        assert_eq!((t.cursor().col, t.cursor().row), (1, 1));
        assert_eq!((t.scroll_top, t.scroll_bottom), (0, 3));
    }

    #[test]
    fn resize_shrink_clamps_cursor_and_region() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[2;4r\x1b[4;9H\x1b7");
        t.resize(4, 2);
        assert_eq!((t.cursor().col, t.cursor().row), (3, 1));
        assert_eq!((t.scroll_top, t.scroll_bottom), (0, 1));
        feed(&mut t, "\x1b8");
        assert_eq!((t.cursor().col, t.cursor().row), (3, 1), "le curseur sauvegardé est borné aussi");
        feed(&mut t, "x");
        assert_eq!(t.text(), vec!["", "   x"], "écrire après resize ne panique pas");
    }

    #[test]
    fn resize_applies_to_scrollback_and_alt_screen() {
        let mut t = term(3, 1);
        feed(&mut t, "a\r\nb");
        t.resize(5, 1);
        assert_eq!(t.scrollback().get(0).unwrap().len(), 5);
        feed(&mut t, "\x1b[?1049h");
        assert_eq!((t.grid().cols(), t.grid().rows()), (5, 1));
    }

    #[test]
    fn resize_resets_tabs_and_pending_wrap() {
        let mut t = term(3, 1);
        feed(&mut t, "abc");
        assert!(t.cursor().pending_wrap);
        t.resize(20, 1);
        assert!(!t.cursor().pending_wrap);
        feed(&mut t, "\x1b[1;1H\t\t");
        assert_eq!(t.cursor().col, 16);
    }

    #[test]
    fn resize_to_zero_is_clamped_to_one() {
        let mut t = term(3, 1);
        t.resize(0, 0);
        assert_eq!((t.grid().cols(), t.grid().rows()), (1, 1));
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: `no method named resize`.

- [ ] **Step 3: Implémenter**

Dans `impl Term` (section publique) :

```rust
    /// Nouvelle taille en cellules. Les lignes sont tronquées ou complétées,
    /// pas réenroulées (prévu en v0.2).
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let template = Cell::default();
        self.grid.resize(cols, rows, template);
        self.alt_grid.resize(cols, rows, template);
        self.scrollback.resize_lines(cols, template);
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;
        self.tabs = Self::default_tabs(cols);
        self.display_offset = 0;
        self.cursor.pending_wrap = false;
        self.cursor.col = self.cursor.col.min(cols - 1);
        self.cursor.row = self.cursor.row.min(rows - 1);
        for saved in [&mut self.saved_cursor, &mut self.saved_cursor_alt] {
            saved.cursor.col = saved.cursor.col.min(cols - 1);
            saved.cursor.row = saved.cursor.row.min(rows - 1);
            saved.cursor.pending_wrap = false;
        }
    }
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.14"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.14 — 2026-10-08 · « Redimensionnement »

- `rustty-vt` : `Term::resize` borne curseurs et région, redimensionne grilles et historique, sans rewrap.
```

```bash
git add Cargo.toml crates/rustty-vt/src/term.rs CHANGELOG.md
git commit -m "rustty-vt : redimensionnement du terminal (0.1.0-alpha.14)"
```

---

### Task 15 : Instantané pour le renderer, défilement de l'affichage, fixtures et documentation

**Files:**
- Create: `crates/rustty-vt/src/snapshot.rs`
- Create: `crates/rustty-vt/tests/fixtures.rs`
- Create: `crates/rustty-vt/tests/snapshots/` (généré par `insta`)
- Modify: `crates/rustty-vt/src/term.rs`
- Modify: `crates/rustty-vt/src/lib.rs`
- Modify: `crates/rustty-vt/Cargo.toml` (`insta` en dev-dependency)
- Modify: `README.md`

**Interfaces:**
- Produces:
  - `pub struct Snapshot { pub cols: usize, pub rows: usize, pub lines: Vec<Line>, pub cursor: Option<Cursor> (None si hors viewport ou invisible), pub cursor_shape: CursorShape, pub display_offset: usize, pub scrollback_len: usize, pub title: String }`
  - `Term::snapshot(&self) -> Snapshot` : les `rows` lignes visibles compte tenu de `display_offset`.
  - `Term::scroll_display(&mut self, delta: isize)` (positif = vers l'historique), `Term::scroll_display_to_bottom()`, `Term::display_offset() -> usize`.
  - Toute sortie du programme (`input` qui modifie la grille) **ne** ramène **pas** l'affichage en bas ; c'est le binaire qui décide (option future). En revanche `resize` et RIS remettent à 0.

- [ ] **Step 1: Écrire les tests**

Dans `mod tests` de `term.rs` :

```rust
    #[test]
    fn snapshot_shows_live_screen_by_default() {
        let mut t = term(3, 2);
        feed(&mut t, "a\r\nb");
        let s = t.snapshot();
        assert_eq!(s.lines.iter().map(|l| l.text().trim_end().to_string()).collect::<Vec<_>>(), vec!["a", "b"]);
        assert_eq!(s.cursor.map(|c| (c.col, c.row)), Some((1, 1)));
        assert_eq!((s.display_offset, s.scrollback_len), (0, 0));
    }

    #[test]
    fn scroll_display_reveals_history_and_shifts_or_hides_cursor() {
        let mut t = term(3, 2);
        feed(&mut t, "a\r\nb\r\nc\r\nd\x1b[H");
        assert_eq!(t.scrollback().len(), 2);
        t.scroll_display(1);
        let s = t.snapshot();
        assert_eq!(s.lines.iter().map(|l| l.text().trim_end().to_string()).collect::<Vec<_>>(), vec!["b", "c"]);
        assert_eq!(s.cursor.map(|c| c.row), Some(1), "le curseur, en ligne 0 du vivant, descend d'une ligne à l'écran");
        t.scroll_display(5);
        assert_eq!(t.display_offset(), 2, "borné à l'historique");
        assert_eq!(t.snapshot().lines.iter().map(|l| l.text().trim_end().to_string()).collect::<Vec<_>>(), vec!["a", "b"]);
        assert!(t.snapshot().cursor.is_none(), "curseur hors viewport");
        t.scroll_display(-1);
        assert_eq!(t.display_offset(), 1);
        t.scroll_display_to_bottom();
        assert_eq!(t.display_offset(), 0);
    }

    #[test]
    fn hidden_cursor_is_absent_from_snapshot() {
        let mut t = term(3, 1);
        feed(&mut t, "\x1b[?25l");
        assert!(t.snapshot().cursor.is_none());
    }

    #[test]
    fn alt_screen_ignores_display_offset() {
        let mut t = term(3, 1);
        feed(&mut t, "a\r\nb");
        t.scroll_display(1);
        feed(&mut t, "\x1b[?1049hz");
        assert_eq!(t.snapshot().lines[0].text().trim_end(), "z");
    }
```

`crates/rustty-vt/tests/fixtures.rs` (tests d'intégration, snapshots `insta`) :

```rust
//! Scénarios de bout en bout : une séquence réaliste, la grille attendue
//! figée par insta. `cargo insta review` pour accepter les changements.

use rustty_vt::Term;

fn render(cols: usize, rows: usize, input: &str) -> String {
    let mut t = Term::new(cols, rows, 50);
    t.input(input.as_bytes());
    let mut out = t.text().join("\n");
    let c = t.cursor();
    out.push_str(&format!("\n-- cursor col={} row={}", c.col, c.row));
    out
}

#[test]
fn shell_prompt_with_colors_and_clear_line() {
    insta::assert_snapshot!(render(20, 3, "\x1b[32muser\x1b[0m@host:~$ ls\r\n\x1b[1;34mdir\x1b[0m  file\r\n\x1b[K$ "));
}

#[test]
fn full_screen_app_uses_alt_screen_and_region() {
    insta::assert_snapshot!(render(
        10,
        4,
        "\x1b[?1049h\x1b[H\x1b[2J\x1b[1;1HTitle\x1b[2;4r\x1b[2;1Hl1\r\nl2\r\nl3\r\nl4\x1b[4;1Hstatus"
    ));
}

#[test]
fn box_drawing_with_dec_graphics() {
    insta::assert_snapshot!(render(6, 3, "\x1b(0lqqqqk\r\nx    x\r\nmqqqqj\x1b(B"));
}

#[test]
fn wide_chars_and_wrapping() {
    insta::assert_snapshot!(render(5, 3, "日本語テキスト"));
}

#[test]
fn garbage_input_does_not_panic() {
    let mut t = Term::new(5, 2, 10);
    let noise: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
    t.input(&noise);
    t.input(b"\x1b[999999999;999999999H\x1b[?999999h\x1b]52;c;\x1b\\");
    assert_eq!((t.grid().cols(), t.grid().rows()), (5, 2));
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-vt`
Expected: `no method named snapshot`, et `insta` introuvable.

- [ ] **Step 3: Implémenter**

`crates/rustty-vt/Cargo.toml`, `[dev-dependencies]` : `insta.workspace = true`.

`crates/rustty-vt/src/snapshot.rs` :

```rust
//! Copie figée de ce qui doit être dessiné. Le renderer ne touche jamais
//! `Term` directement : il reçoit un `Snapshot` pris sous verrou.

use crate::cursor::{Cursor, CursorShape};
use crate::line::Line;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub cols: usize,
    pub rows: usize,
    /// Les `rows` lignes visibles, de haut en bas, historique compris.
    pub lines: Vec<Line>,
    /// Position à l'écran, `None` si invisible ou hors de la zone affichée.
    pub cursor: Option<Cursor>,
    pub cursor_shape: CursorShape,
    pub display_offset: usize,
    pub scrollback_len: usize,
    pub title: String,
}
```

Dans `term.rs`, ajouter `use crate::line::Line;` et `use crate::snapshot::Snapshot;`, puis dans `impl Term` (section publique) :

```rust
    pub fn display_offset(&self) -> usize {
        self.display_offset
    }

    /// Décale l'affichage vers l'historique (`delta > 0`) ou vers l'écran vivant.
    pub fn scroll_display(&mut self, delta: isize) {
        let max = self.scrollback.len();
        let current = isize::try_from(self.display_offset).unwrap_or(isize::MAX);
        let wanted = current.saturating_add(delta).max(0);
        self.display_offset = usize::try_from(wanted).unwrap_or(0).min(max);
    }

    pub fn scroll_display_to_bottom(&mut self) {
        self.display_offset = 0;
    }

    pub fn snapshot(&self) -> Snapshot {
        let rows = self.rows();
        let offset = if self.modes.alt_screen { 0 } else { self.display_offset.min(self.scrollback.len()) };
        let grid = self.active_grid();
        let lines = (0..rows)
            .map(|r| {
                if r < offset {
                    // Ligne d'historique : offset-1 est la plus récente affichée en haut.
                    self.scrollback.get(offset - 1 - r).cloned().unwrap_or_else(|| Line::new(grid.cols()))
                } else {
                    grid.line(r - offset).clone()
                }
            })
            .collect();
        let cursor = if self.modes.cursor_visible && self.cursor.row + offset < rows {
            Some(Cursor { row: self.cursor.row + offset, ..self.cursor })
        } else {
            None
        };
        Snapshot {
            cols: grid.cols(),
            rows,
            lines,
            cursor,
            cursor_shape: self.cursor_shape,
            display_offset: offset,
            scrollback_len: self.scrollback.len(),
            title: self.title.clone(),
        }
    }
```

`lib.rs` : `pub mod snapshot;` et `pub use snapshot::Snapshot;`.

Lancer une première fois `cargo test -p rustty-vt` : les tests `insta` échouent en créant des fichiers `.snap.new`. Les inspecter avec `cargo insta review` (installer : `cargo install cargo-insta`) ou, sans l'outil, lire chaque `tests/snapshots/*.snap.new`, vérifier que la grille correspond à l'attendu décrit par le nom du test, puis renommer en `.snap`. Attendu notamment :

- `shell_prompt_with_colors_and_clear_line` : lignes `user@host:~$ ls`, `dir  file`, `$ `, curseur col=2 row=2.
- `box_drawing_with_dec_graphics` : `┌────┐`, `│    │`, `└────┘`.
- `wide_chars_and_wrapping` : `日本`, `語テ`, `キス` puis `ト` sur une 4ᵉ ligne qui n'existe pas : avec 3 lignes la grille a défilé, on attend `語テ`, `キス`, `ト`.

`README.md`, remplacer la section « Statut » par :

```markdown
> Statut : fondations. La crate `rustty-vt` (émulation de terminal pure) est
> fonctionnelle et testée ; les crates layout, config, pty, render et le binaire
> suivent, voir les plans dans `docs/superpowers/plans/`.

## Développement

```bash
cargo test --workspace                       # tous les tests
cargo clippy --workspace --all-targets -- -D warnings
cargo insta review                           # accepter les snapshots de grille modifiés
```
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`, aucun `.snap.new` restant (`git status` ne montre que des `.snap`).

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.15"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.15 — 2026-10-08 · « Instantané et défilement de l'affichage »

- `rustty-vt` : `Snapshot` pour le renderer, défilement de l'affichage dans l'historique, scénarios de bout en bout figés par insta, test de robustesse sur du bruit.
- README : statut et commandes de développement.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-vt/Cargo.toml crates/rustty-vt/src/lib.rs crates/rustty-vt/src/snapshot.rs crates/rustty-vt/src/term.rs crates/rustty-vt/tests/fixtures.rs crates/rustty-vt/tests/snapshots README.md CHANGELOG.md
git commit -m "rustty-vt : instantané, défilement de l'affichage et fixtures (0.1.0-alpha.15)"
git push
```

Après le push, attendre la CI verte sur les trois OS avant de passer au plan suivant (`gh run list --limit 3`, puis `gh run view <id>` ; filtrer par SHA car `--commit` est cassé).

---

## Suite

Plan suivant : `docs/superpowers/plans/2026-10-08-layout-et-config.md` (crates `rustty-layout` et `rustty-config`), à rédiger une fois ce plan exécuté et la CI verte.

