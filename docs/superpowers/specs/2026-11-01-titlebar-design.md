# Barre de titre maison

Date : 2026-11-01

## Intention

Sous GNOME Wayland, le compositeur ne décore pas les fenêtres : winit dessine
lui-même une barre de titre Adwaita, claire par défaut, grise depuis
`decorations_theme = "dark"` (alpha.137). Elle reste épaisse, non
personnalisable, et sans aucune action propre à rustty.

Rustty retire les décorations système et dessine **sa propre barre de titre**
au-dessus de la barre d'onglets : fine, noire avec un léger reflet, portant des
icônes d'action (Material Symbols) et les trois boutons de fenêtre. Tout son
aspect vient du fichier TOML. Un objectif secondaire : donner une **couleur de
bandeau par fenêtre** pour distinguer d'un coup d'œil plusieurs instances de
rustty ouvertes en même temps.

Maquette retenue (variante B du brainstorming, `.superpowers/brainstorm/…/barre-style.html`) :
barre de 28 px, fond quasi noir avec liseré clair en haut et dégradé subtil,
icônes plates à gauche, titre centré, pastilles rondes jaune / verte / rouge à
droite dont l'icône n'apparaît qu'au survol.

## Ce que fait rustty

### Disposition

```
┌──────────────────────────────────────────────────────────────┐
│ [⚙] [🎨] [📁]        titre de la fenêtre          (–) (□) (×) │  barre de titre
├──────────────────────────────────────────────────────────────┤  séparateur 1 px
│ 1: zsh │ 2: cargo │ +                                        │  barre d'onglets (si visible)
├──────────────────────────────────────────────────────────────┤
│ panneaux…                                                     │
```

1. La barre de titre occupe toute la largeur, **toujours en haut**, même quand
   la barre d'onglets est en bas. Sa hauteur est réservée par
   `geometry::window_geometry` comme l'est celle de la barre d'onglets ; le
   bandeau d'erreur et les panneaux descendent d'autant.
2. De gauche à droite : trois boutons d'action (réglages, couleur, dossier),
   une zone de titre élastique, trois boutons de fenêtre (réduire, agrandir ou
   restaurer, fermer) calés à droite. Espacement et marges configurables.
3. Un séparateur de 1 px, couleur configurable, sépare la barre de titre de ce
   qui suit.
4. `[titlebar] enabled = false` remet les décorations de winit (avec
   `decorations_theme`) et retire la barre : rien d'autre ne change.

### Aspect

- **Fond** : couleur `background`, surmontée d'un reflet : un liseré de 1 px
  blanc à l'alpha `gloss` en haut, et un dégradé vertical de `background`
  éclairci de `gloss` vers `background`. `gloss = 0` donne une barre plate.
- **Boutons** : chaque bouton est un rectangle de `size` px de côté avec
  `background`, `border_color`, `border_width`, `radius` (un rayon ≥ size/2
  donne un disque), `icon_color`, et les mêmes clés au survol
  (`hover_background`, `hover_border_color`, `hover_icon_color`). Un style
  commun `[titlebar.button]` sert de base ; `[titlebar.buttons.<nom>]`
  surcharge par bouton, `<nom>` ∈ `settings`, `color`, `folder`, `minimize`,
  `maximize`, `close`. Chaque bouton a aussi `visible = true`.
- **Couleurs avec alpha** : les clés de couleur de cette section acceptent
  `#rrggbb` et `#rrggbbaa` (une icône transparente hors survol, un fond
  blanc à 10 %…). Le type `Rgb` existant reste inchangé ; la section utilise
  un type `Rgba` qui désérialise les deux formes.
- **Titre** : texte centré dans la zone restante, police et taille de la
  barre d'onglets, couleur `title_color`, tronqué avec `…` s'il déborde.
- **Lisibilité.** Quand le fond effectif de la barre (couleur de session ou
  `background`) est clair, la couleur du titre et celle des icônes des
  boutons `settings`, `color`, `folder` sont remplacées par la couleur
  lisible calculée par `readable_on`, comme le texte des onglets colorés.
  Les boutons de fenêtre gardent leur style.
- **Unités.** Toutes les tailles de la section sont en **pixels logiques**
  (ceux de la maquette), multipliés par le `scale_factor` de la fenêtre au
  moment de la mise en page et à chaque `ScaleFactorChanged`. Les bornes de
  validation s'entendent dans cette unité.
