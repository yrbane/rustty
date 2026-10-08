# Plan d'implémentation — `rustty-layout` et `rustty-config`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer deux crates de logique pure : `rustty-layout`, l'arbre d'onglets et de divisions qui produit des rectangles en pixels, et `rustty-config`, le chargement, la validation et les valeurs par défaut de la configuration TOML, raccourcis compris.

**Architecture:** `rustty-layout` sépare la géométrie (`Rect`, `Axis`, `Direction`, `WindowId`), l'arbre binaire récursif (`Node`, privé) et la façade par onglet (`TabLayout`) qui porte le focus et le zoom. `rustty-config` sépare les types de valeurs (`Rgb`), les sections de configuration désérialisées par `serde` avec défauts complets, l'analyse des combinaisons de touches (`KeyCombo`), les actions (`Action`) et la table de raccourcis (`KeyMap`), le chargement avec erreurs positionnées (`ConfigError`), et un fichier d'exemple généré et testé contre les défauts.

**Tech Stack:** Rust 1.96 (edition 2024) ; `serde` + `toml` + `thiserror` + `directories` pour la config ; `proptest` pour les invariants géométriques du layout.

**Spec:** `docs/superpowers/specs/2026-10-08-rustty-design.md`, sections 2.2, 3.2, 3.3, 6.

## Global Constraints

- Code et identifiants en **anglais** ; commentaires, commits, CHANGELOG en **français**. Aucune mention d'assistant dans les commits.
- Chaque commit bumpe `[workspace.package] version` du `Cargo.toml` racine : la version après le commit du plan est `0.1.0-alpha.20`, la tâche 1 produit `alpha.21`, la tâche N produit `alpha.(20+N)`. Entrée en tête de `CHANGELOG.md` au format `## 0.1.0-alpha.N — 2026-10-08 · « titre »`.
- `cargo fmt --all`, puis `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace` doivent passer avant chaque commit, depuis la racine. `rustfmt` fait autorité sur la mise en forme du code de ce plan.
- `git add` ciblé, jamais `-A`.
- Aucune dépendance système ni thread dans les deux crates. `rustty-config` lit un fichier (`std::fs`) uniquement dans `Config::load` ; la surveillance du fichier (`notify`) appartient au binaire, pas à cette crate. Ruling consigné ici : la spec 3.3 place le rechargement à chaud dans `rustty-config` mais la spec 2.2 interdit les dépendances système ; la 2.2 prime, le binaire fera la surveillance et rappellera `Config::load`.
- Un fichier = une responsabilité ; aucun fichier au-delà de ~300 lignes hors tests.
- Clippy strict refuse le code mort : un champ ou une méthode n'apparaît que dans la tâche qui le consomme.

## Review Focus

1. **Division d'une fenêtre minuscule** (bornes de 1×1 px, ou plus petites que l'espace entre panneaux) : aucune panique, des rectangles de taille nulle plutôt qu'un dépassement. Test dans la tâche 2 (`rects_survive_degenerate_bounds`) et propriété dans la tâche 6.
- **Ratio poussé aux extrêmes** par des resize répétés : borné à `[0.1, 0.9]`, les deux panneaux restent visibles. Test dans la tâche 4 (`resize_is_clamped`).
3. **Clé TOML inconnue ou faute de frappe** (`[fonts]`, `opacityy`) : erreur avec numéro de ligne et nom de clé, pas de silence. Test dans la tâche 8 (`unknown_field_is_an_error_with_line`).
4. **Combinaison de touches mal écrite** (`"ctrl+"`, `"ctlr+t"`, `"ctrl+shift+"`) : erreur nommant la combinaison fautive, le reste de la config n'est pas appliqué. Test dans la tâche 9 (`invalid_combos_are_rejected_with_the_offending_text`) et tâche 11.
5. **Fichier de config absent** : défauts silencieux, pas d'erreur ; fichier présent mais illisible (permissions) : erreur `Io` avec le chemin. Test dans la tâche 11 (`missing_file_means_defaults`, `unreadable_file_is_an_io_error`).

## Structure des modules

```
crates/rustty-layout/src/
├── lib.rs           # réexports
├── geometry.rs      # Rect, Axis, Direction, WindowId
├── node.rs          # Node (arbre binaire, privé) : leaves, contains, rects, split, remove, resize, rotate
└── tab_layout.rs    # TabLayout : façade, focus, zoom, neighbor

crates/rustty-config/src/
├── lib.rs           # réexports
├── color.rs         # Rgb + parsing « #rrggbb »
├── sections.rs      # Font, Window, Tabs, CloseButtonStyle, Colors (serde, défauts)
├── keys.rs          # Mods, Key, KeyCombo (FromStr/Display/Deserialize)
├── action.rs        # Action, ResizeDir, et la forme TOML (string ou table)
├── keymap.rs        # KeyMap : défauts + surcharges, resolve
├── error.rs         # ConfigError (positionné)
├── config.rs        # Config : from_str, validate, load, default_path
└── example.rs       # DEFAULT_TOML (fichier d'exemple commenté) + test d'égalité avec Config::default()
docs/rustty.example.toml   # copie de DEFAULT_TOML, régénérée par un test
```

---

### Task 1 : Crate `rustty-layout`, géométrie et onglet à une fenêtre

**Files:**
- Create: `crates/rustty-layout/Cargo.toml`
- Create: `crates/rustty-layout/src/lib.rs`
- Create: `crates/rustty-layout/src/geometry.rs`
- Create: `crates/rustty-layout/src/node.rs`
- Create: `crates/rustty-layout/src/tab_layout.rs`
- Modify: `Cargo.toml` (racine : `proptest` dans `[workspace.dependencies]`)

