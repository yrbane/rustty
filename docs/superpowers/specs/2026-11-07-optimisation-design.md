# Audit chiffré et optimisation de rustty — conception

Mandat : « refactorise le code et optimise tout, tu as le droit de tout
retourner si c'est optimisé ». Démarrage sans questions, donc ce document
pose d'abord les mesures, puis les décisions qu'elles justifient. Rien ne
change de comportement visible ; chaque chantier a une cible chiffrée et un
test ou un benchmark qui la pince.

## 1. Mesures (release, machine locale, 2026-11-07, alpha.182)

Benchmarks reproductibles : `cargo run --release -p rustty-vt --example audit`
et `cargo run --release -p rustty-render --example audit` (ajoutés par ce
chantier, conservés).

### 1.1 Émulation (`rustty-vt`)

| Mesure | Valeur | Lecture |
|---|---|---|
| vte seul, 1 MiB texte brut, performer vide | 811 MiB/s | plafond du parseur |
| `Term::input`, 1 MiB texte brut, 80×24, historique 10 000 | 90 MiB/s | **9× sous vte** : ~85 % du coût est dans `put_char` |
| idem, lignes terminées par CR seul (aucun défilement) | 7,7 ms/MiB | coût par caractère ≈ 6,5 ns |
| idem avec CRLF, historique 0 | 8,5 ms/MiB | le défilement coûte ~10 % |
| SGR dense (couleurs 24 bits à chaque mot), 8 000 lignes | 67 MiB/s | |
| `snapshot()` 200×60 avec historique | 16 µs | négligeable, pas un chantier |
| `resize` 200→120→200 colonnes, historique 10 000 | 29,6 ms (≈15 ms par resize) | **appelé à chaque mouvement de barre de split** |
| redessin plein écran alternatif 200×60 (vim) | 91 µs | bon |
| `compact_links` 5 000 liens, historique 5 000 | 1,2 ms | bon |
| `size_of` : `Cell` 16, `Line` 80, `Snapshot` 128 | | `Line` = `Vec<Cell>` + `BTreeMap` + `Vec<ImageStrip>` + bool |

Où part le temps par caractère (lecture du code de `print.rs`, `line.rs`) :
`charsets.map`, `width()`, `erase_template()` reconstruit à chaque caractère,
`clear_wide_partner` (deux lectures de cellule), `Line::set` qui fait
`zerowidth.remove(&col)` sur un `BTreeMap` à chaque écriture, `active_grid_mut`
(branche), avance du curseur. Et par ligne défilée : `Grid::scroll_up` clone
la ligne évincée (`to_vec`) puis `rotate_left` et `reset`.

Reflow : chaque ligne logique est reconstruite en `Vec<Entry>` (`Entry` =
`Cell` + `Option<String>`, 40 octets) puis redécoupée en nouvelles `Line`
allouées une à une, même quand la ligne tient déjà dans la nouvelle largeur.

### 1.2 Rendu (`rustty-render`)

| Mesure | Valeur |
|---|---|
| `pane_instances` 200×60 stylé | 164 µs |
| `Renderer::render` 200×60 hors écran (lavapipe, CPU) | 768 µs |
| objets GPU créés **par image** | 5 `create_buffer_init` + 5 `create_bind_group` (fonds, décor, images, glyphes de panneau, glyphes de chrome), multipliés par le nombre de passes (une par taille de police) |

### 1.3 Binaire et profil

- Aucune section `[profile.release]` : pas de LTO, 16 unités de codegen,
  symboles conservés. Binaire : **21,9 MiB**.
- `catch_unwind` est utilisé dans `rustty-pty` (lecteur) : `panic = "abort"`
  est exclu.

### 1.4 Code

| Mesure | Valeur |
|---|---|
| lignes (src hors tests) | rustty 11 308, vt 6 134, render 4 911, config 2 645, layout 1 249, pty 716 |
| fonctions ≥ 100 lignes | `render::render` 143, `renderer::draw` 114, `grid::pane_instances` 104, `window_state::open` 100 |
| duplication (fenêtres de 4 lignes identiques, hors tests) | pipelines `quad`/`glyph`/`image` : 10 fenêtres ; `context_menu/chrome.rs` ↔ `tooltip.rs` : 6 ; test « point dans un rectangle » réécrit dans 5 fichiers (`tooltip::inside`, `pane_header::contains`, `context_menu::layout::contains`, `title_bar::inside`, …) |
| clippy | `pedantic` 206 (dont 85 `cast_possible_truncation`), `nursery` 70, `perf` 14 |
| dépendances dupliquées imputables à rustty | `base64` 0.22 (nous) contre 0.23 (dépendances) |

## 2. Décisions

Non-objectifs : aucun changement de comportement visible, de format de
configuration ni de protocole ; pas de réécriture de `rustty-layout`
(déjà petit et vérifié par proptest) ; pas de chasse aux 206 avertissements
`pedantic` (les casts sont voulus et bornés). Anti-objectif : une
optimisation sans benchmark avant/après n'existe pas.

Garde-fous communs : suite complète verte, clippy à zéro, images de
référence inchangées, un commit par chantier avec version et CHANGELOG.

### 2.1 Profil de compilation