- **Icônes** : Material Symbols Rounded, taille `icon_size` px. Glyphes :
  `settings`, `palette`, `folder_open`, `remove` (réduire), `crop_square`
  (agrandir) / `filter_none` (restaurer), `close`.
- **Défauts** (variante B) : `height = 28`, `background = "#141414"`,
  `gloss = 0.10`, `separator_color = "#000000"`, `title_color = "#d8d8d8"`,
  `icon_size = 14`, `resize_border = 6`. Bouton commun : fond transparent,
  bordure 0, rayon 5, icône `#bdbdbd`, survol fond `#ffffff1a`, icône
  `#ffffff`. `minimize` / `maximize` / `close` : `size = 14`, `radius = 7`,
  fonds `#f6bd3b` / `#3dc84f` / `#ff5f57`, bordure 1 px `#00000099`, icône
  `#00000000` hors survol et `#111111` au survol, le fond ne change pas au
  survol.

### Actions

| Bouton | Clic | Action `[keys]` | Raccourci par défaut |
|---|---|---|---|
| réglages | ouvre le TOML dans l'éditeur | `open_config` | aucun |
| couleur | ouvre le nuancier sous le bouton | `pick_titlebar_color` | aucun |
| dossier | ouvre le dossier courant du shell | `open_cwd` | aucun |
| titre (double clic) | renomme la fenêtre | `rename_window` | `ctrl+shift+alt+w` |
| réduire | `set_minimized(true)` | `minimize_window` | aucun |
| agrandir | bascule `set_maximized` | `toggle_maximize` | aucun |
| fermer | même chemin que `CloseRequested` (confirmation si enfants) | aucune action : `close_window` existant ferme le panneau ; le bouton passe par `request_close_window` | aucun |

- **Réglages.** Commande `[titlebar] editor` (défaut `"code"`). Si la clé est
  vide : `$VISUAL`, puis `$EDITOR`, puis l'ouvreur du bureau (`xdg-open`,
  `open`, `start`). Le fichier est créé depuis le modèle s'il n'existe pas
  (même code que `--init-config`). Le processus est lancé détaché, sans
  attente, sortie ignorée ; échec de lancement → bandeau d'erreur existant.
- **Dossier.** `TermWindow::cwd()` du panneau qui a le focus ; commande
  `[titlebar] file_manager` (défaut vide = ouvreur du bureau avec le chemin).
  Pas de cwd lisible → le dossier personnel.
- **Couleur.** Le nuancier existant (`ContextMenu::palette`, 12 pastilles
  `ACCENT_SLOTS`) s'ouvre sous le bouton avec une cible `MenuTarget::TitleBar`.
  Un clic sur une pastille donne à **cette fenêtre, pour la session**, la
  couleur de fond correspondante (`Model.titlebar_color: Option<Rgb>`), par
  dessus `background`. Une entrée « Par défaut » en bas du nuancier écrit la
  couleur résolue dans `[titlebar] background` du TOML, efface la couleur de
  session de cette fenêtre et laisse le rechargement à chaud faire le reste.
  Une entrée « Couleur du fichier » retire la couleur de session.
- **Titre.** Le texte de la barre est exactement celui donné à winit :
  `title::window_title(raw, VERSION)`, donc avec le suffixe « — rustty X »,
  où `raw` est le nom fixe de la fenêtre s'il existe, sinon le titre affiché
  de l'onglet actif (son nom personnalisé s'il en a un, ce que `update_title`
  ignore aujourd'hui : à corriger au passage), sinon rien. Double clic → édition en place
  avec le mécanisme `Rename` existant (nouvelle cible `Rename::Window`,
  curseur `▌`, Entrée valide, Échap annule, 64 caractères). Un nom vide rend
  la main au titre du shell. Le titre affiché dans la barre et le titre donné
  à winit (barre des tâches, alt-tab) sont le même texte.
- **Agrandir / restaurer** : l'icône suit `window.is_maximized()`, relue
  après chaque `WindowEvent::Resized`.

### Déplacer et redimensionner

- **Bords.** Quand la fenêtre n'est pas agrandie, une marge de `resize_border`
  px sur le pourtour (coins compris) change le curseur (`EwResize`,
  `NsResize`, `NwseResize`, `NeswResize`) et un appui appelle
  `window.drag_resize_window(direction)`. Cette marge a priorité sur tout le
  reste de la fenêtre. `NotSupported` (macOS) est ignoré silencieusement.