**Interfaces:**
- Produces:
  - `pub struct Rect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }` ; `Rect::new(x, y, width, height)`, `right()`, `bottom()`, `area() -> u64`, `intersects(&Rect) -> bool`, `contains_rect(&Rect) -> bool`.
  - `pub enum Axis { Horizontal, Vertical }` — **convention kitty** : `Horizontal` = ligne de séparation horizontale, panneaux empilés (premier en haut) ; `Vertical` = côte à côte (premier à gauche).
  - `pub enum Direction { Left, Right, Up, Down }` ; `Direction::axis(self) -> Axis`.
  - `pub struct WindowId(pub u64)` (Copy, Ord, Hash).
  - `pub(crate) enum Node { Leaf(WindowId), Split { axis, ratio: f32, first: Box<Node>, second: Box<Node> } }` avec `leaves(&self, out: &mut Vec<WindowId>)`, `contains(&self, id) -> bool`, `rects(&self, bounds, gap, out: &mut Vec<(WindowId, Rect)>)` (feuille seule pour l'instant : `out.push((id, bounds))`).
  - `pub struct TabLayout` ; `TabLayout::new() -> (TabLayout, WindowId)` (première fenêtre, id 1), `is_empty()`, `windows() -> Vec<WindowId>`, `focused() -> Option<WindowId>`, `focus(&mut self, id) -> bool`, `contains(id) -> bool`, `rects(&self, bounds, gap) -> Vec<(WindowId, Rect)>`.

- [ ] **Step 1: Écrire les tests**

`crates/rustty-layout/src/geometry.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_edges_and_area() {
        let r = Rect::new(10, 20, 30, 40);
        assert_eq!((r.right(), r.bottom()), (40, 60));
        assert_eq!(r.area(), 1200);
    }

    #[test]
    fn rect_intersection_excludes_touching_edges() {
        let a = Rect::new(0, 0, 10, 10);
        assert!(a.intersects(&Rect::new(5, 5, 10, 10)));
        assert!(!a.intersects(&Rect::new(10, 0, 10, 10)), "bord commun : pas de chevauchement");
        assert!(!a.intersects(&Rect::new(0, 10, 10, 10)));
        assert!(!Rect::new(0, 0, 0, 5).intersects(&a), "rectangle vide");
    }

    #[test]
    fn rect_containment() {
        let outer = Rect::new(0, 0, 100, 100);
        assert!(outer.contains_rect(&Rect::new(0, 0, 100, 100)));
        assert!(outer.contains_rect(&Rect::new(10, 10, 20, 20)));
        assert!(!outer.contains_rect(&Rect::new(90, 90, 20, 20)));
    }

    #[test]
    fn direction_maps_to_axis() {
        assert_eq!(Direction::Left.axis(), Axis::Vertical);
        assert_eq!(Direction::Right.axis(), Axis::Vertical);
        assert_eq!(Direction::Up.axis(), Axis::Horizontal);
        assert_eq!(Direction::Down.axis(), Axis::Horizontal);
    }
}
```

`crates/rustty-layout/src/tab_layout.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_layout_has_one_focused_window() {
        let (layout, first) = TabLayout::new();
        assert_eq!(first, WindowId(1));
        assert!(!layout.is_empty());
        assert_eq!(layout.windows(), vec![first]);
        assert_eq!(layout.focused(), Some(first));
        assert!(layout.contains(first));
        assert!(!layout.contains(WindowId(99)));
    }

    #[test]
    fn single_window_takes_the_whole_bounds() {
        let (layout, first) = TabLayout::new();
        let bounds = Rect::new(5, 5, 800, 600);
        assert_eq!(layout.rects(bounds, 4), vec![(first, bounds)]);
    }

    #[test]
    fn focus_only_accepts_known_windows() {
        let (mut layout, first) = TabLayout::new();
        assert!(!layout.focus(WindowId(42)));
        assert_eq!(layout.focused(), Some(first));
        assert!(layout.focus(first));
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cd /home/seb/Dev/rustty && cargo test -p rustty-layout`
Expected: `error: package ID specification 'rustty-layout' did not match any packages`.

- [ ] **Step 3: Implémenter**

`Cargo.toml` racine, `[workspace.dependencies]`, ajouter :

```toml
proptest = "1"
serde = { version = "1", features = ["derive"] }
toml = "0.9"
thiserror = "2"
directories = "6"
```

`crates/rustty-layout/Cargo.toml` :

```toml
[package]
name = "rustty-layout"
description = "Arbre d'onglets et de divisions : du focus aux rectangles en pixels"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]

[dev-dependencies]
proptest.workspace = true

[lints]
workspace = true
```

`crates/rustty-layout/src/lib.rs` :

```rust
//! Géométrie des onglets et des fenêtres : un arbre de divisions par onglet,
//! qui ne sait rien du rendu et produit des rectangles en pixels.

pub mod geometry;
mod node;
pub mod tab_layout;

pub use geometry::{Axis, Direction, Rect, WindowId};
pub use tab_layout::TabLayout;
```

`crates/rustty-layout/src/geometry.rs` :

```rust
//! Types géométriques partagés : rectangles en pixels, axes, directions,
//! identifiants de fenêtre.

/// Rectangle en pixels, origine en haut à gauche.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    pub const fn right(&self) -> u32 {
        self.x + self.width
    }

    pub const fn bottom(&self) -> u32 {
        self.y + self.height
    }

    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Vrai si les intérieurs se chevauchent ; des bords communs ne comptent pas.
    pub const fn intersects(&self, other: &Rect) -> bool {
        self.width > 0
            && self.height > 0
            && other.width > 0
            && other.height > 0
            && self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    pub const fn contains_rect(&self, other: &Rect) -> bool {
        other.x >= self.x && other.y >= self.y && other.right() <= self.right() && other.bottom() <= self.bottom()
    }
}

/// Axe d'une division, convention kitty : `Horizontal` = ligne de séparation
/// horizontale, donc panneaux empilés (premier en haut) ; `Vertical` = côte à
/// côte (premier à gauche).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// L'axe de division qui sépare deux voisins dans cette direction.
    pub const fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Vertical,
            Self::Up | Self::Down => Axis::Horizontal,
        }
    }
}

/// Identifiant d'une fenêtre de terminal au sein d'un onglet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);
```

`crates/rustty-layout/src/node.rs` (version initiale, étendue dans les tâches 2 à 4) :

```rust
//! Arbre binaire des divisions. Récursif et privé : la façade `TabLayout`
//! est la seule à le manipuler.

use crate::geometry::{Axis, Rect, WindowId};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    Leaf(WindowId),
    Split { axis: Axis, ratio: f32, first: Box<Node>, second: Box<Node> },
}

impl Node {
    /// Feuilles de gauche à droite et de haut en bas.
    pub(crate) fn leaves(&self, out: &mut Vec<WindowId>) {
        match self {
            Self::Leaf(id) => out.push(*id),
            Self::Split { first, second, .. } => {
                first.leaves(out);
                second.leaves(out);
            }
        }
    }

    pub(crate) fn contains(&self, id: WindowId) -> bool {
        match self {
            Self::Leaf(leaf) => *leaf == id,
            Self::Split { first, second, .. } => first.contains(id) || second.contains(id),
        }
    }

    /// Rectangle de chaque feuille dans `bounds`, `gap` pixels entre panneaux.
    pub(crate) fn rects(&self, bounds: Rect, gap: u32, out: &mut Vec<(WindowId, Rect)>) {
        match self {
            Self::Leaf(id) => out.push((*id, bounds)),
            Self::Split { .. } => {
                let _ = gap;
                unreachable!("les divisions arrivent en tâche 2")
            }
        }
    }
}
```

`crates/rustty-layout/src/tab_layout.rs` (version initiale) :

```rust
//! Façade d'un onglet : l'arbre de divisions, la fenêtre focalisée et le zoom.

use crate::geometry::{Rect, WindowId};
use crate::node::Node;

#[derive(Clone, Debug, PartialEq)]
pub struct TabLayout {
    root: Option<Node>,
    focused: Option<WindowId>,
    next_id: u64,
}

impl TabLayout {
    /// Un onglet naît avec une fenêtre, focalisée.
    pub fn new() -> (Self, WindowId) {
        let first = WindowId(1);
        (Self { root: Some(Node::Leaf(first)), focused: Some(first), next_id: 2 }, first)
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub fn windows(&self) -> Vec<WindowId> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.leaves(&mut out);
        }
        out
    }

    pub fn focused(&self) -> Option<WindowId> {
        self.focused
    }

    pub fn contains(&self, id: WindowId) -> bool {
        self.root.as_ref().is_some_and(|r| r.contains(id))
    }

    /// Vrai si la fenêtre existe et a pris le focus.
    pub fn focus(&mut self, id: WindowId) -> bool {
        if !self.contains(id) {
            return false;
        }
        self.focused = Some(id);
        true
    }

    pub fn rects(&self, bounds: Rect, gap: u32) -> Vec<(WindowId, Rect)> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.rects(bounds, gap, &mut out);
        }
        out
    }
}
```

Le champ `next_id` n'est lu qu'en tâche 2 ; si clippy le signale en `dead_code`, le retirer ici et l'ajouter en tâche 2 (même règle que pour `rustty-vt`).

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.21"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.21 — 2026-10-08 · « Crate layout »

- `rustty-layout` : géométrie (`Rect`, `Axis`, `Direction`, `WindowId`) et `TabLayout` à une fenêtre avec focus.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-layout CHANGELOG.md
git commit -m "rustty-layout : géométrie et onglet à une fenêtre (0.1.0-alpha.21)"
```

---

### Task 2 : Division et rectangles

**Files:**
- Modify: `crates/rustty-layout/src/node.rs`
- Modify: `crates/rustty-layout/src/tab_layout.rs`

**Interfaces:**
- Produces: `Node::split_leaf(&mut self, target, axis, new_id) -> bool`, `Node::rects` complet ; `TabLayout::split(&mut self, target: WindowId, axis: Axis) -> Option<WindowId>` (nouvelle fenêtre en second, ratio 0.5, prend le focus ; `None` si `target` inconnue).

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `tab_layout.rs` (ajouter `use crate::geometry::Axis;` en tête du module de tests) :

```rust
    fn rect_of(layout: &TabLayout, id: WindowId, bounds: Rect, gap: u32) -> Rect {
        layout.rects(bounds, gap).into_iter().find(|(w, _)| *w == id).map(|(_, r)| r).unwrap()
    }

    #[test]
    fn vertical_split_puts_the_new_window_on_the_right_and_focuses_it() {
        let (mut layout, first) = TabLayout::new();
        let second = layout.split(first, Axis::Vertical).unwrap();
        assert_eq!(second, WindowId(2));
        assert_eq!(layout.windows(), vec![first, second]);
        assert_eq!(layout.focused(), Some(second));
        let bounds = Rect::new(0, 0, 100, 50);
        assert_eq!(rect_of(&layout, first, bounds, 0), Rect::new(0, 0, 50, 50));
        assert_eq!(rect_of(&layout, second, bounds, 0), Rect::new(50, 0, 50, 50));
    }

    #[test]
    fn horizontal_split_stacks_the_new_window_below() {
        let (mut layout, first) = TabLayout::new();
        let second = layout.split(first, Axis::Horizontal).unwrap();
        let bounds = Rect::new(0, 0, 100, 50);
        assert_eq!(rect_of(&layout, first, bounds, 0), Rect::new(0, 0, 100, 25));
        assert_eq!(rect_of(&layout, second, bounds, 0), Rect::new(0, 25, 100, 25));
    }

    #[test]
    fn gap_is_taken_between_panes_and_odd_pixels_go_to_the_second() {
        let (mut layout, first) = TabLayout::new();
        let second = layout.split(first, Axis::Vertical).unwrap();
        let bounds = Rect::new(10, 10, 101, 40);
        assert_eq!(rect_of(&layout, first, bounds, 4), Rect::new(10, 10, 48, 40), "(101-4)*0.5 = 48.5 tronqué à 48, l'impair va au second");
        assert_eq!(rect_of(&layout, second, bounds, 4), Rect::new(62, 10, 49, 40));
    }

    #[test]
    fn nested_splits_divide_the_target_only() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        assert_eq!(layout.windows(), vec![a, b, c]);
        let bounds = Rect::new(0, 0, 100, 100);
        assert_eq!(rect_of(&layout, a, bounds, 0), Rect::new(0, 0, 50, 100));
        assert_eq!(rect_of(&layout, b, bounds, 0), Rect::new(50, 0, 50, 50));
        assert_eq!(rect_of(&layout, c, bounds, 0), Rect::new(50, 50, 50, 50));
    }

    #[test]
    fn splitting_an_unknown_window_does_nothing() {
        let (mut layout, first) = TabLayout::new();
        assert_eq!(layout.split(WindowId(7), Axis::Vertical), None);
        assert_eq!(layout.windows(), vec![first]);
    }

    #[test]
    fn rects_survive_degenerate_bounds() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let _ = layout.split(b, Axis::Horizontal).unwrap();
        for bounds in [Rect::new(0, 0, 1, 1), Rect::new(0, 0, 3, 3), Rect::new(0, 0, 0, 0)] {
            let rects = layout.rects(bounds, 4);
            assert_eq!(rects.len(), 3, "{bounds:?}");
            for (_, r) in &rects {
                assert!(bounds.contains_rect(r), "{r:?} déborde de {bounds:?}");
            }
        }
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-layout`
Expected: `no method named split`.

- [ ] **Step 3: Implémenter**

Dans `node.rs`, remplacer `rects` et ajouter `split_leaf` :

```rust
    /// Rectangle de chaque feuille dans `bounds`, `gap` pixels entre panneaux.
    /// Le pixel impair va au second panneau (troncature) ; des bornes plus petites que `gap`
    /// donnent des rectangles de taille nulle, jamais un dépassement.
    pub(crate) fn rects(&self, bounds: Rect, gap: u32, out: &mut Vec<(WindowId, Rect)>) {
        match self {
            Self::Leaf(id) => out.push((*id, bounds)),
            Self::Split { axis, ratio, first, second } => {
                let (a, b) = split_rect(bounds, *axis, *ratio, gap);
                first.rects(a, gap, out);
                second.rects(b, gap, out);
            }
        }
    }

    /// Remplace la feuille `target` par une division dont elle est le premier
    /// panneau et `new_id` le second. Vrai si `target` a été trouvée.
    pub(crate) fn split_leaf(&mut self, target: WindowId, axis: Axis, new_id: WindowId) -> bool {
        match self {
            Self::Leaf(id) if *id == target => {
                *self = Self::Split {
                    axis,
                    ratio: 0.5,
                    first: Box::new(Self::Leaf(target)),
                    second: Box::new(Self::Leaf(new_id)),
                };
                true
            }
            Self::Leaf(_) => false,
            Self::Split { first, second, .. } => first.split_leaf(target, axis, new_id) || second.split_leaf(target, axis, new_id),
        }
    }
}

/// Coupe `bounds` en deux selon `axis` : le premier panneau reçoit `ratio` de
/// l'espace restant une fois `gap` retiré.
fn split_rect(bounds: Rect, axis: Axis, ratio: f32, gap: u32) -> (Rect, Rect) {
    let total = match axis {
        Axis::Vertical => bounds.width,
        Axis::Horizontal => bounds.height,
    };
    let available = total.saturating_sub(gap);
    // `ratio` est dans [0, 1] et `available` tient dans un f32 sans perte
    // visible à l'échelle d'un écran : la conversion est sûre.
    let first_size = ((available as f32) * ratio).floor().min(available as f32) as u32;
    let second_size = available - first_size;
    let gap = if available == 0 { total } else { gap };
    match axis {
        Axis::Vertical => (
            Rect::new(bounds.x, bounds.y, first_size, bounds.height),
            Rect::new(bounds.x + first_size + gap, bounds.y, second_size, bounds.height),
        ),
        Axis::Horizontal => (
            Rect::new(bounds.x, bounds.y, bounds.width, first_size),
            Rect::new(bounds.x, bounds.y + first_size + gap, bounds.width, second_size),
        ),
    }
}
```

Si clippy signale `cast_precision_loss` ou `cast_possible_truncation` (lints `pedantic`, non activés ici), ignorer : seul `clippy::all` est en vigueur.

Dans `tab_layout.rs`, ajouter `use crate::geometry::Axis;` et la méthode :

```rust
    /// Divise `target` selon `axis` ; la nouvelle fenêtre est le second
    /// panneau (droite ou bas) et prend le focus.
    pub fn split(&mut self, target: WindowId, axis: Axis) -> Option<WindowId> {
        let root = self.root.as_mut()?;
        let new_id = WindowId(self.next_id);
        if !root.split_leaf(target, axis, new_id) {
            return None;
        }
        self.next_id += 1;
        self.focused = Some(new_id);
        Some(new_id)
    }
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Vérifier à la main le cas `Rect::new(0, 0, 3, 3)` avec `gap = 4` : `available = 0`, premier panneau largeur 0 en `x = 0`, second largeur 0 en `x = 0 + 0 + 3 = 3`, dans les bornes.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.22"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.22 — 2026-10-08 · « Divisions »

