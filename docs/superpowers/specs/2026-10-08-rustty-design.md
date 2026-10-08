# rustty — spécification de conception (v0.1)

Date : 2026-10-08
Statut : validé en discussion, en attente de relecture écrite

## 1. Intention

rustty est un émulateur de terminal accéléré par GPU, écrit en Rust, **inspiré de
kitty** : on reprend son architecture (grille de cellules rendue par le GPU, onglets
contenant des arbres de fenêtres, layouts, configuration riche) et ses idées, mais le
code est écrit de zéro. Ce n'est **pas** un port fidèle de kitty : on priorise les
fonctionnalités réellement utilisées par l'auteur et on refuse la parité pour la parité.

Succès de la v0.1 : l'auteur peut remplacer kitty au quotidien sur sa machine Linux
avec sa configuration recréée en TOML, et le binaire compile et fonctionne sur macOS
et Windows.

### Décisions cadrantes (validées)

| Sujet | Décision |
|---|---|
| Objectif | Terminal Rust inspiré de kitty, fonctionnalités priorisées par l'usage |
| Plateformes v0.1 | Linux (Wayland + X11), macOS, Windows (ConPTY) |
| Configuration | TOML natif, pas de compatibilité `kitty.conf` |
| Contenu v0.1 | Fenêtre GPU, PTY, émulation VT, polices, couleurs, scrollback, config, raccourcis, onglets avec bouton de fermeture cliquable et survol, splits, opacité du fond |
| Visibilité | Dépôt `yrbane/rustty` privé jusqu'à la v0.1, public ensuite |
| Pile technique | Approche A : `winit` + `wgpu` + `portable-pty` + `vte` + `fontdb`/`swash`, cœur VT maison |
| Licence | GPL-3.0 par défaut (inspiration kitty) ; révisable si aucun code n'est repris |

### Contraintes de l'auteur

- Code et identifiants en anglais ; commentaires, commits, CHANGELOG et docs en français.
- TDD partout où la logique est pure. SOLID, DRY, KISS, YAGNI.
- Chaque commit porte une version SemVer (version unique du workspace Cargo), une
  entrée en tête de `CHANGELOG.md`, et la version est affichée dans l'interface.
- Aucune mention d'assistant dans les commits.

## 2. Architecture

### 2.1 Processus et threads

Un seul processus. Un thread d'interface (boucle `winit`) et **un thread lecteur par
fenêtre de terminal**.

```
clavier/souris ──► winit (thread UI) ──► encodage ──► écriture PTY
                        ▲                                   │
                        │ réveil (EventLoopProxy)           ▼
                   rendu wgpu ◄── instantané ◄── Term (Mutex) ◄── thread lecteur ◄── lecture PTY
                                                     ▲
                                                 parseur vte
```

- Le thread lecteur lit le PTY par blocs, alimente le parseur `vte`, applique les
  actions sur l'état `Term` sous verrou (`parking_lot::Mutex`), puis signale le thread
  UI via `EventLoopProxy`.
- Le thread UI, à la demande de redessin, prend le verrou juste le temps de copier
  les lignes visibles et l'état du curseur (instantané), puis dessine sans verrou.
- Les écritures vers le PTY sont faites depuis le thread UI via un canal vers un
  writer dédié, pour ne jamais bloquer la boucle d'événements.

### 2.2 Workspace Cargo

```
rustty/
├── Cargo.toml              # workspace, version unique [workspace.package]
├── crates/
│   ├── rustty-vt/          # émulation : grille, scrollback, curseur, modes, SGR
│   ├── rustty-layout/      # arbre onglets/splits, calcul des rectangles
│   ├── rustty-config/      # TOML, défauts, validation, raccourcis → actions
│   ├── rustty-pty/         # spawn du shell, lecture/écriture, resize
│   ├── rustty-render/      # wgpu : fonds, glyphes, décorations, atlas, polices
│   └── rustty/             # binaire : boucle winit, assemblage, entrées, chrome
├── docs/
├── CHANGELOG.md
└── README.md
```