- **Déplacement.** Un appui sur le fond de la barre ou sur le titre arme un
  déplacement : au-delà de 4 px de mouvement, `window.drag_window()` et le
  compositeur prend la main. Un relâchement sans mouvement est un clic, deux
  clics sur le titre dans le délai de double clic ouvrent le renommage. Les
  boutons ne déplacent jamais la fenêtre.
- Les raccourcis du bureau (Super+glisser, Alt+F8) continuent de fonctionner.

### Priorité des appuis

Dans `on_press`, du plus prioritaire au moins : marge de redimensionnement ;
glisser en cours ; menu ou nuancier ouvert ; infobulle d'orthographe ; barre
de titre (bouton, puis titre ou fond) ; barre d'onglets ; en-têtes de panneau ;
séparateurs de split ; panneau. Un appui sur la barre de titre ferme le menu
contextuel et annule un renommage d'onglet en cours, comme un appui ailleurs.

## Rendu

- **Primitive de quad stylé.** Le pipeline `quad` ne sait dessiner que des
  rectangles pleins. L'instance gagne `radius`, `border_width`,
  `border_color` et `color_bottom` (dégradé vertical, `color` en haut). Le
  shader calcule la distance signée au rectangle arrondi pour l'antialiasing
  du contour, et mélange la bordure. Avec `radius = 0`, `border_width = 0`,
  `color_bottom = color`, la sortie est identique au pixel près à l'actuelle :
  les images de référence existantes (`tests/golden/tab_bar.png`) ne bougent
  pas.
- **Icônes.** Police `crates/rustty-render/assets/MaterialSymbolsRounded-subset.ttf`
  (Apache 2.0, `LICENSE-MaterialSymbols` à côté), réduite aux sept glyphes utilisés, régénérée par
  `scripts/material-icons.sh` (fonttools dans un venv jetable, comme
  `scripts/icons.sh` pour l'icône d'application). Elle est chargée depuis les
  octets embarqués dans une face **dédiée** : les codepoints Material sont en
  zone privée, où le repli actuel prendrait la première police système qui
  les couvre (Nerd Font). Chaque `FontSet` charge la face Material à
  `icon_size` px logiques × `scale_factor`, indépendamment de sa taille de
  texte ; `ChromeText` indique la face voulue (`Text` ou `Icon`) et la clé du
  cache de glyphes inclut la face. Les icônes partent donc dans la passe de
  base avec le reste du chrome : pas de passe ni de renderer supplémentaire.
- **Titre et boutons** passent par `Chrome { quads, texts }` comme la barre
  d'onglets : `title_bar_chrome(layout, style, state)` est une fonction pure
  dans `rustty-render/src/chrome/title_bar.rs`, `layout_title_bar` produit
  `TitleBarLayout { bar, buttons: Vec<(TitleButton, Rect)>, title: Rect }`
  avec `hit_test → TitleTarget::{None, Button(TitleButton), Title, Background}`.

## Configuration

```toml
[titlebar]
enabled = true
height = 28
background = "#141414"
gloss = 0.10
separator_color = "#000000"
title_color = "#d8d8d8"
icon_size = 14
padding = 6            # marge gauche/droite
spacing = 4            # entre boutons
resize_border = 6
editor = "code"        # vide : $VISUAL, $EDITOR, puis ouvreur du bureau
file_manager = ""      # vide : ouvreur du bureau

[titlebar.button]      # style commun
size = 22
background = "#00000000"
border_color = "#00000000"
border_width = 0
radius = 5
icon_color = "#bdbdbd"
hover_background = "#ffffff1a"
hover_border_color = "#00000000"
hover_icon_color = "#ffffff"

[titlebar.buttons.settings]   # idem color, folder : visible = true, rien d'autre
visible = true

[titlebar.buttons.minimize]
size = 14
radius = 7
background = "#f6bd3b"
hover_background = "#f6bd3b"
border_width = 1
border_color = "#00000099"
icon_color = "#00000000"
hover_icon_color = "#111111"
# maximize : "#3dc84f" ; close : "#ff5f57"
```

- Validation : `height` 16 à 64, `gloss` 0 à 1, `icon_size` 8 à 32,
  `size` 8 à `height`, `radius` 0 à 32, `border_width` 0 à 4,
  `resize_border` 0 à 16. Clé inconnue → erreur positionnée, comme ailleurs.