- `rustty-layout` : `TabLayout::split` horizontal et vertical, rectangles avec ratio et espace entre panneaux, bornes dégénérées sans panique.
```

```bash
git add Cargo.toml crates/rustty-layout CHANGELOG.md
git commit -m "rustty-layout : division et calcul des rectangles (0.1.0-alpha.22)"
```

---

### Task 3 : Fermeture d'une fenêtre

**Files:**
- Modify: `crates/rustty-layout/src/node.rs`
- Modify: `crates/rustty-layout/src/tab_layout.rs`

**Interfaces:**
- Produces: `Node::remove(self, id) -> Option<Node>` (consomme le nœud ; `None` si le nœud était la feuille retirée) ; `Node::first_leaf(&self) -> WindowId` ; `TabLayout::close(&mut self, id) -> bool`. Après fermeture de la fenêtre focalisée, le focus va à la première feuille du panneau frère promu ; fermer la dernière fenêtre vide l'onglet (`is_empty()`, `focused() == None`).

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `tab_layout.rs` :

```rust
    #[test]
    fn closing_a_pane_gives_its_space_to_the_sibling() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        assert!(layout.close(b));
        assert_eq!(layout.windows(), vec![a]);
        assert_eq!(layout.focused(), Some(a));
        let bounds = Rect::new(0, 0, 100, 50);
        assert_eq!(layout.rects(bounds, 0), vec![(a, bounds)]);
    }

    #[test]
    fn closing_a_nested_pane_promotes_the_sibling_subtree() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        assert!(layout.close(a));
        assert_eq!(layout.windows(), vec![b, c]);
        let bounds = Rect::new(0, 0, 100, 100);
        assert_eq!(rect_of(&layout, b, bounds, 0), Rect::new(0, 0, 100, 50));
        assert_eq!(rect_of(&layout, c, bounds, 0), Rect::new(0, 50, 100, 50));
    }

    #[test]
    fn closing_the_focused_pane_moves_focus_to_the_promoted_sibling() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        layout.focus(a);
        assert!(layout.close(a));
        assert_eq!(layout.focused(), Some(b), "première feuille du frère promu");
        assert!(layout.close(b));
        assert_eq!(layout.focused(), Some(c));
    }

    #[test]
    fn closing_an_unfocused_pane_keeps_focus() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        assert!(layout.close(a));
        assert_eq!(layout.focused(), Some(b));
    }

    #[test]
    fn closing_the_last_window_empties_the_tab() {
        let (mut layout, a) = TabLayout::new();
        assert!(layout.close(a));
        assert!(layout.is_empty());
        assert_eq!(layout.focused(), None);
        assert!(layout.windows().is_empty());
        assert!(layout.rects(Rect::new(0, 0, 10, 10), 0).is_empty());
        assert!(!layout.close(a), "déjà fermée");
        assert_eq!(layout.split(a, Axis::Vertical), None);
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-layout`
Expected: `no method named close`.

- [ ] **Step 3: Implémenter**

Dans `node.rs`, ajouter à `impl Node` :

```rust
    pub(crate) fn first_leaf(&self) -> WindowId {
        match self {
            Self::Leaf(id) => *id,
            Self::Split { first, .. } => first.first_leaf(),
        }
    }

    /// Retire la feuille `id`. Rend l'arbre restant, ou `None` si ce nœud
    /// était cette feuille. Un nœud qui ne contient pas `id` est rendu intact.
    pub(crate) fn remove(self, id: WindowId) -> Option<Node> {
        match self {
            Self::Leaf(leaf) if leaf == id => None,
            Self::Leaf(_) => Some(self),
            Self::Split { axis, ratio, first, second } => {
                if first.contains(id) {
                    match first.remove(id) {
                        None => Some(*second),
                        Some(kept) => Some(Self::Split { axis, ratio, first: Box::new(kept), second }),
                    }
                } else if second.contains(id) {
                    match second.remove(id) {
                        None => Some(*first),
                        Some(kept) => Some(Self::Split { axis, ratio, first, second: Box::new(kept) }),
                    }
                } else {
                    Some(Self::Split { axis, ratio, first, second })
                }
            }
        }
    }
```

Dans `tab_layout.rs`, ajouter :

```rust
    /// Ferme `id` ; son panneau frère reprend l'espace. Si `id` avait le focus,
    /// la première feuille du frère promu le reçoit. Vrai si `id` existait.
    pub fn close(&mut self, id: WindowId) -> bool {
        if !self.contains(id) {
            return false;
        }
        let root = self.root.take().expect("contains(id) garantit une racine");
        let sibling_focus = Self::promoted_sibling_first_leaf(&root, id);
        self.root = root.remove(id);
        if self.focused == Some(id) {
            self.focused = sibling_focus;
        }
        true
    }

    /// Première feuille du panneau frère de `id`, c'est-à-dire ce qui prendra
    /// sa place à l'écran. `None` si `id` est la racine.
    fn promoted_sibling_first_leaf(node: &Node, id: WindowId) -> Option<WindowId> {
        match node {
            Node::Leaf(_) => None,
            Node::Split { first, second, .. } => {
                if **first == Node::Leaf(id) {
                    Some(second.first_leaf())
                } else if **second == Node::Leaf(id) {
                    Some(first.first_leaf())
                } else if first.contains(id) {
                    Self::promoted_sibling_first_leaf(first, id)
                } else {
                    Self::promoted_sibling_first_leaf(second, id)
                }
            }
        }
    }
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.23"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.23 — 2026-10-08 · « Fermeture de fenêtre »

- `rustty-layout` : `TabLayout::close` promeut le panneau frère et transfère le focus ; fermer la dernière fenêtre vide l'onglet.
```

```bash
git add Cargo.toml crates/rustty-layout CHANGELOG.md
git commit -m "rustty-layout : fermeture d'une fenêtre et promotion du frère (0.1.0-alpha.23)"
```

---

### Task 4 : Redimensionnement et rotation

**Files:**
- Modify: `crates/rustty-layout/src/node.rs`
- Modify: `crates/rustty-layout/src/tab_layout.rs`

**Interfaces:**
- Produces: `Node::resize(&mut self, id, axis, delta: f32) -> bool` (agit sur la division **la plus proche** de `id` ayant cet axe ; `delta > 0` agrandit le panneau qui contient `id` ; ratio borné à `[0.1, 0.9]`), `Node::rotate(&mut self, id) -> bool` (inverse l'axe de la division la plus proche) ; `TabLayout::resize(&mut self, id, axis, delta) -> bool`, `TabLayout::rotate(&mut self, id) -> bool`. Constantes `pub const MIN_RATIO: f32 = 0.1; pub const MAX_RATIO: f32 = 0.9;` dans `node.rs` (réexportées par `lib.rs`).

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `tab_layout.rs` :

```rust
    #[test]
    fn resize_grows_the_pane_containing_the_window() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let bounds = Rect::new(0, 0, 100, 10);
        assert!(layout.resize(a, Axis::Vertical, 0.1));
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 60);
        assert!(layout.resize(b, Axis::Vertical, 0.3));
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 30, "agrandir b réduit a");
    }

    #[test]
    fn resize_targets_the_nearest_split_with_that_axis() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        let bounds = Rect::new(0, 0, 100, 100);
        assert!(layout.resize(c, Axis::Vertical, 0.2), "c n'a pas de division verticale directe : on remonte");
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 30);
        assert_eq!(rect_of(&layout, c, bounds, 0).width, 70);
        assert!(layout.resize(c, Axis::Horizontal, 0.2));
        assert_eq!(rect_of(&layout, c, bounds, 0).height, 70);
        assert_eq!(rect_of(&layout, b, bounds, 0).height, 30);
    }

    #[test]
    fn resize_is_clamped() {
        let (mut layout, a) = TabLayout::new();
        let _ = layout.split(a, Axis::Vertical).unwrap();
        let bounds = Rect::new(0, 0, 100, 10);
        for _ in 0..20 {
            layout.resize(a, Axis::Vertical, 0.1);
        }
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 90);
        for _ in 0..40 {
            layout.resize(a, Axis::Vertical, -0.1);
        }
        assert_eq!(rect_of(&layout, a, bounds, 0).width, 10);
    }

    #[test]
    fn resize_without_a_matching_split_or_unknown_window_does_nothing() {
        let (mut layout, a) = TabLayout::new();
        assert!(!layout.resize(a, Axis::Vertical, 0.1), "une seule fenêtre");
        let _ = layout.split(a, Axis::Vertical).unwrap();
        assert!(!layout.resize(a, Axis::Horizontal, 0.1), "pas de division horizontale");
        assert!(!layout.resize(WindowId(9), Axis::Vertical, 0.1));
    }

    #[test]
    fn rotate_flips_the_nearest_split() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        let bounds = Rect::new(0, 0, 100, 100);
        assert!(layout.rotate(c));
        assert_eq!(rect_of(&layout, b, bounds, 0), Rect::new(50, 0, 25, 100), "b et c passent côte à côte");
        assert_eq!(rect_of(&layout, c, bounds, 0), Rect::new(75, 0, 25, 100));
        assert_eq!(rect_of(&layout, a, bounds, 0), Rect::new(0, 0, 50, 100), "la division racine n'a pas bougé");
        let (mut single, s) = TabLayout::new();
        assert!(!single.rotate(s));
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-layout`
Expected: `no method named resize` / `rotate`.

- [ ] **Step 3: Implémenter**

Dans `node.rs`, ajouter les constantes après les `use` :

```rust
/// Un panneau ne peut pas descendre sous 10 % de l'espace de sa division.
pub const MIN_RATIO: f32 = 0.1;
pub const MAX_RATIO: f32 = 0.9;
```

et à `impl Node` :

```rust
    /// Ajuste la division la plus proche de `id` ayant l'axe `axis` : `delta`
    /// positif agrandit le côté qui contient `id`. Faux si aucune ne convient.
    pub(crate) fn resize(&mut self, id: WindowId, axis: Axis, delta: f32) -> bool {
        let Self::Split { axis: own_axis, ratio, first, second } = self else { return false };
        let in_first = first.contains(id);
        if !in_first && !second.contains(id) {
            return false;
        }
        let child = if in_first { first } else { second };
        if child.resize(id, axis, delta) {
            return true;
        }
        if *own_axis != axis {
            return false;
        }
        let signed = if in_first { delta } else { -delta };
        *ratio = (*ratio + signed).clamp(MIN_RATIO, MAX_RATIO);
        true
    }

    /// Inverse l'axe de la division la plus proche de `id`.
    pub(crate) fn rotate(&mut self, id: WindowId) -> bool {
        let Self::Split { axis, first, second, .. } = self else { return false };
        let in_first = first.contains(id);
        if !in_first && !second.contains(id) {
            return false;
        }
        let child = if in_first { first } else { second };
        if child.rotate(id) {
            return true;
        }
        *axis = match axis {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        };
        true
    }
```

Dans `tab_layout.rs` :

```rust
    /// Agrandit (`delta > 0`) ou réduit le panneau de `id` le long de `axis`.
    pub fn resize(&mut self, id: WindowId, axis: Axis, delta: f32) -> bool {
        self.root.as_mut().is_some_and(|r| r.resize(id, axis, delta))
    }

    /// Inverse l'orientation de la division la plus proche de `id`.
    pub fn rotate(&mut self, id: WindowId) -> bool {
        self.root.as_mut().is_some_and(|r| r.rotate(id))
    }
```

`lib.rs` : `pub use node::{MAX_RATIO, MIN_RATIO};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Dans `resize_targets_the_nearest_split_with_that_axis`, après `resize(c, Vertical, 0.2)` la racine passe à 0.3 : `a` = 30 px, le sous-arbre `b/c` = 70 px.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.24"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.24 — 2026-10-08 · « Redimensionnement et rotation »