Dépendances entre crates : `rustty` dépend de toutes ; `rustty-render` dépend de
`rustty-vt` (types de cellule) et `rustty-config` (couleurs, police) ; les autres
sont indépendantes. `rustty-vt`, `rustty-layout`, `rustty-config` n'ont **aucune**
dépendance système : ce sont les cibles prioritaires du TDD.

## 3. Composants

### 3.1 `rustty-vt`

Responsabilité : transformer un flux d'octets en état de terminal.

- `Grid` : lignes de `Cell { ch: char ou index de graphème, fg, bg, attrs }`,
  largeur/hauteur, zone de scroll, **scrollback** circulaire borné (taille configurable).
- `Term` : grille principale + grille alternative, curseur (position, visible, forme),
  modes (DECAWM, DECTCEM, origin, bracketed paste, souris, alt screen, focus
  reporting), état SGR courant, titre, jeux de caractères G0/G1 (DEC special graphics
  pour les lignes), tabulations.
- `Performer` : implémentation de `vte::Perform` qui traduit CSI/OSC/ESC/contrôles en
  mutations de `Term`.
- Sorties vers l'hôte : réponses aux requêtes (DA, DSR, CPR, OSC couleurs) poussées
  dans une file que le binaire écrit au PTY.
- Événements vers l'UI : `TitleChanged`, `Bell`, `ModeChanged(mouse/alt)`,
  `ClipboardSet(OSC 52)` sous forme d'énumération drainée après chaque passe.
- Largeur des caractères : `unicode-width`, cellules larges marquées par une cellule
  de continuation. Les graphèmes composés sont stockés dans une table annexe
  (`SmallVec` ou index) pour garder `Cell` compact.
- Redimensionnement : rewrap des lignes désactivé en v0.1 (coupe/complète), noté
  pour v0.2.

Hors v0.1 mais prévu dans les types : protocole graphique de kitty (place-holders de
cellule), protocole clavier de kitty, hyperliens OSC 8 (stockage prévu, rendu v0.2).

### 3.2 `rustty-layout`

Responsabilité : géométrie des onglets et des fenêtres, sans rien savoir du rendu.

```rust
enum Node { Leaf(WindowId), Split { axis: Axis, ratio: f32, first: Box<Node>, second: Box<Node> } }
struct TabLayout { root: Node, focused: WindowId, zoomed: Option<WindowId> }
```

Opérations : `split(target, axis) -> WindowId`, `close(id)`, `resize(id, axis, delta)`,
`neighbor(id, Direction) -> Option<WindowId>`, `rotate(id)`, `toggle_zoom(id)`,
`rects(bounds, gap) -> Vec<(WindowId, Rect)>`.

Invariants testés par propriétés : les rectangles produits couvrent `bounds` moins les
espaces, ne se chevauchent pas, et chaque feuille a exactement un rectangle.

### 3.3 `rustty-config`

Responsabilité : lire et valider la configuration, fournir des défauts complets.

- Emplacement : `$XDG_CONFIG_HOME/rustty/rustty.toml` (Linux), équivalents macOS et
  Windows via `directories`. Option `--config <chemin>`.
- Structure (`serde` + `toml`) :

```toml
[font]
family = "JetBrainsMono Nerd Font"   # fallback automatique vers une mono système
size = 11.0
bold_is_bright = false

[window]
opacity = 0.9                        # 0.0..=1.0
padding = 4
scrollback_lines = 10000
confirm_close_with_running_children = true

[tabs]
position = "top"                     # top | bottom | hidden
min_tabs = 1
close_button = true
title_template = "{index}: {title}"

[tabs.close_button_style]
foreground = "#ffffff"
background = "#d32f2f"
hover_foreground = "#ffffff"
hover_background = "#ef5350"

[colors]
foreground = "#cdd6f4"
background = "#1e1e2e"
cursor = "#f5e0dc"
selection_background = "#45475a"
palette = ["#45475a", "#f38ba8", ... ]   # 16 entrées

[keys]
"ctrl+shift+t"     = "new_tab"
"ctrl+shift+q"     = "close_tab"
"ctrl+shift+w"     = "close_window"
"ctrl+shift+o"     = { split = "horizontal" }
"ctrl+shift+e"     = { split = "vertical" }
"shift+left"       = { focus = "left" }
"ctrl+left"        = { resize = "narrower" }
"ctrl+shift+a"     = { opacity = "+0.05" }
```