- Rechargement à chaud : tout change sans redémarrer, `enabled` compris
  (`set_decorations`).
- Modèle `--init-config`, `docs/rustty.example.toml`, README et le test
  `every_section_and_key_is_documented` couvrent chaque clé.

### Écriture dans le TOML

Seul « Par défaut » du nuancier écrit. La dépendance `toml_edit` est ajoutée
pour ne toucher que la clé `[titlebar] background` et garder commentaires,
ordre et formatage. Fichier absent → créé depuis le modèle d'abord. Écriture
atomique (fichier temporaire puis renommage). Le veilleur de config déclenche
le rechargement normal ; un échec d'écriture passe par le bandeau d'erreur.

## Non-objectifs

- Pas de vrai plein écran ; pas de menu de fenêtre (`show_window_menu`).
- Pas d'icônes au choix par bouton, pas de boutons supplémentaires, pas
  d'ordre configurable : le jeu de six est fixe, seul `visible` varie.
- Pas de barre de titre masquée automatiquement en mode agrandi.
- Pas d'écran de réglages dans rustty : « réglages » ouvre le fichier.
- Pas de gestion multi-fenêtres dans un même processus : plusieurs fenêtres =
  plusieurs processus, la couleur de session vit dans chacun.
- Pas de test manuel sous Windows ni macOS : le code compile partout, seul
  Linux (GNOME Wayland, et X11 si possible) est vérifié.

## Anti-objectifs

- Toucher au réglage clair/sombre du bureau : jamais.
- Changer d'un pixel la barre d'onglets ou les panneaux quand `[titlebar]`
  est à ses défauts : les images de référence existantes doivent rester
  vertes.
- Bloquer l'interface en lançant l'éditeur ou l'explorateur.
- Écraser ou reformater le TOML de l'utilisateur au-delà de la clé visée.

## Tests

- `rustty-config` : analyse de la section, défauts, validation des bornes,
  `Rgba` sous ses deux formes, actions `[keys]` nouvelles.
- `rustty-render` : `layout_title_bar` et `hit_test` (fonctions pures,
  boutons masqués, largeur étroite), `title_bar_chrome` (quads et textes
  attendus pour un état donné : survol, agrandi, nom fixe tronqué), image de
  référence `tests/golden/title_bar.png` dans `tests/offscreen.rs`, image de
  référence du quad stylé (rayon, bordure, dégradé).
- `rustty` : direction de redimensionnement en fonction du point et de la
  taille (fonction pure), seuil de déplacement et double clic sur le titre
  (machine pure comme `drag.rs`), résolution de la commande d'éditeur selon
  clé et variables d'environnement, écriture de `background` qui conserve
  commentaires et autres clés, `Rename::Window`.
- `docs/e2e.md` : nouvelle section « Barre de titre » (chaque bouton, survol,
  déplacement, bords, renommage, nuancier et « Par défaut », `enabled = false`,
  rechargement à chaud).

## Jalons

Chaque jalon est une version publiable, dans cet ordre :

1. **Barre et fenêtre** : quad stylé, section de config minimale (`enabled`,
   `height`, couleurs, `gloss`), barre dessinée avec le titre et les trois
   boutons de fenêtre (fonctionnels), décorations retirées, déplacement et
   bords de redimensionnement.
2. **Icônes et actions** : police Material embarquée, boutons réglages,
   dossier, styles de boutons complets, actions `[keys]`.
3. **Titre et couleur** : renommage de la fenêtre, nuancier, couleur de
   session, « Par défaut » et `toml_edit`.
4. **Finitions** : documentation, E2E, captures et vitrines publiques.

## Laissé au constructeur

- Le découpage exact des modules dans `crates/rustty/src` (un `title_input.rs`
  à côté de `drag_input.rs` paraît naturel) et la forme de `TitleBarStyle`.
- La formule exacte du reflet (hauteur du liseré, courbe du dégradé) tant que
  `gloss = 0` donne une barre plate et que le défaut ressemble à la maquette.
- Le comportement quand la fenêtre est trop étroite pour tout afficher : les
  boutons gardent leur place, le titre rétrécit jusqu'à disparaître.
- La façon d'obtenir l'état agrandi initial et de le suivre.