`[profile.release]` : `lto = "fat"`, `codegen-units = 1`, `opt-level = 3`,
`strip = "symbols"` ; `panic` inchangé (`catch_unwind`). Cible : binaire
≤ 12 MiB, benchmarks égaux ou meilleurs. Un profil `profile.bench`
identique pour les exemples d'audit.

### 2.2 Chemin d'impression

- `Line::set` n'interroge `zerowidth` que si la table n'est pas vide
  (`is_empty()` est une lecture de champ).
- `put_char` : chemin rapide pour un caractère étroit sans retour en attente
  ni mode insertion, sur une cellule qui n'est ni large ni moitié droite :
  une seule lecture de la cellule cible, pas de `erase_template`, pas de
  second passage par `active_grid_mut`.
- `Grid::scroll_up` rend les lignes évincées par échange (`mem::replace`
  avec une ligne vierge réutilisée) au lieu de les cloner.
- Cible : texte brut ≥ 200 MiB/s, SGR dense ≥ 120 MiB/s. Tests de
  `rustty-vt` inchangés (le comportement est le même).

### 2.3 Reflow

- Chemin rapide dans `reflow` : une ligne logique d'une seule ligne source
  (non `wrapped`, précédente non `wrapped`) dont le contenu utile tient dans
  `cols` est redimensionnée en place (`Line::resize`), sans passer par
  `Vec<Entry>` ni réallouer.
- Les lignes logiques longues gardent l'algorithme actuel.
- Cible : `resize` 200→120→200 colonnes avec 10 000 lignes ≤ 10 ms
  (contre 29,6 ms) ; le test `reflowing_ten_thousand_short_lines_is_fast`
  resserré à 20 ms.
- Dans la fenêtre : pendant un glisser de barre, `relayout` ne redimensionne
  les terminaux qu'une fois par image (au `RedrawRequested`), pas à chaque
  événement de souris. Cible : au plus un `Term::resize` par panneau et par
  image pendant le glisser (test sur le modèle : deux `drag_divider_to`
  successifs sans redessin = un seul resize).

### 2.4 Rendu : objets GPU persistants

- Chaque pipeline garde un tampon d'instances persistant par usage (fonds,
  décor, glyphes de panneau, glyphes de chrome, images), rempli par
  `queue.write_buffer` et agrandi seulement quand il est trop petit
  (doublement). Le tampon des globales (viewport) et son bind group sont
  recréés uniquement quand le viewport change.
- Cible : zéro `create_buffer`/`create_bind_group` par image en régime
  établi (compteur exposé en test), `Renderer::render` hors écran
  ≤ 600 µs. Images de référence inchangées.

### 2.5 Structure

- `rustty/src/render.rs` : `render()` devient pure assemblée + présentation :
  `assemble_frame(&self) -> FrameParts` (groupes par taille, chrome de base,
  overlay, menu) testable sans GPU, puis `present(parts)`.
- `rustty-render/src/renderer.rs::draw` : `collect(frame) -> Collected`
  (instances CPU) puis `upload` puis `pass` ; `grid::pane_instances` découpé
  en `cell_background`, `cell_glyph`, `cell_decorations`, `cursor_quads`.
- `window_state::open` : extraction de `build_renderers`, `first_tab`
  (ou équivalent) pour descendre sous 60 lignes.
- Pipelines : `pipeline/common.rs` (`shader(device, label, src)`,
  `instanced_pipeline(device, label, layouts, stride, attrs, format)`,
  `globals_layout`) partagé par `quad`, `glyph`, `image`.
- `PixelRect::contains_point(x: f64, y: f64)` et `contains_px(x: u32, y: u32)`
  dans `rustty-render`, remplaçant les cinq copies.
- Boîte à bordure commune à l'infobulle et au menu contextuel :
  `tooltip::panel_quads(rect, palette) -> [ChromeQuad; 2]` utilisé par les
  deux.
- Cible : plus aucune fonction ≥ 100 lignes (contrôlé par un test qui
  compte, dans `scripts/` ou en `#[test]` de `rustty`, les fonctions des
  fichiers src), duplication des pipelines < 3 fenêtres.

### 2.6 Hygiène

- Les 14 avertissements `clippy::perf` corrigés ; `[workspace.lints.clippy]
  perf = "warn"` pour qu'ils ne reviennent pas.
- `base64` passé en 0.23 si l'API utilisée est inchangée.
- Un seul `snapshot()` par mouvement de souris, partagé par le survol du
  correcteur et celui des liens (`on_cursor_moved` prend l'instantané, les
  deux `*_hover` le reçoivent).
- `docs/perf.md` : tableau avant/après des benchmarks, procédure pour les
  relancer.

### 2.7 Ordre

1. Profil release (mesure de référence du binaire et des benchs après LTO :
   c'est la base des comparaisons suivantes).
2. Chemin d'impression.
3. Reflow et resize par image.
4. Objets GPU persistants.
5. Structure (render, renderer, pane_instances, open, pipelines, rect,
   panneau).
6. Hygiène et `docs/perf.md`.

## 3. Laissé au constructeur

Le détail des signatures internes, le découpage exact des helpers de
`pane_instances`, la stratégie de croissance des tampons (doublement
proposé), et l'emplacement du compteur d'objets GPU (champ `#[cfg(test)]`
ou compteur toujours présent mais léger).