- Les raccourcis se résolvent en `enum Action` fermée ; une clé inconnue ou une action
  inconnue produit une **erreur positionnée** (ligne, clé) remontée à l'UI.
- Rechargement à chaud : `notify` surveille le fichier ; une config invalide garde
  l'ancienne et affiche un bandeau.

### 3.4 `rustty-pty`

Responsabilité : lancer le shell et échanger des octets, sur les trois OS.

- Par-dessus `portable-pty` : `spawn(shell, args, env, size) -> (Child, Reader, Writer)`.
- Shell par défaut : `$SHELL` sur Unix, PowerShell sur Windows, surchargeable.
- `resize(cols, rows, px_w, px_h)`, détection de la fin du processus, code de sortie.
- Variables injectées : `TERM=xterm-256color` en v0.1 (`xterm-rustty` + terminfo
  dédié en v0.2), `COLORTERM=truecolor`.

### 3.5 `rustty-render`

Responsabilité : dessiner un instantané de terminal et le chrome avec wgpu.

- Trois pipelines instanciés : **fonds de cellule** (quad coloré par cellule, fusionné en
  runs horizontaux de même couleur), **glyphes** (quad texturé depuis un atlas), **décorations**
  (soulignés, barré, curseur, bordures de split, barre d'onglets, bandeau d'erreur).
- Atlas de glyphes : texture R8 pour les glyphes monochromes, RGBA8 pour la couleur
  (emoji), clé `(font_id, glyph_id, subpixel_offset)`, éviction LRU par page.
- Polices : `fontdb` pour la découverte et le fallback par couverture de caractère ;
  `swash` pour la mise en forme et la rastérisation. Variantes regular/bold/italic.
- Glyphes dessinés par le renderer lui-même, indépendamment de la police : caractères
  de boîte (U+2500..U+257F), blocs (U+2580..U+259F), symboles powerline (U+E0B0..U+E0BF).
  Cela garantit une barre d'onglets et des bordures nettes sans police patchée.
- Opacité : la couleur de fond de la fenêtre est rendue avec alpha ; la surface est
  créée avec un mode alpha compositing quand la plateforme le permet, sinon opaque.
- Tests : rendu hors écran dans une texture puis comparaison PNG avec tolérance, via
  l'adaptateur logiciel de wgpu si aucun GPU (CI).

### 3.6 Binaire `rustty`

Responsabilité : assembler, gérer les entrées et le chrome.

- Une `OsWindow` winit contient `Vec<Tab>`, chaque `Tab` un `TabLayout` et une
  `HashMap<WindowId, TermWindow { term, pty, reader_thread }>`.
- **Barre d'onglets** : composant dédié qui calcule, à chaque mise en page, le
  rectangle de chaque onglet et celui de son bouton de fermeture (4 cellules : espace,
  demi-cercle gauche, `✕`, demi-cercle droit). État `hovered: Option<HoverTarget>` où
  `HoverTarget ∈ { Tab(id), CloseButton(id), NewTabButton }`. Au mouvement de souris on
  recalcule la cible et on redessine seulement si elle change.
  Clic gauche : titre → activer ; bouton → fermer. Clic du milieu → fermer. Double
  clic → renommer (v0.2). Glisser → réordonner.
- **Entrées clavier** : encodage xterm classique (flèches, fonctions, modificateurs,
  mode application), bracketed paste, raccourcis résolus avant encodage.
- **Souris** : sélection par glisser avec copie dans le presse-papiers, molette pour le
  scrollback, transmission des événements aux applications en mode souris (X10,
  normal, bouton, any-event, encodage SGR).
- Presse-papiers via `arboard`. Titre de fenêtre = titre du terminal actif + version.
- Journalisation `tracing`, niveau via `RUSTTY_LOG`.

## 4. Flux détaillés

