# Changelog

Toutes les évolutions notables de rustty sont consignées ici. Le format suit
[Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet respecte
[SemVer](https://semver.org/lang/fr/).

## 0.1.0-alpha.22 — 2026-10-08 · « Divisions »

- `rustty-layout` : `TabLayout::split` horizontal et vertical, rectangles avec ratio et espace entre panneaux, bornes dégénérées sans panique.

## 0.1.0-alpha.21 — 2026-10-08 · « Crate layout »

- `rustty-layout` : géométrie (`Rect`, `Axis`, `Direction`, `WindowId`) et `TabLayout` à une fenêtre avec focus.

## 0.1.0-alpha.20 — 2026-10-08 · « Plan layout et config »

- Plan d'implémentation des crates `rustty-layout` (arbre de divisions, voisinage, zoom, invariants par propriétés) et `rustty-config` (sections TOML, couleurs, raccourcis, actions, chargement, fichier d'exemple), douze tâches.

## 0.1.0-alpha.19 — 2026-10-08 · « Icône carrée »

- `assets/icon.svg` : viewBox carré 840 × 840, dessin d'origine centré verticalement, prêt pour la génération des icônes ICO, ICNS et PNG.

## 0.1.0-alpha.18 — 2026-10-08 · « Mascotte et icône »

- Identité visuelle : mascotte crabe en tête du README (`assets/logo.png`, version 512 px ; original 1254 px dans `assets/logo-1254.png`) et icône vectorielle de l'application (`assets/icon.svg`), utilisée par le futur binaire.
- `res/` devient la zone de dépôt des visuels bruts, hors git.

## 0.1.0-alpha.17 — 2026-10-08 · « Corrections de revue »

- `rustty-vt` : le resize des rangées garde la ligne du curseur visible en échangeant des lignes avec l'historique ; la vue reste stable quand de la sortie arrive pendant la relecture de l'historique ; nouvel événement `TermEvent::ModeChanged` pour les modes qui concernent l'hôte ; une couleur SGR étendue tronquée ne pose plus d'attributs parasites ; les titres OSC gardent leurs points-virgules ; RIS rafraîchit le titre et ED 3 remet le décalage d'affichage à zéro.

## 0.1.0-alpha.16 — 2026-10-08 · « Instantané et défilement de l'affichage »

- `rustty-vt` : `Snapshot` pour le renderer, défilement de l'affichage dans l'historique, scénarios de bout en bout figés par insta, test de robustesse sur du bruit.
- README : statut et commandes de développement.

## 0.1.0-alpha.15 — 2026-10-08 · « Redimensionnement »

- `rustty-vt` : `Term::resize` borne curseurs et région, redimensionne grilles et historique, sans rewrap.

## 0.1.0-alpha.14 — 2026-10-08 · « Caractères larges »

- `rustty-vt` : caractères CJK et emojis sur deux cellules, retour à la ligne en fin de ligne, nettoyage des moitiés orphelines, combinants attachés à la première moitié.

## 0.1.0-alpha.13 — 2026-10-08 · « Séquences ESC et réinitialisation »

- `rustty-vt` : IND, RI, NEL, HTS/TBC, désignation G0/G1 du jeu graphique DEC, RIS.

## 0.1.0-alpha.12 — 2026-10-08 · « Titre, presse-papiers et requêtes »

- `rustty-vt` : titre de fenêtre (OSC 0/2), écriture du presse-papiers (OSC 52, lecture refusée), réponses DA1, DA2, DSR 5 et 6.

## 0.1.0-alpha.11 — 2026-10-08 · « Modes et écran alternatif »

- `rustty-vt` : modes DEC (touches application, origine, autowrap, curseur, souris, SGR souris, focus, collage encadré), écrans alternatifs 47/1047/1049, modes ANSI insertion et LNM, forme du curseur DECSCUSR.

## 0.1.0-alpha.10 — 2026-10-08 · « Attributs SGR »

- `rustty-vt` : gras, atténué, italique, souligné (avec sous-paramètres), clignotant, inversé, caché, barré, couleurs 16, 256 et vraies couleurs dans les deux syntaxes.

## 0.1.0-alpha.9 — 2026-10-08 · « Effacement et édition »

- `rustty-vt` : ED (dont effacement de l'historique), EL, ECH, ICH, DCH, IL, DL, SU, SD, avec respect de la région de défilement.

## 0.1.0-alpha.8 — 2026-10-08 · « Déplacements du curseur »

- `rustty-vt` : lecture des paramètres CSI ; CUU/CUD/CUF/CUB/CNL/CPL/CHA/VPA/CUP/HVP, DECSTBM, mode origine, DECSC/DECRC et CSI s/u.

## 0.1.0-alpha.7 — 2026-10-08 · « Cœur du terminal »

- `rustty-vt` : façade `Term` pilotée par `vte` avec dispatch séparé de la logique ; impression de texte, retour à la ligne différé, mode insertion, combinants, contrôles C0, défilement vers l'historique.

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
