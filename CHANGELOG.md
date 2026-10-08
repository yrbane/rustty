# Changelog

Toutes les évolutions notables de rustty sont consignées ici. Le format suit
[Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet respecte
[SemVer](https://semver.org/lang/fr/).

## 0.1.0-alpha.40 — 2026-10-08 · « Polices »

- `rustty-render` : `FontSet` charge la famille configurée ou un repli à chasse fixe (quatre variantes), police DejaVu Sans Mono embarquée en dernier recours et pour les tests, métriques de cellule (largeur, hauteur, ligne de base, soulignement, barré).

## 0.1.0-alpha.39 — 2026-10-08 · « Crate render et palette »

- `rustty-render` : `Rgba` et `Palette` (16 couleurs ANSI, cube 6×6×6, gris, vraies couleurs ; gras vif, inversion, atténuation, texte caché).

## 0.1.0-alpha.38 — 2026-10-08 · « Thread lecteur »

- `rustty-pty` : `spawn_reader` lit le pseudo-terminal par blocs de 64 Kio et pousse `PtyEvent::Data` puis `Eof` dans un canal ou un rappel ; 2 000 lignes livrées dans l'ordre sans perte.

## 0.1.0-alpha.37 — 2026-10-08 · « Pseudo-terminal »

- `rustty-pty` : `Pty::spawn` lance le shell dans un pseudo-terminal ; lecture clonable, écriture, redimensionnement, code de sortie, `kill`, et le processus est tué à la destruction. Tests avec un vrai shell sur Unix et Windows.

## 0.1.0-alpha.36 — 2026-10-08 · « Crate pty »

- `rustty-pty` : `Shell` (défaut `$SHELL`, `/bin/sh` ou `powershell.exe`), environnement `TERM`/`COLORTERM`, `PtySize` bornée à une cellule, `PtyError`.

## 0.1.0-alpha.35 — 2026-10-08 · « Plan pty et render »

- Plan d'implémentation des crates `rustty-pty` (shell dans un pseudo-terminal, thread lecteur) et `rustty-render` (palette, polices avec repli et police embarquée, glyphes procéduraux, atlas, pipelines wgpu, renderer testé hors écran contre des images de référence, barre d'onglets), quatorze tâches.

## 0.1.0-alpha.34 — 2026-10-08 · « Corrections de revue »

- `rustty-config` : palette strictement de 16 couleurs ; deux graphies d'un même raccourci dans `[keys]` sont une erreur ; variation d'opacité bornée à ±1 et acceptée en nombre ou en chaîne (`+0.05` ou `"+0.05"`) ; les messages d'erreur d'action conservent le détail (variante inconnue, valeur hors plage).
- `rustty-layout` : un `resize` avec un delta non fini est ignoré au lieu de corrompre le ratio.

## 0.1.0-alpha.33 — 2026-10-08 · « Portabilité Windows »

- Fins de ligne forcées en LF par `.gitattributes` (le fichier d'exemple est comparé octet à octet) ; le test du chemin de configuration par défaut accepte la disposition Windows (`rustty\config\rustty.toml`).

## 0.1.0-alpha.32 — 2026-10-08 · « Fichier d'exemple »

- `rustty-config` : `DEFAULT_TOML`, fichier d'exemple commenté publié dans `docs/rustty.example.toml`, garanti égal aux défauts par les tests.
- README : statut des trois crates et section Configuration.

## 0.1.0-alpha.31 — 2026-10-08 · « Chargement de la configuration »

- `rustty-config` : `Config::load` (fichier absent = défauts, illisible = erreur avec le chemin), chemin par défaut `~/.config/rustty/rustty.toml` et équivalents macOS et Windows.

## 0.1.0-alpha.30 — 2026-10-08 · « Actions et raccourcis »

- `rustty-config` : actions (onglets, divisions, focus, redimensionnement, zoom, rotation, opacité, presse-papiers, défilement, rechargement), table de raccourcis par défaut surchargeable dans `[keys]`, `"none"` pour délier, erreurs nommant le raccourci fautif.

## 0.1.0-alpha.29 — 2026-10-08 · « Combinaisons de touches »

- `rustty-config` : `KeyCombo` analysé depuis `ctrl+shift+t`, alias des modificateurs et des touches nommées, forme canonique, erreurs nommant le raccourci fautif.

## 0.1.0-alpha.28 — 2026-10-08 · « Sections de configuration »

- `rustty-config` : sections `font`, `window`, `tabs` (avec style du bouton de fermeture) et `colors` (palette Catppuccin Mocha par défaut), clés inconnues refusées, erreurs positionnées ligne et colonne, validation des bornes.

## 0.1.0-alpha.27 — 2026-10-08 · « Crate config »

- `rustty-config` : type `Rgb` analysé depuis `#rrggbb`, avec erreurs nommant la valeur fautive.

## 0.1.0-alpha.26 — 2026-10-08 · « Zoom et invariants »

- `rustty-layout` : zoom d'une fenêtre sur tout l'onglet ; tests par propriétés garantissant couverture sans chevauchement, une entrée par fenêtre et focus toujours valide, quelle que soit la suite d'opérations.

## 0.1.0-alpha.25 — 2026-10-08 · « Voisinage »

- `rustty-layout` : `neighbor` choisit la fenêtre adjacente partageant le plus long bord, pour les déplacements de focus au clavier.

## 0.1.0-alpha.24 — 2026-10-08 · « Redimensionnement et rotation »

- `rustty-layout` : `resize` sur la division la plus proche de l'axe demandé, ratio borné à 10–90 % ; `rotate` inverse l'orientation d'une division.

## 0.1.0-alpha.23 — 2026-10-08 · « Fermeture de fenêtre »

- `rustty-layout` : `TabLayout::close` promeut le panneau frère et transfère le focus ; fermer la dernière fenêtre vide l'onglet.

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