- `rustty-layout` : `resize` sur la division la plus proche de l'axe demandé, ratio borné à 10–90 % ; `rotate` inverse l'orientation d'une division.
```

```bash
git add Cargo.toml crates/rustty-layout CHANGELOG.md
git commit -m "rustty-layout : redimensionnement borné et rotation (0.1.0-alpha.24)"
```

---

### Task 5 : Voisin dans une direction

**Files:**
- Modify: `crates/rustty-layout/src/tab_layout.rs`

**Interfaces:**
- Produces: `TabLayout::neighbor(&self, id, direction) -> Option<WindowId>` : la fenêtre adjacente dans cette direction qui partage le plus grand segment de bord avec `id` ; à égalité, la plus haute (ou la plus à gauche). Calcul géométrique sur une grille virtuelle 10 000 × 10 000 sans espace entre panneaux.

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `tab_layout.rs` (ajouter `use crate::geometry::Direction;`) :

```rust
    #[test]
    fn neighbor_in_a_simple_split() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        assert_eq!(layout.neighbor(a, Direction::Right), Some(b));
        assert_eq!(layout.neighbor(b, Direction::Left), Some(a));
        assert_eq!(layout.neighbor(a, Direction::Left), None);
        assert_eq!(layout.neighbor(a, Direction::Up), None);
        assert_eq!(layout.neighbor(a, Direction::Down), None);
    }

    #[test]
    fn neighbor_picks_the_pane_sharing_the_longest_edge() {
        // a | b   avec b découpé en b (haut, 70 %) et c (bas, 30 %), puis a
        //   | c   découpé en a (haut) et d (bas) à 50 % : d touche b et c.
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        layout.resize(b, Axis::Horizontal, 0.2);
        let d = layout.split(a, Axis::Horizontal).unwrap();
        assert_eq!(layout.neighbor(a, Direction::Right), Some(b));
        assert_eq!(layout.neighbor(d, Direction::Right), Some(c), "d (50-100 %) recouvre c (70-100 %) sur 30 % et b sur 20 %");
        assert_eq!(layout.neighbor(c, Direction::Left), Some(d));
        assert_eq!(layout.neighbor(b, Direction::Left), Some(a), "b (0-70 %) recouvre a sur 50 % et d sur 20 %");
        assert_eq!(layout.neighbor(b, Direction::Down), Some(c));
        assert_eq!(layout.neighbor(c, Direction::Up), Some(b));
    }

    #[test]
    fn neighbor_ties_go_to_the_topmost_or_leftmost() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let c = layout.split(b, Axis::Horizontal).unwrap();
        assert_eq!(layout.neighbor(a, Direction::Right), Some(b), "b et c partagent 50 % chacun : le plus haut gagne");
        let _ = c;
    }

    #[test]
    fn neighbor_of_unknown_or_single_window_is_none() {
        let (layout, a) = TabLayout::new();
        assert_eq!(layout.neighbor(a, Direction::Right), None);
        assert_eq!(layout.neighbor(WindowId(5), Direction::Right), None);
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-layout`
Expected: `no method named neighbor`.

- [ ] **Step 3: Implémenter**

Dans `tab_layout.rs`, ajouter `use crate::geometry::Direction;` et :

```rust
    /// Côté de la grille virtuelle utilisée pour les calculs de voisinage.
    const VIRTUAL_SIDE: u32 = 10_000;

    /// Fenêtre adjacente à `id` dans `direction`, celle qui partage le plus
    /// long bord ; à égalité la plus haute ou la plus à gauche.
    pub fn neighbor(&self, id: WindowId, direction: Direction) -> Option<WindowId> {
        let rects = self.rects(Rect::new(0, 0, Self::VIRTUAL_SIDE, Self::VIRTUAL_SIDE), 0);
        let (_, from) = *rects.iter().find(|(w, _)| *w == id)?;
        rects
            .iter()
            .filter(|(w, _)| *w != id)
            .filter_map(|(w, r)| {
                let adjacent = match direction {
                    Direction::Left => r.right() == from.x,
                    Direction::Right => r.x == from.right(),
                    Direction::Up => r.bottom() == from.y,
                    Direction::Down => r.y == from.bottom(),
                };
                let overlap = match direction.axis() {
                    Axis::Vertical => overlap_1d(from.y, from.bottom(), r.y, r.bottom()),
                    Axis::Horizontal => overlap_1d(from.x, from.right(), r.x, r.right()),
                };
                (adjacent && overlap > 0).then_some((overlap, std::cmp::Reverse((r.y, r.x)), *w))
            })
            .max_by_key(|(overlap, pos, _)| (*overlap, *pos))
            .map(|(_, _, w)| w)
    }
```

et, hors de l'`impl`, en bas du fichier avant les tests :

```rust
/// Longueur du recouvrement de `[a0, a1)` et `[b0, b1)`.
fn overlap_1d(a0: u32, a1: u32, b0: u32, b1: u32) -> u32 {
    a1.min(b1).saturating_sub(a0.max(b0))
}
```

Le `std::cmp::Reverse((y, x))` fait gagner, à recouvrement égal, le rectangle au plus petit `(y, x)` sous `max_by_key`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.25"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.25 — 2026-10-08 · « Voisinage »

- `rustty-layout` : `neighbor` choisit la fenêtre adjacente partageant le plus long bord, pour les déplacements de focus au clavier.
```

```bash
git add Cargo.toml crates/rustty-layout CHANGELOG.md
git commit -m "rustty-layout : recherche du voisin dans une direction (0.1.0-alpha.25)"
```

---

### Task 6 : Zoom et invariants par propriétés

**Files:**
- Modify: `crates/rustty-layout/src/tab_layout.rs`
- Create: `crates/rustty-layout/tests/invariants.rs`

**Interfaces:**
- Produces: `TabLayout::toggle_zoom(&mut self, id) -> bool`, `TabLayout::zoomed() -> Option<WindowId>`. Zoomé : `rects` ne rend que cette fenêtre sur tout `bounds` ; `split` et `close` de la fenêtre zoomée annulent le zoom.

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `tab_layout.rs` :

```rust
    #[test]
    fn zoom_shows_only_one_window_full_size_and_toggles_back() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        let bounds = Rect::new(0, 0, 100, 50);
        assert!(layout.toggle_zoom(a));
        assert_eq!(layout.zoomed(), Some(a));
        assert_eq!(layout.rects(bounds, 4), vec![(a, bounds)]);
        assert_eq!(layout.windows(), vec![a, b], "les fenêtres existent toujours");
        assert!(layout.toggle_zoom(a));
        assert_eq!(layout.zoomed(), None);
        assert_eq!(layout.rects(bounds, 4).len(), 2);
    }

    #[test]
    fn zooming_another_window_moves_the_zoom() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        layout.toggle_zoom(a);
        assert!(layout.toggle_zoom(b));
        assert_eq!(layout.zoomed(), Some(b));
        assert!(!layout.toggle_zoom(WindowId(9)));
    }

    #[test]
    fn split_and_close_cancel_the_zoom() {
        let (mut layout, a) = TabLayout::new();
        let b = layout.split(a, Axis::Vertical).unwrap();
        layout.toggle_zoom(a);
        let _ = layout.split(a, Axis::Horizontal).unwrap();
        assert_eq!(layout.zoomed(), None, "diviser rompt le zoom");
        layout.toggle_zoom(b);
        layout.close(b);
        assert_eq!(layout.zoomed(), None, "fermer la fenêtre zoomée rompt le zoom");
        layout.toggle_zoom(a);
        layout.close(layout.windows()[1]);
        assert_eq!(layout.zoomed(), Some(a), "fermer une autre fenêtre conserve le zoom");
    }
```

`crates/rustty-layout/tests/invariants.rs` :

```rust
//! Propriétés géométriques : quelle que soit la suite d'opérations, les
//! rectangles couvrent les bornes sans se chevaucher et chaque fenêtre en a un.

use proptest::prelude::*;
use rustty_layout::{Axis, Direction, Rect, TabLayout, WindowId};

#[derive(Clone, Debug)]
enum Op {
    Split(usize, Axis),
    Close(usize),
    Resize(usize, Axis, f32),
    Rotate(usize),
    Zoom(usize),
    Focus(usize, Direction),
}

fn op() -> impl Strategy<Value = Op> {
    let axis = prop_oneof![Just(Axis::Horizontal), Just(Axis::Vertical)];
    let dir = prop_oneof![Just(Direction::Left), Just(Direction::Right), Just(Direction::Up), Just(Direction::Down)];
    prop_oneof![
        (0..8usize, axis.clone()).prop_map(|(i, a)| Op::Split(i, a)),
        (0..8usize).prop_map(Op::Close),
        (0..8usize, axis, -0.5f32..0.5).prop_map(|(i, a, d)| Op::Resize(i, a, d)),
        (0..8usize).prop_map(Op::Rotate),
        (0..8usize).prop_map(Op::Zoom),
        (0..8usize, dir).prop_map(|(i, d)| Op::Focus(i, d)),
    ]
}

/// Choisit une fenêtre existante à partir d'un index arbitraire.
fn pick(layout: &TabLayout, i: usize) -> Option<WindowId> {
    let windows = layout.windows();
    (!windows.is_empty()).then(|| windows[i % windows.len()])
}

fn apply(layout: &mut TabLayout, op: &Op) {
    match *op {
        Op::Split(i, axis) => {
            if let Some(w) = pick(layout, i) {
                layout.split(w, axis);
            }
        }
        Op::Close(i) => {
            if let Some(w) = pick(layout, i) {
                layout.close(w);
            }
        }
        Op::Resize(i, axis, delta) => {
            if let Some(w) = pick(layout, i) {
                layout.resize(w, axis, delta);
            }
        }
        Op::Rotate(i) => {
            if let Some(w) = pick(layout, i) {
                layout.rotate(w);
            }
        }
        Op::Zoom(i) => {
            if let Some(w) = pick(layout, i) {
                layout.toggle_zoom(w);
            }
        }
        Op::Focus(i, dir) => {
            if let Some(w) = pick(layout, i) {
                if let Some(n) = layout.neighbor(w, dir) {
                    layout.focus(n);
                }
            }
        }
    }
}

proptest! {
    #[test]
    fn rects_cover_bounds_without_overlap(ops in prop::collection::vec(op(), 0..40), w in 0u32..300, h in 0u32..300, gap in 0u32..6) {
        let (mut layout, _) = TabLayout::new();
        for o in &ops {
            apply(&mut layout, o);
        }
        let bounds = Rect::new(7, 11, w, h);
        let rects = layout.rects(bounds, gap);
        let visible: Vec<WindowId> = match layout.zoomed() {
            Some(z) => vec![z],
            None => layout.windows(),
        };
        prop_assert_eq!(rects.iter().map(|(id, _)| *id).collect::<Vec<_>>(), visible, "une entrée par fenêtre visible, dans l'ordre");
        for (i, (_, a)) in rects.iter().enumerate() {
            prop_assert!(bounds.contains_rect(a), "{:?} déborde de {:?}", a, bounds);
            for (_, b) in &rects[i + 1..] {
                prop_assert!(!a.intersects(b), "{:?} chevauche {:?}", a, b);
            }
        }
        if gap == 0 {
            let total: u64 = rects.iter().map(|(_, r)| r.area()).sum();
            prop_assert_eq!(total, bounds.area(), "sans espace, les aires se somment exactement");
        }
        if let Some(f) = layout.focused() {
            prop_assert!(layout.contains(f), "le focus pointe toujours une fenêtre existante");
        } else {
            prop_assert!(layout.is_empty());
        }
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-layout`
Expected: `no method named toggle_zoom` / `zoomed`.

- [ ] **Step 3: Implémenter**

Dans `tab_layout.rs`, ajouter le champ `zoomed: Option<WindowId>` à la structure (initialisé à `None` dans `new`), puis :

```rust
    pub fn zoomed(&self) -> Option<WindowId> {
        self.zoomed
    }

    /// Zoome `id` sur tout l'onglet, ou rétablit le layout si `id` l'était déjà.
    pub fn toggle_zoom(&mut self, id: WindowId) -> bool {
        if !self.contains(id) {
            return false;
        }
        self.zoomed = if self.zoomed == Some(id) { None } else { Some(id) };
        true
    }
```

Modifier `rects` :

```rust
    pub fn rects(&self, bounds: Rect, gap: u32) -> Vec<(WindowId, Rect)> {
        if let Some(z) = self.zoomed {
            return vec![(z, bounds)];
        }
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.rects(bounds, gap, &mut out);
        }
        out
    }
```

Dans `split`, après `self.focused = Some(new_id);` ajouter `self.zoomed = None;`. Dans `close`, après `if !self.contains(id) { return false; }` ajouter `if self.zoomed == Some(id) { self.zoomed = None; }`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`, dont `rects_cover_bounds_without_overlap` (256 cas par défaut). Si proptest trouve un contre-exemple, il l'écrit dans `crates/rustty-layout/proptest-regressions/` : corriger le code (pas le test) et versionner ce fichier.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.26"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.26 — 2026-10-08 · « Zoom et invariants »

- `rustty-layout` : zoom d'une fenêtre sur tout l'onglet ; tests par propriétés garantissant couverture sans chevauchement, une entrée par fenêtre et focus toujours valide, quelle que soit la suite d'opérations.
```

```bash
git add Cargo.toml crates/rustty-layout CHANGELOG.md
git commit -m "rustty-layout : zoom et tests par propriétés (0.1.0-alpha.26)"
```

---

### Task 7 : Crate `rustty-config` et couleurs

**Files:**
- Create: `crates/rustty-config/Cargo.toml`
- Create: `crates/rustty-config/src/lib.rs`
- Create: `crates/rustty-config/src/color.rs`

**Interfaces:**
- Produces: `pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }` (Copy, Eq, Hash, Default = noir) ; `Rgb::new(r, g, b)` ; `FromStr` acceptant `#rrggbb` (casse indifférente) et refusant tout le reste avec `ColorParseError(String)` ; `Display` → `#rrggbb` minuscule ; `Deserialize` depuis une chaîne TOML.

- [ ] **Step 1: Écrire les tests**

`crates/rustty-config/src/color.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_six_digit_hex_in_any_case() {
        assert_eq!("#1e1e2e".parse::<Rgb>().unwrap(), Rgb::new(0x1e, 0x1e, 0x2e));
        assert_eq!("#FFAA00".parse::<Rgb>().unwrap(), Rgb::new(255, 170, 0));
    }

    #[test]
    fn rejects_other_shapes_with_the_offending_text() {
        for bad in ["1e1e2e", "#fff", "#12345", "#1234567", "#gg0000", "", "rouge"] {
            let err = bad.parse::<Rgb>().unwrap_err();
            assert!(err.to_string().contains(bad), "{err}");
        }
    }

    #[test]
    fn displays_as_lowercase_hex() {
        assert_eq!(Rgb::new(255, 170, 0).to_string(), "#ffaa00");
    }

    #[test]
    fn deserializes_from_a_toml_string() {
        #[derive(serde::Deserialize)]
        struct Doc {
            c: Rgb,
        }
        let d: Doc = toml::from_str("c = \"#89b4fa\"").unwrap();
        assert_eq!(d.c, Rgb::new(0x89, 0xb4, 0xfa));
        let err = toml::from_str::<Doc>("c = \"#zz\"").unwrap_err();
        assert!(err.to_string().contains("#zz"));
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-config`
Expected: `did not match any packages`.

- [ ] **Step 3: Implémenter**

`crates/rustty-config/Cargo.toml` :

```toml
[package]
name = "rustty-config"
description = "Configuration TOML de rustty : valeurs par défaut, validation, raccourcis"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
serde.workspace = true
thiserror.workspace = true
toml.workspace = true

[dev-dependencies]

[lints]
workspace = true
```

`crates/rustty-config/src/lib.rs` :

```rust
//! Configuration de rustty : lecture TOML, valeurs par défaut complètes,
//! validation avec erreurs positionnées, table des raccourcis.

pub mod color;

pub use color::{ColorParseError, Rgb};
```

`crates/rustty-config/src/color.rs` :

```rust
//! Couleur concrète de la configuration, écrite `#rrggbb`.

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("couleur invalide « {0} » : attendu #rrggbb")]
pub struct ColorParseError(pub String);

