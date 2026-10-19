<p align="center">
  <img src="assets/logo.png" width="320" alt="Mascotte rustty : un crabe orange souriant dans une fenêtre de terminal">
</p>

# rustty

Émulateur de terminal accéléré par GPU, écrit en Rust, inspiré de
[kitty](https://sw.kovidgoyal.net/kitty/).

> Statut : alpha utilisable. Le binaire `rustty` ouvre une fenêtre GPU, lance
> le shell, gère onglets (bouton ✕ au survol), splits, sélection, presse-papiers,
> scrollback, opacité, barres de split et couleurs d'onglets configurables,
> renommage des onglets, zoom de police et rechargement de la configuration à
> chaud. Les panneaux d'un onglet divisé portent un bandeau et se déplacent à
> la souris ; les onglets se réordonnent en les glissant. Les tests
> couvrent toute la logique ; le parcours manuel est dans `docs/e2e.md`.

## Développement

```bash
cargo test --workspace                       # tous les tests
cargo clippy --workspace --all-targets -- -D warnings
cargo insta review                           # accepter les snapshots de grille modifiés
```

Les tests de rendu comparent des images de référence (`crates/rustty-render/tests/golden/`)
produites avec la police embarquée DejaVu Sans Mono. Pour les régénérer après un
changement voulu du rendu : `UPDATE_GOLDEN=1 cargo test -p rustty-render --test offscreen`,
puis vérifier les PNG à l'œil avant de les committer.

## Installer

```bash
cargo install --path crates/rustty                          # depuis un clone du dépôt
cargo install --git https://github.com/yrbane/rustty rustty # sans cloner
rustty --install-desktop                                    # Linux : lanceur et icône dans les menus
rustty --init-config                                        # fichier de configuration à éditer
```

Le binaire va dans `~/.cargo/bin` (à mettre dans le `PATH`). `--init-config`
écrit `~/.config/rustty/rustty.toml`, complet et commenté : les raccourcis se
redéfinissent dans sa section `[keys]` (`"none"` en délie un), et la
configuration se recharge à chaud.

## Lancer

```bash
cargo run -p rustty                      # fenêtre avec le shell par défaut
cargo run -p rustty -- --config ~/r.toml # autre fichier de configuration
cargo run -p rustty -- --hold            # garder le panneau quand le shell sort
RUSTTY_LOG=rustty=debug cargo run -p rustty  # journal détaillé
cargo run -p rustty -- --install-desktop # Linux : lanceur + icône dans les menus et la barre des tâches
```

Sous Linux, `--install-desktop` écrit `rustty.desktop` et les icônes sous
`~/.local/share` : GNOME, KDE et les autres associent alors la fenêtre
(identifiant `rustty`) à son icône. Sous Windows, l'icône est embarquée dans
l'exécutable. Les icônes se régénèrent depuis `assets/icon.svg` avec
`scripts/icons.sh`.

### Raccourcis

Raccourcis par défaut (modifiables dans `[keys]`) : `ctrl+shift+t` nouvel onglet,
`ctrl+shift+q` fermer l'onglet, `ctrl+shift+e` / `ctrl+shift+o` split vertical /
horizontal, `shift+flèches` focus, `ctrl+flèches` redimensionner, `ctrl+shift+c` /
`ctrl+shift+v` copier / coller, `shift+page_up` / `shift+page_down` historique, `ctrl+shift+left` / `ctrl+shift+right` ou `ctrl+shift+tab` / `ctrl+tab` onglet précédent / suivant, `alt+1`…`alt+5` onglet n, `ctrl+shift+z` zoom, `ctrl+shift+r` rotation, `ctrl+shift+f5` recharger la config, `ctrl+shift+alt+t` (ou double clic sur l'onglet) renommer l'onglet, `ctrl+plus` / `ctrl+minus` / `ctrl+0` et `ctrl+molette` taille de police du panneau survolé (les autres panneaux et la barre d'onglets ne changent pas). Les barres de split se glissent à la souris ; au redimensionnement, les lignes se réorganisent (reflow).

### Souris : bandeaux, glisser-déposer et menus

- **Bandeau de panneau.** Dès qu'un onglet affiche au moins deux panneaux (sans
  zoom), chaque panneau porte en haut un bandeau d'une rangée : son titre (le
  nom donné au panneau, sinon le titre annoncé par le shell, tronqué avec `…`)
  à gauche, et à droite un bouton ✕ qui apparaît au survol (masqué sur un
  bandeau de moins de 6 cellules) et ferme le panneau. Il prend la couleur de
  la division qui contient le panneau (réglages `[splits]`). Un onglet à un
  seul panneau, ou un panneau agrandi, n'a pas de bandeau. Un clic sur le
  bandeau focalise le panneau, un double clic le renomme ; le bandeau n'est
  jamais transmis aux applications qui captent la souris.
- **Déplacer un panneau.** On le tire par son bandeau (au-delà de 4 px ; en
  dessous, c'est un clic) : la moitié visée d'un autre panneau (haut, bas,
  gauche ou droite, délimitées par les diagonales) s'éclaire et le panneau s'y
  installe au lâcher. Survoler un onglet 0,5 s l'affiche, on peut alors lâcher
  sur l'un de ses panneaux ; lâcher directement sur un onglet (même le sien)
  place le panneau à droite de tout l'onglet. Le shell n'est jamais tué ni
  relancé. `Échap`, un lâcher hors cible ou sur le panneau lui-même annulent
  et réaffichent l'onglet de départ. Un panneau seul dans son onglet n'a pas
  de bandeau et ne se tire donc pas.
- **Réordonner les onglets.** On tire un onglet le long de la barre ; un repère
  vertical montre où il s'insérera, et l'onglet actif reste le même. Un simple
  clic garde son effet (activation, double clic pour renommer).
- **Menus au clic droit.** Dans un panneau : copier, coller, diviser, agrandir,
  renommer l'onglet, nouvel onglet, fermer le panneau (`shift`+clic droit dans
  une application qui capte la souris). Sur un onglet : renommer, changer la
  couleur, nouvel onglet, fermer, fermer les autres. Sur un bandeau : diviser
  horizontalement ou verticalement, agrandir, déplacer dans un nouvel onglet,
  renommer, fermer le panneau. « Changer la couleur » ouvre un nuancier de
  12 couleurs vives plus « Couleur par défaut » ; le choix s'affiche même avec
  `[tabs.colors] random = false`.

## Afficher une image

```bash
rustty img photo.png                     # fichier local
rustty img https://exemple.org/photo.jpg # URL http(s), 64 Mio au plus
alias img='rustty img'                   # à mettre dans ~/.bashrc ou ~/.zshrc
```

L'image s'affiche dans le panneau courant, via le protocole graphique de kitty ;
elle défile avec le texte, suit le zoom de police et disparaît avec `clear`.
Formats : PNG, JPEG, GIF (première image), WebP, BMP ; l'orientation EXIF des
photos est appliquée. Les images de plus de
2048 px de côté sont réduites avant l'envoi. `rustty img` marche aussi dans kitty.

Côté terminal, chaque image posée est conservée à sa taille affichée (marge ×2
pour le zoom) et les images posées (écran et historique) ne dépassent pas 320 Mio
par terminal : au-delà, les plus anciennes de l'historique disparaissent d'abord.

## Objectifs de la v0.1

- Fenêtre rendue par le GPU via `wgpu`, sur Linux (Wayland et X11), macOS et Windows.
- Émulation VT solide avec scrollback, couleurs vraies, polices et emojis.
- Onglets avec bouton de fermeture cliquable et réactif au survol, réordonnables
  à la souris, avec menu au clic droit et couleur au choix.
- Divisions horizontales et verticales de l'écran, redimensionnables au clavier
  et à la souris ; bandeaux de panneau et glisser-déposer des panneaux.
- Un nouvel onglet ou un nouveau panneau démarre dans le dossier du panneau actif
  (celui du programme au premier plan, sinon du shell ; Linux et macOS — sous
  Windows, le dossier personnel).
- Opacité du fond, configuration en TOML rechargée à chaud, raccourcis configurables.

## Configuration

Nouveautés de personnalisation : `[splits]` (barre entre panneaux avec ou
sans, épaisseur, couleur fixe ou tirée au sort pour chaque division),
`[tabs]` `padding_horizontal`, `padding_vertical`, `spacing`, et `[tabs.colors]`
(couleurs des onglets, ou `random = true` pour une couleur vive tirée au sort
par onglet, texte automatiquement lisible), et `[window] osc52_clipboard`
(`true` par défaut ; `false` empêche les programmes d'écrire dans le
presse-papiers par la séquence OSC 52).

Fichier TOML, `~/.config/rustty/rustty.toml` sur Linux (équivalents macOS et
Windows). Toutes les clés sont optionnelles ; le fichier d'exemple
[`docs/rustty.example.toml`](docs/rustty.example.toml) liste chaque clé avec
sa valeur par défaut et les raccourcis fournis.

## Pile technique

`winit` · `wgpu` · `portable-pty` · `vte` · `fontdb` · `swash`

## Licence

GPL-3.0, voir [LICENSE](LICENSE).
