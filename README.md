# rustty

Émulateur de terminal accéléré par GPU, écrit en Rust, inspiré de
[kitty](https://sw.kovidgoyal.net/kitty/).

> Statut : en conception. La v0.1 est en cours de construction, voir la
> [spécification](docs/superpowers/specs/2026-10-08-rustty-design.md).

## Objectifs de la v0.1

- Fenêtre rendue par le GPU via `wgpu`, sur Linux (Wayland et X11), macOS et Windows.
- Émulation VT solide avec scrollback, couleurs vraies, polices et emojis.
- Onglets avec bouton de fermeture cliquable et réactif au survol.
- Divisions horizontales et verticales de l'écran, redimensionnables au clavier.
- Opacité du fond, configuration en TOML rechargée à chaud, raccourcis configurables.

## Pile technique

`winit` · `wgpu` · `portable-pty` · `vte` · `fontdb` · `swash`

## Licence

GPL-3.0, voir [LICENSE](LICENSE).