impl FromStr for Rgb {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ColorParseError(s.to_string());
        let hex = s.strip_prefix('#').ok_or_else(err)?;
        if hex.len() != 6 || !hex.is_ascii() {
            return Err(err());
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| err());
        Ok(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.27"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.27 — 2026-10-08 · « Crate config »

- `rustty-config` : type `Rgb` analysé depuis `#rrggbb`, avec erreurs nommant la valeur fautive.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-config CHANGELOG.md
git commit -m "rustty-config : crate et couleurs hexadécimales (0.1.0-alpha.27)"
```

---

### Task 8 : Sections de configuration, défauts, validation et erreurs positionnées

**Files:**
- Create: `crates/rustty-config/src/sections.rs`
- Create: `crates/rustty-config/src/error.rs`
- Create: `crates/rustty-config/src/config.rs`
- Modify: `crates/rustty-config/src/lib.rs`

**Interfaces:**
- Produces:
  - `sections.rs` : `Font { family: String, size: f32, bold_is_bright: bool }`, `Window { opacity: f32, padding: u32, scrollback_lines: usize, confirm_close_with_running_children: bool }`, `TabBarPosition { Top, Bottom, Hidden }`, `CloseButtonStyle { foreground, background, hover_foreground, hover_background: Rgb }`, `Tabs { position, min_tabs: u32, close_button: bool, title_template: String, close_button_style }`, `Colors { foreground, background, cursor, selection_background: Rgb, palette: [Rgb; 16] }`. Tous `Deserialize` avec `#[serde(default, deny_unknown_fields)]`, `Default` = valeurs de la spec (thème Catppuccin Mocha).
  - `error.rs` : `pub enum ConfigError { Parse { line: usize, column: usize, message: String }, Invalid { field: &'static str, reason: String }, Io { path: PathBuf, source: std::io::Error } }` (`thiserror`, `Display` en français).
  - `config.rs` : `pub struct Config { pub font, pub window, pub tabs, pub colors }` ; `Config::from_str(toml: &str) -> Result<Config, ConfigError>` (parse **puis** `validate`) ; `Config::validate(&self) -> Result<(), ConfigError>`.

- [ ] **Step 1: Écrire les tests**

`crates/rustty-config/src/config.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgb;
    use crate::sections::TabBarPosition;

    #[test]
    fn empty_document_is_the_default_config() {
        assert_eq!(Config::from_str("").unwrap(), Config::default());
    }

    #[test]
    fn defaults_match_the_spec() {
        let c = Config::default();
        assert_eq!(c.font.family, "monospace");
        assert_eq!(c.font.size, 11.0);
        assert_eq!(c.window.opacity, 1.0);
        assert_eq!(c.window.scrollback_lines, 10_000);
        assert_eq!(c.tabs.position, TabBarPosition::Top);
        assert!(c.tabs.close_button);
        assert_eq!(c.tabs.close_button_style.background, Rgb::new(0xd3, 0x2f, 0x2f));
        assert_eq!(c.colors.background, Rgb::new(0x1e, 0x1e, 0x2e));
        assert_eq!(c.colors.palette[1], Rgb::new(0xf3, 0x8b, 0xa8));
        assert_eq!(c.colors.palette[15], Rgb::new(0xa6, 0xad, 0xc8));
    }

    #[test]
    fn partial_sections_override_only_what_they_name() {
        let c = Config::from_str("[font]\nsize = 14.5\n\n[window]\nopacity = 0.9\n\n[tabs]\nposition = \"bottom\"\n").unwrap();
        assert_eq!(c.font.size, 14.5);
        assert_eq!(c.font.family, "monospace", "le reste de la section garde le défaut");
        assert_eq!(c.window.opacity, 0.9);
        assert_eq!(c.tabs.position, TabBarPosition::Bottom);
    }

    #[test]
    fn nested_close_button_style_and_palette() {
        let toml = "[tabs.close_button_style]\nhover_background = \"#ff0000\"\n\n[colors]\npalette = [\"#000000\", \"#111111\", \"#222222\", \"#333333\", \"#444444\", \"#555555\", \"#666666\", \"#777777\", \"#888888\", \"#999999\", \"#aaaaaa\", \"#bbbbbb\", \"#cccccc\", \"#dddddd\", \"#eeeeee\", \"#ffffff\"]\n";
        let c = Config::from_str(toml).unwrap();
        assert_eq!(c.tabs.close_button_style.hover_background, Rgb::new(255, 0, 0));
        assert_eq!(c.tabs.close_button_style.foreground, Rgb::new(255, 255, 255), "défaut conservé");
        assert_eq!(c.colors.palette[15], Rgb::new(255, 255, 255));
    }

    #[test]
    fn unknown_field_is_an_error_with_line() {
        let err = Config::from_str("[font]\nsize = 12\n\n[window]\nopacityy = 0.5\n").unwrap_err();
        match err {
            ConfigError::Parse { line, message, .. } => {
                assert_eq!(line, 5, "{message}");
                assert!(message.contains("opacityy"), "{message}");
            }
            other => panic!("attendu Parse, obtenu {other:?}"),
        }
        let err = Config::from_str("[fonts]\nsize = 12\n").unwrap_err();
        assert!(matches!(err, ConfigError::Parse { line: 1, .. }), "{err}");
    }

    #[test]
    fn syntax_error_is_positioned() {
        let err = Config::from_str("[font]\nsize = \n").unwrap_err();
        assert!(matches!(err, ConfigError::Parse { line: 2, .. }), "{err}");
    }

    #[test]
    fn wrong_palette_length_is_an_error() {
        let err = Config::from_str("[colors]\npalette = [\"#000000\"]\n").unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }), "{err}");
    }

    #[test]
    fn validation_rejects_out_of_range_values() {
        for (toml, field) in [
            ("[window]\nopacity = 1.5\n", "window.opacity"),
            ("[window]\nopacity = -0.1\n", "window.opacity"),
            ("[font]\nsize = 0\n", "font.size"),
            ("[font]\nsize = -3\n", "font.size"),
            ("[tabs]\nmin_tabs = 0\n", "tabs.min_tabs"),
            ("[window]\nscrollback_lines = 10000000\n", "window.scrollback_lines"),
        ] {
            match Config::from_str(toml) {
                Err(ConfigError::Invalid { field: f, .. }) => assert_eq!(f, field, "{toml}"),
                other => panic!("{toml}: attendu Invalid, obtenu {other:?}"),
            }
        }
    }

    #[test]
    fn error_messages_are_in_french_and_name_the_position() {
        let err = Config::from_str("[window]\nopacity = 2\n").unwrap_err();
        assert!(err.to_string().contains("window.opacity"), "{err}");
        let err = Config::from_str("x = \n").unwrap_err();
        assert!(err.to_string().starts_with("ligne 1"), "{err}");
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-config`
Expected: modules introuvables.

- [ ] **Step 3: Implémenter**

`crates/rustty-config/src/sections.rs` :

```rust
//! Les sections du fichier TOML et leurs valeurs par défaut. Chaque section
//! accepte un sous-ensemble de clés et refuse les clés inconnues.

use serde::Deserialize;

use crate::color::Rgb;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Font {
    /// Famille demandée ; repli automatique sur une police à chasse fixe du système.
    pub family: String,
    /// Taille en points.
    pub size: f32,
    /// Afficher le gras avec la couleur vive correspondante (0–7 → 8–15).
    pub bold_is_bright: bool,
}

impl Default for Font {
    fn default() -> Self {
        Self { family: "monospace".into(), size: 11.0, bold_is_bright: false }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Window {
    /// Opacité du fond, 0.0 (transparent) à 1.0 (opaque).
    pub opacity: f32,
    /// Marge intérieure en pixels autour de la grille.
    pub padding: u32,
    /// Lignes d'historique conservées par fenêtre.
    pub scrollback_lines: usize,
    /// Demander confirmation avant de fermer une fenêtre dont le shell a des enfants.
    pub confirm_close_with_running_children: bool,
}

impl Default for Window {
    fn default() -> Self {
        Self { opacity: 1.0, padding: 4, scrollback_lines: 10_000, confirm_close_with_running_children: true }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabBarPosition {
    Top,
    Bottom,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CloseButtonStyle {
    pub foreground: Rgb,
    pub background: Rgb,
    pub hover_foreground: Rgb,
    pub hover_background: Rgb,
}

impl Default for CloseButtonStyle {
    fn default() -> Self {
        Self {
            foreground: Rgb::new(0xff, 0xff, 0xff),
            background: Rgb::new(0xd3, 0x2f, 0x2f),
            hover_foreground: Rgb::new(0xff, 0xff, 0xff),
            hover_background: Rgb::new(0xef, 0x53, 0x50),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tabs {
    pub position: TabBarPosition,
    /// Nombre d'onglets à partir duquel la barre s'affiche.
    pub min_tabs: u32,
    /// Bouton de fermeture cliquable sur chaque onglet.
    pub close_button: bool,
    /// Gabarit du titre : `{index}` et `{title}` sont remplacés.
    pub title_template: String,
    pub close_button_style: CloseButtonStyle,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            position: TabBarPosition::Top,
            min_tabs: 1,
            close_button: true,
            title_template: "{index}: {title}".into(),
            close_button_style: CloseButtonStyle::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Rgb,
    pub selection_background: Rgb,
    /// Les 16 couleurs ANSI : 0–7 normales, 8–15 vives.
    pub palette: [Rgb; 16],
}

impl Default for Colors {
    /// Thème Catppuccin Mocha.
    fn default() -> Self {
        let p = |r, g, b| Rgb::new(r, g, b);
        Self {
            foreground: p(0xcd, 0xd6, 0xf4),
            background: p(0x1e, 0x1e, 0x2e),
            cursor: p(0xf5, 0xe0, 0xdc),
            selection_background: p(0x45, 0x47, 0x5a),
            palette: [
                p(0x45, 0x47, 0x5a),
                p(0xf3, 0x8b, 0xa8),
                p(0xa6, 0xe3, 0xa1),
                p(0xf9, 0xe2, 0xaf),
                p(0x89, 0xb4, 0xfa),
                p(0xf5, 0xc2, 0xe7),
                p(0x94, 0xe2, 0xd5),
                p(0xba, 0xc2, 0xde),
                p(0x58, 0x5b, 0x70),
                p(0xf3, 0x8b, 0xa8),
                p(0xa6, 0xe3, 0xa1),
                p(0xf9, 0xe2, 0xaf),
                p(0x89, 0xb4, 0xfa),
                p(0xf5, 0xc2, 0xe7),
                p(0x94, 0xe2, 0xd5),
                p(0xa6, 0xad, 0xc8),
            ],
        }
    }
}
```

`crates/rustty-config/src/error.rs` :

```rust
//! Erreurs de configuration, toujours positionnées : ligne et colonne pour
//! une erreur de syntaxe ou de clé, nom du champ pour une valeur hors bornes.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("ligne {line}, colonne {column} : {message}")]
    Parse { line: usize, column: usize, message: String },
    #[error("valeur invalide pour {field} : {reason}")]
    Invalid { field: &'static str, reason: String },
    #[error("impossible de lire {path} : {source}")]
    Io { path: PathBuf, source: std::io::Error },
}

impl ConfigError {
    /// Convertit une erreur `toml` en position (ligne, colonne) 1-indexée.
    pub(crate) fn from_toml(source: &str, err: &toml::de::Error) -> Self {
        let offset = err.span().map_or(0, |s| s.start).min(source.len());
        let before = &source[..offset];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, str::len) + 1;
        Self::Parse { line, column, message: err.message().to_string() }
    }
}
```

`crates/rustty-config/src/config.rs` :

```rust
//! La configuration complète : analyse, validation.

use serde::Deserialize;

use crate::error::ConfigError;
use crate::sections::{Colors, Font, Tabs, Window};

/// Plafond de lignes d'historique par fenêtre : au-delà, la mémoire explose
/// avant que l'utilisateur ne s'en serve.
pub const MAX_SCROLLBACK_LINES: usize = 1_000_000;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub font: Font,
    pub window: Window,
    pub tabs: Tabs,
    pub colors: Colors,
}

impl Config {
    /// Analyse un document TOML puis le valide.
    pub fn from_str(toml: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(toml).map_err(|e| ConfigError::from_toml(toml, &e))?;
        config.validate()?;
        Ok(config)
    }

    /// Bornes que le typage ne garantit pas.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let invalid = |field, reason: String| Err(ConfigError::Invalid { field, reason });
        if !(0.0..=1.0).contains(&self.window.opacity) {
            return invalid("window.opacity", format!("{} n'est pas entre 0.0 et 1.0", self.window.opacity));
        }
        if !(self.font.size > 0.0 && self.font.size.is_finite()) {
            return invalid("font.size", format!("{} doit être strictement positive", self.font.size));
        }
        if self.tabs.min_tabs == 0 {
            return invalid("tabs.min_tabs", "doit valoir au moins 1 (utiliser position = \"hidden\" pour masquer la barre)".into());
        }
        if self.window.scrollback_lines > MAX_SCROLLBACK_LINES {
            return invalid("window.scrollback_lines", format!("{} dépasse le plafond de {MAX_SCROLLBACK_LINES}", self.window.scrollback_lines));
        }
        Ok(())
    }
}
```

Clippy signalera `should_implement_trait` pour `from_str` : ajouter `#[allow(clippy::should_implement_trait)]` sur la méthode avec le commentaire `// Pas FromStr : l'erreur porte un contexte riche et la méthode valide en plus d'analyser.`

`lib.rs`, ajouter :

```rust
pub mod config;
pub mod error;
pub mod sections;

pub use config::{Config, MAX_SCROLLBACK_LINES};
pub use error::ConfigError;
pub use sections::{CloseButtonStyle, Colors, Font, TabBarPosition, Tabs, Window};
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Si `toml` ne fournit pas de span pour « unknown field » (ligne 0), lire la version résolue dans `Cargo.lock` : `toml` ≥ 0.8 attache un span ; sinon reporter la ligne de la table (`[window]`) est acceptable et le test `unknown_field_is_an_error_with_line` doit être ajusté à `line == 4` avec un commentaire expliquant la limite.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.28"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.28 — 2026-10-08 · « Sections de configuration »

- `rustty-config` : sections `font`, `window`, `tabs` (avec style du bouton de fermeture) et `colors` (palette Catppuccin Mocha par défaut), clés inconnues refusées, erreurs positionnées ligne et colonne, validation des bornes.
```

```bash
git add Cargo.toml crates/rustty-config CHANGELOG.md
git commit -m "rustty-config : sections, défauts, validation et erreurs positionnées (0.1.0-alpha.28)"
```

---

### Task 9 : Combinaisons de touches

**Files:**
- Create: `crates/rustty-config/src/keys.rs`
- Modify: `crates/rustty-config/src/lib.rs`
- Modify: `crates/rustty-config/Cargo.toml` (`bitflags`)

**Interfaces:**
- Produces:
  - `pub struct Mods: u8` (bitflags : `CTRL`, `SHIFT`, `ALT`, `SUPER`).
  - `pub enum NamedKey { Enter, Tab, Space, Backspace, Escape, Delete, Insert, Home, End, PageUp, PageDown, Left, Right, Up, Down, F(u8) }`.
  - `pub enum Key { Char(char), Named(NamedKey) }` ; `pub struct KeyCombo { pub mods: Mods, pub key: Key }` (Copy, Eq, Hash).
  - `FromStr for KeyCombo` : jetons séparés par `+`, casse indifférente ; modificateurs `ctrl`/`control`, `shift`, `alt`/`option`, `super`/`cmd`/`win`/`meta` ; dernier jeton = touche : un caractère unique (mis en minuscule) ou un nom (`enter`, `return`, `tab`, `space`, `backspace`, `escape`/`esc`, `delete`/`del`, `insert`, `home`, `end`, `page_up`/`pageup`, `page_down`/`pagedown`, `left`, `right`, `up`, `down`, `f1`…`f24`). Erreur `KeyParseError { input: String, reason: String }`.
  - `Display for KeyCombo` : forme canonique `ctrl+shift+t`, `shift+page_up`, `f5`.
  - `Deserialize for KeyCombo` depuis une chaîne.

- [ ] **Step 1: Écrire les tests**

`crates/rustty-config/src/keys.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn combo(s: &str) -> KeyCombo {
        s.parse().unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn parses_modifiers_and_character_keys() {
        assert_eq!(combo("ctrl+shift+t"), KeyCombo { mods: Mods::CTRL | Mods::SHIFT, key: Key::Char('t') });
        assert_eq!(combo("Ctrl+Shift+T"), combo("ctrl+shift+t"), "casse indifférente, caractère normalisé en minuscule");
        assert_eq!(combo("alt+1"), KeyCombo { mods: Mods::ALT, key: Key::Char('1') });
        assert_eq!(combo("super+é"), KeyCombo { mods: Mods::SUPER, key: Key::Char('é') });
        assert_eq!(combo("control+option+cmd+x").mods, Mods::CTRL | Mods::ALT | Mods::SUPER);
    }

    #[test]
    fn parses_named_keys_and_aliases() {
        assert_eq!(combo("shift+page_up").key, Key::Named(NamedKey::PageUp));
        assert_eq!(combo("shift+PageUp").key, Key::Named(NamedKey::PageUp));
        assert_eq!(combo("esc").key, Key::Named(NamedKey::Escape));
        assert_eq!(combo("return").key, Key::Named(NamedKey::Enter));
        assert_eq!(combo("f5").key, Key::Named(NamedKey::F(5)));
        assert_eq!(combo("ctrl+F12").key, Key::Named(NamedKey::F(12)));
        assert_eq!(combo("left").key, Key::Named(NamedKey::Left));
    }

    #[test]
    fn invalid_combos_are_rejected_with_the_offending_text() {
        for bad in ["", "ctrl+", "+t", "ctlr+t", "ctrl+shift+", "ctrl+ctrl+t", "f0", "f25", "ctrl+toto", "a+b"] {
            let err = bad.parse::<KeyCombo>().unwrap_err();
            assert!(err.to_string().contains(bad), "{bad:?} → {err}");
        }
    }

    #[test]
    fn display_is_canonical_and_round_trips() {
        for (input, canonical) in [
            ("Shift+Ctrl+T", "ctrl+shift+t"),
            ("cmd+alt+space", "alt+super+space"),
            ("shift+pageup", "shift+page_up"),
            ("F5", "f5"),
            ("x", "x"),
        ] {
            let c = combo(input);
            assert_eq!(c.to_string(), canonical);
            assert_eq!(combo(canonical), c);
        }
    }

    #[test]
    fn deserializes_from_toml_string() {
        #[derive(serde::Deserialize)]
        struct Doc {
            k: KeyCombo,
        }
        let d: Doc = toml::from_str("k = \"ctrl+shift+e\"").unwrap();
        assert_eq!(d.k, combo("ctrl+shift+e"));
        let err = toml::from_str::<Doc>("k = \"ctrl+\"").unwrap_err();
        assert!(err.to_string().contains("ctrl+"), "{err}");
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-config`
Expected: module `keys` introuvable.

- [ ] **Step 3: Implémenter**

`Cargo.toml` de la crate, `[dependencies]` : ajouter `bitflags.workspace = true`.

`crates/rustty-config/src/keys.rs` :

```rust
//! Combinaisons de touches telles qu'écrites dans la configuration :
//! `ctrl+shift+t`, `shift+page_up`, `f5`.

use std::fmt;
use std::str::FromStr;

use bitflags::bitflags;
use serde::Deserialize;

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Mods: u8 {
        const CTRL = 1 << 0;
        const SHIFT = 1 << 1;
        const ALT = 1 << 2;
        const SUPER = 1 << 3;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Enter,
    Tab,
    Space,
    Backspace,
    Escape,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
    /// F1 à F24.
    F(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// Caractère produit par la touche, en minuscule.
    Char(char),
    Named(NamedKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub mods: Mods,
    pub key: Key,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("raccourci invalide « {input} » : {reason}")]
pub struct KeyParseError {
    pub input: String,
    pub reason: String,
}

const MODIFIERS: &[(&str, Mods)] = &[
    ("ctrl", Mods::CTRL),
    ("control", Mods::CTRL),
    ("shift", Mods::SHIFT),
    ("alt", Mods::ALT),
    ("option", Mods::ALT),
    ("super", Mods::SUPER),
    ("cmd", Mods::SUPER),
    ("win", Mods::SUPER),
    ("meta", Mods::SUPER),
];

const NAMED: &[(&str, NamedKey)] = &[
    ("enter", NamedKey::Enter),
    ("return", NamedKey::Enter),
    ("tab", NamedKey::Tab),
    ("space", NamedKey::Space),
    ("backspace", NamedKey::Backspace),
    ("escape", NamedKey::Escape),
    ("esc", NamedKey::Escape),
    ("delete", NamedKey::Delete),
    ("del", NamedKey::Delete),
    ("insert", NamedKey::Insert),
    ("home", NamedKey::Home),
    ("end", NamedKey::End),
    ("page_up", NamedKey::PageUp),
    ("pageup", NamedKey::PageUp),
    ("page_down", NamedKey::PageDown),
    ("pagedown", NamedKey::PageDown),
    ("left", NamedKey::Left),
    ("right", NamedKey::Right),
    ("up", NamedKey::Up),
    ("down", NamedKey::Down),
];

fn parse_key(token: &str) -> Option<Key> {
    let mut chars = token.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(Key::Char(c));
    }
    if let Some(named) = NAMED.iter().find(|(name, _)| *name == token) {
        return Some(Key::Named(named.1));
    }
    let n: u8 = token.strip_prefix('f')?.parse().ok()?;
    (1..=24).contains(&n).then_some(Key::Named(NamedKey::F(n)))
}

impl FromStr for KeyCombo {
    type Err = KeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fail = |reason: &str| KeyParseError { input: s.to_string(), reason: reason.to_string() };
        let lowered = s.to_lowercase();
        let tokens: Vec<&str> = lowered.split('+').collect();
        let (key_token, mod_tokens) = tokens.split_last().ok_or_else(|| fail("vide"))?;
        let mut mods = Mods::empty();
        for token in mod_tokens {
            let Some((_, m)) = MODIFIERS.iter().find(|(name, _)| name == token) else {
                return Err(fail(&format!("modificateur inconnu « {token} »")));
            };
            if mods.contains(*m) {
                return Err(fail(&format!("modificateur « {token} » répété")));
            }
            mods |= *m;
        }
        if key_token.is_empty() {
            return Err(fail("il manque la touche après le dernier « + »"));
        }
        let key = parse_key(key_token).ok_or_else(|| fail(&format!("touche inconnue « {key_token} »")))?;
        Ok(Self { mods, key })
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, m) in [("ctrl", Mods::CTRL), ("shift", Mods::SHIFT), ("alt", Mods::ALT), ("super", Mods::SUPER)] {
            if self.mods.contains(m) {
                write!(f, "{name}+")?;
            }
        }
        match self.key {
            Key::Char(c) => write!(f, "{c}"),
            Key::Named(NamedKey::F(n)) => write!(f, "f{n}"),
            Key::Named(named) => {
                let name = NAMED.iter().find(|(_, k)| *k == named).map_or("?", |(n, _)| n);
                write!(f, "{name}")
            }
        }
    }
}

impl<'de> Deserialize<'de> for KeyCombo {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
```

Note sur `Display` : la table `NAMED` liste le nom canonique **en premier** pour chaque touche (`enter` avant `return`, `page_up` avant `pageup`), c'est ce que `find` renvoie.

`lib.rs` : `pub mod keys;` et `pub use keys::{Key, KeyCombo, KeyParseError, Mods, NamedKey};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Cas `"a+b"` : `a` n'est pas un modificateur → erreur « modificateur inconnu » qui contient bien `a+b` dans `input`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.29"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.29 — 2026-10-08 · « Combinaisons de touches »

- `rustty-config` : `KeyCombo` analysé depuis `ctrl+shift+t`, alias des modificateurs et des touches nommées, forme canonique, erreurs nommant le raccourci fautif.
```

```bash
git add Cargo.toml crates/rustty-config CHANGELOG.md
git commit -m "rustty-config : analyse des combinaisons de touches (0.1.0-alpha.29)"
```

---

### Task 10 : Actions et table des raccourcis

**Files:**
- Create: `crates/rustty-config/src/action.rs`
- Create: `crates/rustty-config/src/keymap.rs`
- Modify: `crates/rustty-config/src/config.rs` (champ `keys`)
- Modify: `crates/rustty-config/src/lib.rs`

**Interfaces:**
- Produces:
  - `action.rs` : `pub enum SplitAxis { Horizontal, Vertical }`, `pub enum FocusDirection { Left, Right, Up, Down }`, `pub enum ResizeDir { Narrower, Wider, Taller, Shorter }` (tous `Deserialize` en minuscules ; le binaire les traduit vers `rustty-layout`, les deux crates restent indépendantes) ; `pub enum Action { NewTab, CloseTab, NextTab, PrevTab, GoToTab(u8), CloseWindow, Split(SplitAxis), Focus(FocusDirection), Resize(ResizeDir), ToggleZoom, Rotate, Opacity(f32), Copy, Paste, ScrollLines(i32), ScrollPages(i32), ScrollToBottom, ReloadConfig, Unbind }` ; `Deserialize for Action` acceptant une chaîne (`"new_tab"`, …, `"none"` = `Unbind`) ou une table à une clé (`{ split = "horizontal" }`, `{ focus = "left" }`, `{ resize = "wider" }`, `{ opacity = +0.05 }`, `{ go_to_tab = 3 }`, `{ scroll_lines = -3 }`, `{ scroll_pages = 1 }`).
  - `keymap.rs` : `pub struct KeyMap` ; `KeyMap::defaults()`, `KeyMap::empty()`, `insert(&mut self, combo, action)`, `resolve(&self, combo) -> Option<Action>` (`Unbind` ⇒ `None`), `len()`, `bindings() -> impl Iterator<Item = (&KeyCombo, &Action)>` ; `Default` = `defaults()` ; `Deserialize` : une table TOML de surcharges **appliquées par-dessus les défauts**, dont les erreurs nomment la clé fautive.
  - `Config` gagne `pub keys: KeyMap`.

- [ ] **Step 1: Écrire les tests**

`crates/rustty-config/src/action.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml_value: &str) -> Result<Action, toml::de::Error> {
        #[derive(serde::Deserialize)]
        struct Doc {
            a: Action,
        }
        toml::from_str::<Doc>(&format!("a = {toml_value}")).map(|d| d.a)
    }

    #[test]
    fn string_forms() {
        assert_eq!(parse("\"new_tab\"").unwrap(), Action::NewTab);
        assert_eq!(parse("\"close_window\"").unwrap(), Action::CloseWindow);
        assert_eq!(parse("\"toggle_zoom\"").unwrap(), Action::ToggleZoom);
        assert_eq!(parse("\"scroll_to_bottom\"").unwrap(), Action::ScrollToBottom);
        assert_eq!(parse("\"none\"").unwrap(), Action::Unbind);
    }

    #[test]
    fn table_forms() {
        assert_eq!(parse("{ split = \"horizontal\" }").unwrap(), Action::Split(SplitAxis::Horizontal));
        assert_eq!(parse("{ focus = \"left\" }").unwrap(), Action::Focus(FocusDirection::Left));
        assert_eq!(parse("{ resize = \"wider\" }").unwrap(), Action::Resize(ResizeDir::Wider));
        assert_eq!(parse("{ opacity = +0.05 }").unwrap(), Action::Opacity(0.05));
        assert_eq!(parse("{ opacity = -0.1 }").unwrap(), Action::Opacity(-0.1));
        assert_eq!(parse("{ go_to_tab = 3 }").unwrap(), Action::GoToTab(3));
        assert_eq!(parse("{ scroll_lines = -3 }").unwrap(), Action::ScrollLines(-3));
        assert_eq!(parse("{ scroll_pages = 1 }").unwrap(), Action::ScrollPages(1));
    }

    #[test]
    fn unknown_forms_are_errors() {
        assert!(parse("\"new_tabz\"").is_err());
        assert!(parse("{ split = \"diagonal\" }").is_err());
        assert!(parse("{ split = \"horizontal\", focus = \"left\" }").is_err(), "une seule clé par action");
        assert!(parse("{ teleport = 1 }").is_err());
        assert!(parse("42").is_err());
    }
}
```

`crates/rustty-config/src/keymap.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{FocusDirection, SplitAxis};

    fn combo(s: &str) -> KeyCombo {
        s.parse().unwrap()
    }

    #[test]
    fn defaults_cover_the_v01_actions() {
        let km = KeyMap::defaults();
        assert_eq!(km.resolve(combo("ctrl+shift+t")), Some(Action::NewTab));
        assert_eq!(km.resolve(combo("ctrl+shift+o")), Some(Action::Split(SplitAxis::Horizontal)));
        assert_eq!(km.resolve(combo("ctrl+shift+e")), Some(Action::Split(SplitAxis::Vertical)));
        assert_eq!(km.resolve(combo("shift+left")), Some(Action::Focus(FocusDirection::Left)));
        assert_eq!(km.resolve(combo("ctrl+shift+c")), Some(Action::Copy));
        assert_eq!(km.resolve(combo("ctrl+shift+v")), Some(Action::Paste));
        assert_eq!(km.resolve(combo("shift+page_up")), Some(Action::ScrollPages(1)));
        assert_eq!(km.resolve(combo("ctrl+shift+f5")), Some(Action::ReloadConfig));
        assert_eq!(km.resolve(combo("ctrl+c")), None, "ctrl+c reste au shell");
        assert!(km.len() >= 20);
    }

    #[test]
    fn user_bindings_override_and_unbind() {
        let km: KeyMap = toml::from_str("\"ctrl+shift+t\" = \"close_tab\"\n\"ctrl+shift+q\" = \"none\"\n\"f9\" = { split = \"vertical\" }\n").unwrap();
        assert_eq!(km.resolve(combo("ctrl+shift+t")), Some(Action::CloseTab));
        assert_eq!(km.resolve(combo("ctrl+shift+q")), None, "délié");
        assert_eq!(km.resolve(combo("f9")), Some(Action::Split(SplitAxis::Vertical)));
        assert_eq!(km.resolve(combo("ctrl+shift+e")), Some(Action::Split(SplitAxis::Vertical)), "défaut non touché conservé");
    }

    #[test]
    fn same_combo_written_differently_is_one_binding() {
        let km: KeyMap = toml::from_str("\"Shift+Ctrl+T\" = \"close_tab\"\n").unwrap();
        assert_eq!(km.resolve(combo("ctrl+shift+t")), Some(Action::CloseTab));
    }

    #[test]
    fn errors_name_the_offending_binding() {
        let err = toml::from_str::<KeyMap>("\"ctrl+\" = \"new_tab\"\n").unwrap_err();
        assert!(err.to_string().contains("ctrl+"), "{err}");
        let err = toml::from_str::<KeyMap>("\"ctrl+shift+t\" = \"new_tabz\"\n").unwrap_err();
        assert!(err.to_string().contains("ctrl+shift+t"), "{err}");
        assert!(err.to_string().contains("new_tabz"), "{err}");
    }

    #[test]
    fn empty_keymap_resolves_nothing() {
        assert_eq!(KeyMap::empty().resolve(combo("ctrl+shift+t")), None);
        assert_eq!(KeyMap::empty().len(), 0);
    }
}
```

Ajouter dans `mod tests` de `config.rs` :

```rust
    #[test]
    fn keys_section_overrides_defaults_and_is_optional() {
        let c = Config::from_str("[keys]\n\"ctrl+shift+n\" = \"new_tab\"\n").unwrap();
        assert_eq!(c.keys.resolve("ctrl+shift+n".parse().unwrap()), Some(crate::action::Action::NewTab));
        assert_eq!(c.keys.resolve("ctrl+shift+t".parse().unwrap()), Some(crate::action::Action::NewTab));
        assert_eq!(Config::from_str("").unwrap().keys.len(), crate::keymap::KeyMap::defaults().len());
    }

    #[test]
    fn bad_binding_in_keys_section_is_a_positioned_parse_error() {
        let err = Config::from_str("[font]\nsize = 12\n\n[keys]\n\"ctlr+t\" = \"new_tab\"\n").unwrap_err();
        match err {
            ConfigError::Parse { line, message, .. } => {
                assert!(message.contains("ctlr+t"), "{message}");
                assert!(line >= 4, "au moins la ligne de la table [keys], obtenu {line}");
            }
            other => panic!("attendu Parse, obtenu {other:?}"),
        }
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-config`
Expected: modules introuvables.

- [ ] **Step 3: Implémenter**

`crates/rustty-config/src/action.rs` :

```rust
//! Ce qu'un raccourci déclenche. Deux écritures TOML : une chaîne pour les
//! actions sans paramètre, une table à une clé pour les autres.

use serde::Deserialize;

/// Même convention que `rustty-layout` : `horizontal` empile, `vertical` juxtapose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResizeDir {
    Narrower,
    Wider,
    Taller,
    Shorter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    GoToTab(u8),
    CloseWindow,
    Split(SplitAxis),
    Focus(FocusDirection),
    Resize(ResizeDir),
    ToggleZoom,
    Rotate,
    /// Variation d'opacité, positive ou négative.
    Opacity(f32),
    Copy,
    Paste,
    ScrollLines(i32),
    ScrollPages(i32),
    ScrollToBottom,
    ReloadConfig,
    /// `"none"` : retire un raccourci par défaut.
    Unbind,
}

/// Les actions sans paramètre, telles qu'écrites dans le TOML.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Simple {
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    CloseWindow,
    ToggleZoom,
    Rotate,
    Copy,
    Paste,
    ScrollToBottom,
    ReloadConfig,
    None,
}

/// Les actions à paramètre : une table avec exactement une clé.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parametrized {
    split: Option<SplitAxis>,
    focus: Option<FocusDirection>,
    resize: Option<ResizeDir>,
    opacity: Option<f32>,
    go_to_tab: Option<u8>,
    scroll_lines: Option<i32>,
    scroll_pages: Option<i32>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Spec {
    Simple(Simple),
    Parametrized(Parametrized),
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        match Spec::deserialize(deserializer).map_err(|_| {
            D::Error::custom("action inconnue : attendu un nom comme \"new_tab\" ou une table comme { split = \"horizontal\" }")
        })? {
            Spec::Simple(s) => Ok(match s {
                Simple::NewTab => Self::NewTab,
                Simple::CloseTab => Self::CloseTab,
                Simple::NextTab => Self::NextTab,
                Simple::PrevTab => Self::PrevTab,
                Simple::CloseWindow => Self::CloseWindow,
                Simple::ToggleZoom => Self::ToggleZoom,
                Simple::Rotate => Self::Rotate,
                Simple::Copy => Self::Copy,
                Simple::Paste => Self::Paste,
                Simple::ScrollToBottom => Self::ScrollToBottom,
                Simple::ReloadConfig => Self::ReloadConfig,
                Simple::None => Self::Unbind,
            }),
            Spec::Parametrized(p) => {
                let candidates = [
                    p.split.map(Self::Split),
                    p.focus.map(Self::Focus),
                    p.resize.map(Self::Resize),
                    p.opacity.map(Self::Opacity),
                    p.go_to_tab.map(Self::GoToTab),
                    p.scroll_lines.map(Self::ScrollLines),
                    p.scroll_pages.map(Self::ScrollPages),
                ];
                let mut found = candidates.into_iter().flatten();
                match (found.next(), found.next()) {
                    (Some(action), None) => Ok(action),
                    _ => Err(D::Error::custom("une action paramétrée a exactement une clé")),
                }
            }
        }
    }
}
```

`crates/rustty-config/src/keymap.rs` :

```rust
//! Table des raccourcis : les défauts, surchargés par la section `[keys]`.

use std::collections::HashMap;

use serde::Deserialize;

use crate::action::{Action, FocusDirection, ResizeDir, SplitAxis};
use crate::keys::KeyCombo;

#[derive(Clone, Debug, PartialEq)]
pub struct KeyMap {
    bindings: HashMap<KeyCombo, Action>,
}

/// Raccourcis par défaut, proches de ceux de kitty.
const DEFAULTS: &[(&str, Action)] = &[
    ("ctrl+shift+t", Action::NewTab),
    ("ctrl+shift+q", Action::CloseTab),
    ("ctrl+shift+w", Action::CloseWindow),
    ("ctrl+shift+right", Action::NextTab),
    ("ctrl+shift+left", Action::PrevTab),
    ("alt+1", Action::GoToTab(1)),
    ("alt+2", Action::GoToTab(2)),
    ("alt+3", Action::GoToTab(3)),
    ("alt+4", Action::GoToTab(4)),
    ("alt+5", Action::GoToTab(5)),
    ("ctrl+shift+o", Action::Split(SplitAxis::Horizontal)),
    ("ctrl+shift+e", Action::Split(SplitAxis::Vertical)),
    ("shift+left", Action::Focus(FocusDirection::Left)),
    ("shift+right", Action::Focus(FocusDirection::Right)),
    ("shift+up", Action::Focus(FocusDirection::Up)),
    ("shift+down", Action::Focus(FocusDirection::Down)),
    ("ctrl+left", Action::Resize(ResizeDir::Narrower)),
    ("ctrl+right", Action::Resize(ResizeDir::Wider)),
    ("ctrl+up", Action::Resize(ResizeDir::Taller)),
    ("ctrl+down", Action::Resize(ResizeDir::Shorter)),
    ("ctrl+shift+z", Action::ToggleZoom),
    ("ctrl+shift+r", Action::Rotate),
    ("ctrl+shift+c", Action::Copy),
    ("ctrl+shift+v", Action::Paste),
    ("shift+page_up", Action::ScrollPages(1)),
    ("shift+page_down", Action::ScrollPages(-1)),
    ("ctrl+shift+end", Action::ScrollToBottom),
    ("ctrl+shift+f5", Action::ReloadConfig),
];

impl KeyMap {
    pub fn empty() -> Self {
        Self { bindings: HashMap::new() }
    }

    pub fn defaults() -> Self {
        let mut km = Self::empty();
        for (combo, action) in DEFAULTS {
            km.insert(combo.parse().expect("les défauts sont valides"), *action);
        }
        km
    }

    pub fn insert(&mut self, combo: KeyCombo, action: Action) {
        self.bindings.insert(combo, action);
    }

    /// L'action liée à `combo`, `None` si rien n'est lié ou si c'est `Unbind`.
    pub fn resolve(&self, combo: KeyCombo) -> Option<Action> {
        match self.bindings.get(&combo) {
            None | Some(Action::Unbind) => None,
            Some(a) => Some(*a),
        }
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn bindings(&self) -> impl Iterator<Item = (&KeyCombo, &Action)> {
        self.bindings.iter()
    }
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::defaults()
    }
}

impl<'de> Deserialize<'de> for KeyMap {
    /// La section `[keys]` est lue clé par clé pour que chaque erreur nomme le
    /// raccourci concerné, puis appliquée par-dessus les défauts.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let raw: Vec<(String, toml::Value)> = HashMap::<String, toml::Value>::deserialize(deserializer)?.into_iter().collect();
        let mut km = Self::defaults();
        for (text, value) in raw {
            let combo: KeyCombo = text.parse().map_err(D::Error::custom)?;
            let shown = value.to_string();
            let action: Action = value
                .try_into()
                .map_err(|e: toml::de::Error| D::Error::custom(format!("raccourci « {text} » = {shown} : {}", e.message())))?;
            km.insert(combo, action);
        }
        Ok(km)
    }
}
```

Dans `config.rs`, ajouter `use crate::keymap::KeyMap;`, le champ `pub keys: KeyMap,` à `Config` (après `colors`), et remplacer `#[derive(Clone, Debug, Default, PartialEq, Deserialize)]` : `Default` dérivé reste correct puisque `KeyMap: Default`.

`lib.rs`, ajouter :

```rust
pub mod action;
pub mod keymap;

pub use action::{Action, FocusDirection, ResizeDir, SplitAxis};
pub use keymap::KeyMap;
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Si `toml::Value::try_into` n'existe pas dans la version résolue, utiliser `Action::deserialize(value)` (`toml::Value` implémente `Deserializer`).

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.30"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.30 — 2026-10-08 · « Actions et raccourcis »

- `rustty-config` : actions (onglets, divisions, focus, redimensionnement, zoom, rotation, opacité, presse-papiers, défilement, rechargement), table de raccourcis par défaut surchargeable dans `[keys]`, `"none"` pour délier, erreurs nommant le raccourci fautif.
```

```bash
git add Cargo.toml crates/rustty-config CHANGELOG.md
git commit -m "rustty-config : actions et table des raccourcis (0.1.0-alpha.30)"
```

---

### Task 11 : Chargement depuis le disque et chemin par défaut

**Files:**
- Modify: `crates/rustty-config/src/config.rs`
- Modify: `crates/rustty-config/Cargo.toml` (`directories`)

**Interfaces:**
- Produces: `Config::load(path: &Path) -> Result<Config, ConfigError>` (fichier absent ⇒ `Config::default()` ; illisible ⇒ `ConfigError::Io { path }` ; sinon `from_str`) ; `Config::default_path() -> Option<PathBuf>` (`<config_dir>/rustty/rustty.toml` via `directories::ProjectDirs::from("", "", "rustty")`, soit `~/.config/rustty/rustty.toml` sur Linux) ; `Config::load_default() -> Result<Config, ConfigError>` (défauts si aucun chemin n'est résolvable).

- [ ] **Step 1: Écrire les tests**

Ajouter dans `mod tests` de `config.rs` :

```rust
    fn scratch_file(name: &str, contents: Option<&str>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustty-config-tests-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        match contents {
            Some(c) => std::fs::write(&path, c).unwrap(),
            None => {
                let _ = std::fs::remove_file(&path);
            }
        }
        path
    }

    #[test]
    fn missing_file_means_defaults() {
        let path = scratch_file("absent.toml", None);
        assert_eq!(Config::load(&path).unwrap(), Config::default());
    }

    #[test]
    fn existing_file_is_parsed_and_validated() {
        let path = scratch_file("ok.toml", Some("[font]\nsize = 13\n"));
        assert_eq!(Config::load(&path).unwrap().font.size, 13.0);
        let path = scratch_file("bad.toml", Some("[window]\nopacity = 7\n"));
        assert!(matches!(Config::load(&path), Err(ConfigError::Invalid { field: "window.opacity", .. })));
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_file_is_an_io_error_naming_the_path() {
        use std::os::unix::fs::PermissionsExt;
        let path = scratch_file("locked.toml", Some("[font]\nsize = 13\n"));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        let result = Config::load(&path);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        if nix_is_root() {
            return; // root lit tout : le cas ne peut pas être reproduit
        }
        match result {
            Err(ConfigError::Io { path: p, .. }) => assert_eq!(p, path),
            other => panic!("attendu Io, obtenu {other:?}"),
        }
    }

    #[cfg(unix)]
    fn nix_is_root() -> bool {
        std::fs::read_to_string("/proc/self/status")
            .map(|s| s.lines().any(|l| l.starts_with("Uid:\t0\t")))
            .unwrap_or(false)
    }

    #[test]
    fn default_path_ends_with_rustty_toml() {
        if let Some(p) = Config::default_path() {
            assert!(p.ends_with("rustty/rustty.toml") || p.ends_with("rustty\\rustty.toml"), "{p:?}");
        }
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-config`
Expected: `no function or associated item named load`.

- [ ] **Step 3: Implémenter**

`Cargo.toml` de la crate, `[dependencies]` : `directories.workspace = true`.

Dans `config.rs`, ajouter `use std::path::{Path, PathBuf};` et dans `impl Config` :

```rust
    /// Lit et valide `path`. Un fichier absent n'est pas une erreur : ce sont
    /// les défauts. Un fichier illisible en est une.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_str(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Io { path: path.to_path_buf(), source }),
        }
    }

    /// `~/.config/rustty/rustty.toml` sur Linux, l'équivalent ailleurs.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "rustty").map(|d| d.config_dir().join("rustty.toml"))
    }

    pub fn load_default() -> Result<Self, ConfigError> {
        match Self::default_path() {
            Some(p) => Self::load(&p),
            None => Ok(Self::default()),
        }
    }
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.31"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.31 — 2026-10-08 · « Chargement de la configuration »

- `rustty-config` : `Config::load` (fichier absent = défauts, illisible = erreur avec le chemin), chemin par défaut `~/.config/rustty/rustty.toml` et équivalents macOS et Windows.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-config CHANGELOG.md
git commit -m "rustty-config : chargement depuis le disque et chemin par défaut (0.1.0-alpha.31)"
```

---

### Task 12 : Fichier d'exemple commenté, testé contre les défauts, et documentation

**Files:**
- Create: `crates/rustty-config/src/example.rs`
- Create: `docs/rustty.example.toml`
- Modify: `crates/rustty-config/src/lib.rs`
- Modify: `README.md`

**Interfaces:**
- Produces: `pub const DEFAULT_TOML: &str` : le fichier d'exemple complet, chaque clé commentée, **toutes les valeurs égales aux défauts**. Deux tests le garantissent : `Config::from_str(DEFAULT_TOML) == Config::default()` et `docs/rustty.example.toml` identique à `DEFAULT_TOML` (via `include_str!`).

- [ ] **Step 1: Écrire les tests**

`crates/rustty-config/src/example.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn example_file_parses_to_the_defaults() {
        let parsed = Config::from_str(DEFAULT_TOML).unwrap();
        assert_eq!(parsed, Config::default(), "chaque valeur de l'exemple doit être un défaut");
    }

    #[test]
    fn example_file_is_published_in_docs() {
        let published = include_str!("../../../docs/rustty.example.toml");
        assert_eq!(published, DEFAULT_TOML, "régénérer docs/rustty.example.toml depuis DEFAULT_TOML");
    }

    #[test]
    fn every_section_and_key_is_documented() {
        for key in ["[font]", "family", "size", "bold_is_bright", "[window]", "opacity", "padding", "scrollback_lines", "confirm_close_with_running_children", "[tabs]", "position", "min_tabs", "close_button", "title_template", "[tabs.close_button_style]", "hover_background", "[colors]", "palette", "[keys]"] {
            assert!(DEFAULT_TOML.contains(key), "{key} absent de l'exemple");
        }
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-config`
Expected: module `example` introuvable.

- [ ] **Step 3: Implémenter**

`crates/rustty-config/src/example.rs` :

```rust
//! Le fichier de configuration d'exemple. Chaque valeur est le défaut : copier
//! ce fichier ne change rien, le modifier change exactement ce qu'on touche.

pub const DEFAULT_TOML: &str = r###"# Configuration de rustty — ~/.config/rustty/rustty.toml
# Chaque valeur ci-dessous est la valeur par défaut : tout est optionnel.

[font]
# Famille de police ; repli automatique sur une police à chasse fixe du système.
family = "monospace"
# Taille en points.
size = 11.0
# Afficher le gras avec la couleur vive correspondante (0–7 → 8–15).
bold_is_bright = false

[window]
# Opacité du fond, de 0.0 (transparent) à 1.0 (opaque).
opacity = 1.0
# Marge intérieure en pixels autour de la grille.
padding = 4
# Lignes d'historique conservées par fenêtre (plafond : 1 000 000).
scrollback_lines = 10000
# Demander confirmation avant de fermer une fenêtre dont le shell a des enfants.
confirm_close_with_running_children = true

[tabs]
# Position de la barre d'onglets : "top", "bottom" ou "hidden".
position = "top"
# Nombre d'onglets à partir duquel la barre s'affiche.
min_tabs = 1
# Bouton de fermeture cliquable sur chaque onglet.
close_button = true
# Gabarit du titre : {index} et {title} sont remplacés.
title_template = "{index}: {title}"

[tabs.close_button_style]
foreground = "#ffffff"
background = "#d32f2f"
hover_foreground = "#ffffff"
hover_background = "#ef5350"

[colors]
# Thème Catppuccin Mocha.
foreground = "#cdd6f4"
background = "#1e1e2e"
cursor = "#f5e0dc"
selection_background = "#45475a"
# Les 16 couleurs ANSI : 0–7 normales, 8–15 vives.
palette = [
  "#45475a", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de",
  "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8",
]

[keys]
# Les raccourcis ci-dessous sont ceux par défaut ; en ajouter en surcharge
# un existant, et "none" délie une combinaison.
# Actions sans paramètre : "new_tab", "close_tab", "next_tab", "prev_tab",
# "close_window", "toggle_zoom", "rotate", "copy", "paste", "scroll_to_bottom",
# "reload_config", "none".
# Actions à paramètre : { split = "horizontal" | "vertical" },
# { focus = "left" | "right" | "up" | "down" },
# { resize = "narrower" | "wider" | "taller" | "shorter" },
# { opacity = +0.05 }, { go_to_tab = 3 }, { scroll_lines = -3 }, { scroll_pages = 1 }.
"ctrl+shift+t" = "new_tab"
"ctrl+shift+q" = "close_tab"
"ctrl+shift+w" = "close_window"
"ctrl+shift+right" = "next_tab"
"ctrl+shift+left" = "prev_tab"
"alt+1" = { go_to_tab = 1 }
"alt+2" = { go_to_tab = 2 }
"alt+3" = { go_to_tab = 3 }
"alt+4" = { go_to_tab = 4 }
"alt+5" = { go_to_tab = 5 }
"ctrl+shift+o" = { split = "horizontal" }
"ctrl+shift+e" = { split = "vertical" }
"shift+left" = { focus = "left" }
"shift+right" = { focus = "right" }
"shift+up" = { focus = "up" }
"shift+down" = { focus = "down" }
"ctrl+left" = { resize = "narrower" }
"ctrl+right" = { resize = "wider" }
"ctrl+up" = { resize = "taller" }
"ctrl+down" = { resize = "shorter" }
"ctrl+shift+z" = "toggle_zoom"
"ctrl+shift+r" = "rotate"
"ctrl+shift+c" = "copy"
"ctrl+shift+v" = "paste"
"shift+page_up" = { scroll_pages = 1 }
"shift+page_down" = { scroll_pages = -1 }
"ctrl+shift+end" = "scroll_to_bottom"
"ctrl+shift+f5" = "reload_config"
"###;
```

Créer `docs/rustty.example.toml` avec **exactement** le contenu de la chaîne (sans les délimiteurs `r###"` et `"###`), par exemple avec un petit programme jetable ou en copiant à la main puis en laissant le test `example_file_is_published_in_docs` confirmer l'égalité octet pour octet.

`lib.rs` : `pub mod example;` et `pub use example::DEFAULT_TOML;`.

`README.md`, remplacer le bloc « Statut » par :

```markdown
> Statut : fondations. Trois crates de logique pure sont fonctionnelles et
> testées : `rustty-vt` (émulation de terminal), `rustty-layout` (onglets et
> divisions) et `rustty-config` (configuration TOML). Les crates pty, render
> et le binaire suivent, voir les plans dans `docs/superpowers/plans/`.
```

et ajouter après la section « Objectifs de la v0.1 » :

```markdown
## Configuration

Fichier TOML, `~/.config/rustty/rustty.toml` sur Linux (équivalents macOS et
Windows). Toutes les clés sont optionnelles ; le fichier d'exemple
[`docs/rustty.example.toml`](docs/rustty.example.toml) liste chaque clé avec
sa valeur par défaut et les raccourcis fournis.
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Si `example_file_parses_to_the_defaults` échoue, c'est que l'exemple et `Default` divergent : corriger celui des deux qui ne respecte pas la spec, jamais le test.

- [ ] **Step 5: Version, CHANGELOG, commit, push**

`version = "0.1.0-alpha.32"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.32 — 2026-10-08 · « Fichier d'exemple »

- `rustty-config` : `DEFAULT_TOML`, fichier d'exemple commenté publié dans `docs/rustty.example.toml`, garanti égal aux défauts par les tests.
- README : statut des trois crates et section Configuration.
```

```bash
git add Cargo.toml crates/rustty-config docs/rustty.example.toml README.md CHANGELOG.md
git commit -m "rustty-config : fichier d'exemple testé contre les défauts (0.1.0-alpha.32)"
git push
```

Vérifier que le push a bien eu lieu (`git status -sb` doit afficher la branche sans `[ahead N]`), puis attendre la CI verte sur les trois OS.

---

## Suite

Plan suivant : `docs/superpowers/plans/2026-10-08-pty-et-render.md` (crates `rustty-pty` et `rustty-render` avec rendu hors écran), à rédiger une fois ce plan exécuté et la CI verte.
