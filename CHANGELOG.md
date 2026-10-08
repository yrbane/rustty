# Changelog

Toutes les évolutions notables de rustty sont consignées ici. Le format suit
[Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet respecte
[SemVer](https://semver.org/lang/fr/).

## 0.1.0-alpha.6 — 2026-10-08 · « Objets-valeur du terminal »

- `rustty-vt` : `Cursor`, `CursorShape`, `SavedCursor`, `Modes`, `Charset`/`Charsets` (table graphique DEC), `TabStops`, `ScrollRegion`, `Outbox`, chacun testé isolément.

## 0.1.0-alpha.5 — 2026-10-08 · « Scrollback »

- `rustty-vt` : `Scrollback` borné, indexé depuis la ligne la plus récente.

## 0.1.0-alpha.4 — 2026-10-08 · « Grille »

- `rustty-vt` : `Grid` avec défilement par région dans les deux sens, effacement et redimensionnement.

## 0.1.0-alpha.3 — 2026-10-08 · « Ligne de grille »

- `rustty-vt` : `Line` avec insertion, suppression, effacement, redimensionnement et table annexe des caractères combinants.

## 0.1.0-alpha.2 — 2026-10-08 · « Cellule et style »

- `rustty-vt` : types `Color`, `Attrs`, `Style`, `Cell` ; cellule effacée conservant les couleurs.

## 0.1.0-alpha.1 — 2026-10-08 · « Workspace et CI »

- Workspace Cargo (edition 2024, version unique), crate `rustty-vt` vide exposant `VERSION`.
- CI GitHub Actions : fmt, clippy en mode strict, tests sur Linux, macOS et Windows.

## 0.0.3 — 2026-10-08 · « Plan modulaire »

- Plan des fondations restructuré en seize tâches : objets-valeur isolés (`Cursor`, `Modes`, `Charsets`, `TabStops`, `ScrollRegion`, `Outbox`), façade `Term` composée, dispatch `vte::Perform` séparé de la logique, un module par famille d'opérations.

## 0.0.2 — 2026-10-08 · « Plan des fondations »

- Plan d'implémentation détaillé du workspace, de la CI et de la crate `rustty-vt` en quinze tâches, tests avant code.

## 0.0.1 — 2026-10-08 · « Naissance du projet »

- Spécification de conception de la v0.1 : intention, architecture en six crates,
  flux de données, configuration TOML, stratégie de tests, jalons.
- README, licence GPL-3.0, journal des modifications.
