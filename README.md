<p align="center">
  <img src="assets/logo.png" width="320" alt="Mascotte rustty : un crabe orange souriant dans une fenêtre de terminal">
</p>

# rustty

Émulateur de terminal accéléré par GPU, écrit en Rust, inspiré de
[kitty](https://sw.kovidgoyal.net/kitty/).

> Statut : alpha utilisable. Le binaire `rustty` ouvre une fenêtre GPU, lance
> le shell, gère onglets (bouton ✕ au survol), splits, sélection, presse-papiers,
> scrollback, opacité et rechargement de la configuration à chaud. Les tests
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

## Lancer

```bash
cargo run -p rustty                      # fenêtre avec le shell par défaut
cargo run -p rustty -- --config ~/r.toml # autre fichier de configuration
cargo run -p rustty -- --hold            # garder le panneau quand le shell sort
RUSTTY_LOG=debug cargo run -p rustty     # journal détaillé
```

Raccourcis par défaut (modifiables dans `[keys]`) : `ctrl+shift+t` nouvel onglet,
`ctrl+shift+q` fermer l'onglet, `ctrl+shift+e` / `ctrl+shift+o` split vertical /
horizontal, `shift+flèches` focus, `ctrl+flèches` redimensionner, `ctrl+shift+c` /
`ctrl+shift+v` copier / coller, `shift+page_up` / `shift+page_down` historique, `ctrl+shift+left` / `ctrl+shift+right` onglet précédent / suivant, `alt+1`…`alt+5` onglet n, `ctrl+shift+z` zoom, `ctrl+shift+r` rotation, `ctrl+shift+f5` recharger la config.

## Objectifs de la v0.1

- Fenêtre rendue par le GPU via `wgpu`, sur Linux (Wayland et X11), macOS et Windows.
- Émulation VT solide avec scrollback, couleurs vraies, polices et emojis.
- Onglets avec bouton de fermeture cliquable et réactif au survol.
- Divisions horizontales et verticales de l'écran, redimensionnables au clavier.
- Opacité du fond, configuration en TOML rechargée à chaud, raccourcis configurables.

## Configuration

Fichier TOML, `~/.config/rustty/rustty.toml` sur Linux (équivalents macOS et
Windows). Toutes les clés sont optionnelles ; le fichier d'exemple
[`docs/rustty.example.toml`](docs/rustty.example.toml) liste chaque clé avec
sa valeur par défaut et les raccourcis fournis.

## Pile technique

`winit` · `wgpu` · `portable-pty` · `vte` · `fontdb` · `swash`

## Licence

GPL-3.0, voir [LICENSE](LICENSE).