### Frappe → écran
1. winit `KeyboardInput` → `keymap.resolve(mods, key)`.
2. Si action : exécution (split, onglet, opacité…). Sinon encodage → `pty.write`.
3. Le shell répond → thread lecteur → `vte` → `Term` muté → `proxy.send_event(Redraw(window))`.
4. Thread UI : instantané → `renderer.draw(snapshot, chrome)` → `surface.present()`.

### Fermeture d'onglet par le bouton
1. `CursorMoved` → `tab_bar.hit_test(x, y)` → `HoverTarget::CloseButton(id)` → redessin.
2. `MouseInput(Left, Released)` sur la même cible → `close_tab(id)`.
3. Si des processus enfants tournent et `confirm_close_with_running_children`, bandeau de
   confirmation ; sinon fermeture, les PTY sont tués, le layout retire les feuilles.
4. Dernier onglet fermé → fermeture de la fenêtre système ; dernière fenêtre → sortie.

### Redimensionnement
1. `Resized` → recalcul des rectangles via `rustty-layout` → pour chaque fenêtre
   `term.resize(cols, rows)` + `pty.resize(...)` → redessin.

## 5. Gestion des erreurs

- Bibliothèques : erreurs typées `thiserror`. Binaire : `anyhow` avec contexte.
- Échec de création du GPU/surface : message clair et code de sortie 1 (pas de repli
  logiciel en v0.1).
- Config invalide au démarrage : défauts + bandeau d'erreur ; au rechargement : ancienne
  config conservée + bandeau.
- Séquence d'échappement inconnue : ignorée, journalisée en `debug`.
- Mort du shell : la fenêtre se ferme (ou reste avec un message si `--hold`).
- Panique dans un thread lecteur : capturée, la fenêtre affiche l'erreur, le reste de
  l'application continue.

## 6. Stratégie de tests

| Crate | Type | Outil |
|---|---|---|
| rustty-vt | Unitaires + fixtures `(entrée, grille attendue)` ; cas vttest ciblés | `cargo test`, `insta` pour les snapshots de grille |
| rustty-layout | Unitaires + propriétés (couverture, non-chevauchement) | `proptest` |
| rustty-config | Fichiers valides/invalides, messages d'erreur, défauts complets | `cargo test` |
| rustty-pty | Intégration : lance `sh -c 'echo ok'`, lit la sortie, vérifie le code de sortie | `cargo test` (Unix + Windows) |
| rustty-render | Rendu hors écran → PNG, comparaison avec tolérance | wgpu headless, `image` |
| rustty (bin) | Tests du composant barre d'onglets (hit test, survol) ; parcours E2E manuel scripté dans `docs/e2e.md` | `cargo test` + vérification manuelle |

CI GitHub Actions : `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` sur
`ubuntu-latest`, `macos-latest`, `windows-latest`. Le dépôt étant privé, la CI
consomme des minutes privées : garder la matrice minimale.

## 7. Versionnage et publication

- Version unique dans `[workspace.package] version`, héritée par toutes les crates.
- Chaque commit : bump SemVer (patch/minor/major selon la nature), entrée en tête de
  `CHANGELOG.md` au format `## X.Y.Z — AAAA-MM-JJ · « titre »`, version affichée dans
  le titre de fenêtre et `rustty --version`.
- Tags annotés `vX.Y.Z` après CI verte ; releases GitHub avec binaires des trois OS
  à partir de la v0.1.
- Passage en public à la v0.1, puis mise à jour des vitrines (profil GitHub,
  yrbane.github.io, captures) conformément aux règles de l'auteur.

## 8. Jalons

- **v0.1** (ce document) : tout ce qui précède.
- **v0.2** : protocole clavier kitty, hyperliens OSC 8, rewrap au resize, renommage
  d'onglet, terminfo dédié, ligatures.
- **v0.3** : protocole graphique (images dans le terminal), contrôle distant par socket
  Unix / pipe nommé, sessions (layout au démarrage).
- Plus tard : layouts supplémentaires (grille, tall, fat), thèmes importables, kittens
  équivalents (`icat`, `diff`).

## 9. Hors périmètre explicite

Compatibilité `kitty.conf`, exécution des kittens Go de kitty, parité avec le remote
control de kitty, rendu CPU de secours, support de terminaux série.
