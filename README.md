<p align="center">
  <img src="assets/logo.png" width="320" alt="Mascotte rustty : un crabe orange souriant dans une fenêtre de terminal">
</p>

# rustty

Émulateur de terminal accéléré par GPU, écrit en Rust, inspiré de
[kitty](https://sw.kovidgoyal.net/kitty/).

> Statut : fondations. Cinq crates sont fonctionnelles et testées : `rustty-vt`
> (émulation), `rustty-layout` (onglets et divisions), `rustty-config` (TOML),
> `rustty-pty` (shell dans un pseudo-terminal) et `rustty-render` (rendu wgpu,
> testé hors écran sur les trois OS). Reste le binaire qui les assemble.

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
