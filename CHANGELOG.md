# Changelog

Toutes les évolutions notables de rustty sont consignées ici. Le format suit
[Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet respecte
[SemVer](https://semver.org/lang/fr/).

## 0.1.0-alpha.106 — 2026-10-09 · « rustty img : limites HTTP franches »

- Une réponse HTTP de plus de 64 Mio donne l'erreur « réponse trop volumineuse (plus de 64 Mio) » au lieu d'être tronquée en silence.
- Un statut HTTP d'erreur s'affiche en français (« code HTTP 404 ») et le téléchargement a un délai global de 30 s.
- L'encodage PNG en mémoire remonte une erreur au lieu de paniquer.

## 0.1.0-alpha.105 — 2026-10-09 · « rustty img : afficher une image »

- Nouvelle commande `rustty img <source>` : lit un fichier local ou une URL `http(s)://` (corps limité à 64 Mio), décode (PNG, JPEG, GIF, WebP, BMP), réduit à 2048 px de côté au plus (Lanczos3) et l'émet en séquences graphiques kitty (`a=T`, PNG, base64 par morceaux de 4096 caractères). Fonctionne aussi dans kitty. Rien n'est écrit en cas d'erreur (message en français, code de sortie 1).
- `TermWindow::resize` transmet désormais la taille d'une cellule au `Term` (par panneau, selon sa police), même quand la grille ne change pas, pour calculer les cellules couvertes par une image ; il renvoie vrai quand la grille change.
- Dépendances : `ureq` 3 (HTTP), `base64`, et décodeurs `image` jpeg/gif/webp/bmp.
- Usage et README mis à jour (« Afficher une image »), parcours manuel dans `docs/e2e.md`.

## 0.1.0-alpha.104 — 2026-10-09 · « Images trop grandes pour le GPU ignorées »

- `rustty-render` : une image dont un côté dépasse `max_texture_dimension_2d` du GPU (inférieur à 8192 sur les moteurs GL/GLES) n'est plus envoyée à wgpu, qui paniquait sur l'erreur de validation : elle n'est simplement pas dessinée, et un message le signale une seule fois par image. Une sortie de terminal hostile ne peut plus faire tomber le rendu.
- Nouvelle fonction pure `fits_device(largeur, hauteur, côté_max)` (image vide refusée), testée.

## 0.1.0-alpha.103 — 2026-10-09 · « Dessin des images dans les panneaux »

- `rustty-render` : les bandes d'image des lignes visibles sont dessinées juste après les fonds de cellule, sous le texte. `images::pane_images` (pur) traduit chaque bande en rectangle pixels et coordonnées de texture : une bande couvre au plus une cellule de haut, la dernière d'une image de hauteur fractionnaire est partielle ; une bande qui déborde à droite du panneau est rognée (texture comprise), une bande qui commence au-delà est ignorée, les lignes hors de la grille du panneau aussi.
- Nouveau pipeline `ImagePipeline` (`shaders/image.wgsl`) : un quad instancié par bande, texture de l'image entière en `Rgba8Unorm` (octets passés tels quels comme les couleurs de la palette), échantillonnage linéaire, alpha droit mélangé comme les glyphes couleur.
- Cache `ImageTextures` par identifiant d'image : chaque texture est envoyée une fois, puis oubliée dès qu'une passe porteuse de panneaux ne dessine plus l'image (la passe du bandeau, sans panneau, ne vide pas le cache). `Renderer::image_texture_count` (caché) pour les tests.
- Tests : géométrie des bandes (bandes fractionnaires, rognage à droite, bandes hors panneau, lignes sous la grille), image de référence `image_strip` (PNG 4×2 étiré sur 4×2 cellules) avec contrôle des quatre couleurs, libération des textures.
- Les tests hors écran partagent leurs aides (`tests/support`) ; les tests internes du renderer passent dans `renderer/tests.rs`.

## 0.1.0-alpha.102 — 2026-10-09 · « Curseur sauvegardé périmé sans effet sur le reflow »

- `rustty-vt` : un curseur DECSC périmé (laissé en bas par une application plein écran, puis `clear`) n'étend plus les rangées reprises par le reflow ; seuls le contenu, les images et le curseur vivant les bornent, l'invite n'est plus poussée dans l'historique quand la hauteur diminue en même temps. Un curseur sauvegardé hors de ces rangées est seulement borné, comme avant.
- `reflow` : seul le premier curseur (le vivant) retient les blancs de fin ; un curseur sauvegardé dans la queue blanche ou au-delà de la ligne se pose en fin de texte, sans multiplier les lignes vides.
- Les rangées d'écran faites d'espaces colorées sans attribut comptent comme vides, avec la même règle que le reflow (`reflow::is_blank`).
- Tests ajoutés : plusieurs curseurs sur une même ligne logique, curseur sur une ligne absente (`None`), DECSC périmé, rangées blanches colorées.

## 0.1.0-alpha.101 — 2026-10-09 · « Reflow des images, du curseur sauvegardé et de l'historique plein »

- `rustty-vt` : les bandes d'image survivent au reflow ; sur le chemin rapide la ligne garde toutes ses bandes (même au-delà de la nouvelle largeur, le rendu rogne), et les bandes de toutes les lignes source d'une ligne logique enroulée vont à sa première ligne produite, colonne conservée.
- `reflow` accepte plusieurs curseurs (`&[(ligne, colonne)]`) et rend leurs positions dans le même ordre ; sur l'écran principal, le curseur sauvegardé par `ESC 7` suit son caractère comme le curseur vivant (`ESC 8` y revient après un redimensionnement), retour à la ligne en attente compris.
- Historique plein : après le reflow, les lignes en trop sont retirées en tête jusqu'au début d'une ligne logique, plus aucune ligne d'historique ne commence par la suite d'une ligne enroulée.
- Une espace sans attribut compte comme blanc quelle que soit sa couleur : une queue à fond coloré ne fabrique plus de lignes vides au reflow (son fond est perdu) ; une seconde moitié de caractère large n'est jamais un blanc.
- Les rangées d'écran portant une image sous le curseur ne sont plus perdues au reflow.
- `reset.rs` repassé sous 300 lignes : le redimensionnement vit dans `term/resize.rs` (tests dans `term/resize_tests.rs`), les tests du reflow dans `reflow/tests.rs` et `reflow/perf.rs`.

## 0.1.0-alpha.100 — 2026-10-09 · « Affichage des images du protocole kitty »

- `rustty-vt` : `Term` exécute les commandes graphiques APC (`a=t`, `a=T`, `a=p`, `a=d`) ; l'image est posée à partir du curseur, une bande par rangée, avec défilement de l'écran et passage dans l'historique comme n'importe quelle ligne.
- Nouveau `ImageStore` (quota 256 Mio, oubli des plus anciennes) pour les images transmises avec `i=`, et `placement::extent` pour calculer l'emprise en cellules (taille naturelle, `c=` / `r=`, réduction à la largeur du terminal).
- Réponses `OK` / erreur (`ENOENT`, `EINVAL`…) selon `i=` et `q=` ; `d=a` / `d=i` retirent les bandes de la grille active ; `Term::set_cell_pixels` règle la taille de cellule (10×20 par défaut).

## 0.1.0-alpha.99 — 2026-10-09 · « Décodage d'images durci »

- `rustty-vt` : les dimensions d'un PNG sont lues dans l'en-tête et refusées (`EFBIG`) au-delà de 8192 px avant tout décodage de pixels ; le décodeur est de plus plafonné à 256 Mio d'allocation.
- Les images vides (largeur ou hauteur nulle, tous formats) sont rejetées (`EINVAL`).
- Documentation des invariants de `ImageData::new` (`debug_assert` sur la longueur RGBA) et de `Line::images` / `push_image` (`cols >= 1`).

## 0.1.0-alpha.98 — 2026-10-09 · « Images décodées et bandes rattachées aux lignes »

- `rustty-vt` : `graphics::decode` convertit PNG, RGB et RGBA bruts en RGBA (côté max 8192 px, longueur brute vérifiée), `ImageData` reçoit un identifiant unique au processus.
- `Placement` et `ImageStrip` (partagés par `Arc`, égalité par pointeur) décrivent une image posée et sa tranche par ligne ; `Line` porte `images()` / `push_image()`.
- Les bandes sont retirées par `reset`, `erase_range`, `insert_blank` et `delete` quand elles coupent la plage, et par `resize` si elles débordent ; écrire du texte par-dessus les conserve.
- Dépendance `image` (PNG seulement) ajoutée à `rustty-vt` ; rien n'est encore branché dans `Term`.

## 0.1.0-alpha.97 — 2026-10-09 · « Action graphique par défaut alignée sur kitty »

- `rustty-vt` : sans `a=`, une commande graphique est désormais `Transmit` (comme kitty) et non `TransmitAndPut`.
- Test ajouté : un premier morceau rejeté avec `m=1` voit ses suites écartées sans erreur parasite, et la transmission suivante aboutit.

## 0.1.0-alpha.96 — 2026-10-09 · « Images : analyse du protocole graphique »

- `rustty-vt` : nouveau module public `graphics` (commandes APC `G…` du protocole graphique kitty), sans effet tant que le hook `Term::apc` n'est pas branché.
- `parse` extrait les clés `a f i s v c r m q d t o` et la charge base64 ; clés inconnues ignorées, nombres invalides ramenés à `None`, `a`/`f` inconnus signalés par `invalid`.
- `Chunks` réassemble les morceaux (`m=1` … `m=0`), décode le base64 en une fois, refuse `t≠d` et `o=z`, et plafonne la charge à 64 Mio (`EFBIG`) ; les morceaux restants après une erreur sont écartés.
- `GraphicsError::code()` : `EINVAL`, `EFBIG`, `ENODATA`, `ENOENT`.

## 0.1.0-alpha.95 — 2026-10-09 · « Images : filtre APC »

- `rustty-vt` : nouveau module `apc` qui extrait les chaînes APC (`ESC _ … ESC \`) avant `vte`, qui les avalait silencieusement.
- Reconstitution d'une APC répartie sur plusieurs lectures ; un ESC isolé en fin de lecture est retenu puis rendu à `vte`, sans perte.
- Une APC dépassant 96 Mio est abandonnée jusqu'à son terminateur ; un ESC suivi d'autre chose que `\` dans une APC l'interrompt.
- Hook interne `Term::apc`, vide pour l'instant (rempli par les tâches suivantes du plan 8).

## 0.1.0-alpha.94 — 2026-10-09 · « Plan 8 : images et corrections différées »

- Plan `docs/superpowers/plans/2026-10-09-images.md` : protocole graphique kitty dans `rustty-vt` (filtre APC, bandes d'image rattachées aux lignes), dessin des images dans `rustty-render`, commande `rustty img <chemin|url>`.
- Le même plan reprend les mineurs différés des revues des plans 4 à 7 : menu contextuel, zoom, molette, souris, reflow, démarrage, lanceur, presse-papiers OSC 52, titres d'onglets.

## 0.1.0-alpha.93 — 2026-10-09 · « Menu : clics dans le menu »

- Un clic sur un séparateur, sur une entrée grisée ou dans la marge du menu le laisse ouvert au lieu de le fermer ; seul un clic hors du menu le ferme.

## 0.1.0-alpha.92 — 2026-10-09 · « Menu contextuel »

- `rustty` : menu au clic droit dans les panneaux — copier (grisé sans sélection), coller, diviser verticalement ou horizontalement, agrandir/réduire, renommer l'onglet, nouvel onglet, fermer le panneau cliqué (confirmation comprise), raccourcis affichés ; `shift`+clic droit dans les applications qui captent la souris ; fermeture par Échap, clic ailleurs, perte du focus ou redimensionnement ; dessiné au-dessus de tout.
- README et `docs/e2e.md`.

## 0.1.0-alpha.91 — 2026-10-09 · « Menu contextuel, modèle »

- `rustty` : modèle pur du menu contextuel — entrées (copier, coller, diviser, agrandir/réduire, renommer l'onglet, nouvel onglet, fermer le panneau) reliées aux actions de la configuration, raccourci le plus court affiché, placement borné à la fenêtre, test de clic (séparateurs et entrées grisées inactifs) et dessin.

## 0.1.0-alpha.90 — 2026-10-09 · « Plan 7 : menu contextuel »

- Plan d'implémentation du menu au clic droit dans les panneaux (copier, coller, diviser, agrandir, renommer l'onglet, nouvel onglet, fermer le panneau). Les images (`img`) passent au plan 8.

## 0.1.0-alpha.89 — 2026-10-09 · « Corrections de revue »

- Reflow quinze fois plus rapide (10 000 lignes d'historique : 5,6 ms au lieu de 82 ms) : les lignes qui tiennent déjà sont seulement mises à la largeur, plus de saccade en glissant une barre.
- Plus aucune ligne perdue quand le curseur est en haut d'un panneau qu'on rétrécit.
- Redimensionner pendant vim ou less réorganise aussi l'écran principal et l'historique au lieu de les tronquer.
- Le bandeau (confirmation de fermeture, erreur de configuration) reste au-dessus d'un panneau zoomé.

## 0.1.0-alpha.88 — 2026-10-09 · « Documentation des panneaux »

- README : section « Installer » (`cargo install --path crates/rustty`, `cargo install --git …`, `--install-desktop`, `--init-config`), raccourcis à jour, zoom par panneau, glisser des barres, reflow ; `docs/e2e.md` complété.
- `rustty` : la molette et le focus quittent `input.rs` pour `wheel_input.rs`.

## 0.1.0-alpha.87 — 2026-10-09 · « Glisser les barres »

- `rustty` : les barres de split se glissent à la souris (saisie élargie à 4 px, même sans bordure), le curseur devient ↔ ou ↕ au survol, le contenu des panneaux se réorganise (reflow).

## 0.1.0-alpha.86 — 2026-10-09 · « Zoom par panneau »

- `rustty` : zoom de police par panneau — `ctrl+molette`, `ctrl+plus`, `ctrl+minus`, `ctrl+0` ne changent que le panneau sous la souris (sinon le panneau actif) ; les autres panneaux, la barre d'onglets et les bandeaux gardent la taille de la config. Un renderer par taille en usage, créé à la demande et libéré quand plus aucun panneau ne s'en sert.

## 0.1.0-alpha.85 — 2026-10-09 · « Rendu par-dessus »

- `rustty-render` : `Renderer::render_onto` dessine par-dessus une image existante (sans effacer), pour composer une même image avec plusieurs tailles de police.

## 0.1.0-alpha.84 — 2026-10-09 · « Reflow »

- `rustty-vt` : reflow au changement de largeur — l'écran principal et l'historique sont redécoupés selon les retours à la ligne automatiques (une ligne longue se replie quand le panneau rétrécit et se déplie quand il s'élargit), curseur reporté sur le même caractère, caractères larges jamais coupés, blancs de fin retirés ; l'écran alternatif n'est pas touché.

## 0.1.0-alpha.83 — 2026-10-09 · « Glisser une barre de split »

- `rustty-layout` : `TabLayout::drag_divider` place la barre d'une division sous la souris (ratio borné à 0,1–0,9, refusé quand un panneau est zoomé) et `split_axis` donne son orientation.

## 0.1.0-alpha.82 — 2026-10-09 · « ctrl+tab et --init-config »

- `ctrl+tab` / `ctrl+shift+tab` changent d'onglet (déliables par `"none"`).
- `rustty --init-config` écrit la configuration d'exemple complète et commentée (raccourcis compris) au chemin par défaut ou à celui de `--config`, sans jamais écraser un fichier existant.

## 0.1.0-alpha.81 — 2026-10-09 · « Plan 6 : panneaux »

- Plan d'implémentation de la deuxième salve de demandes : `rustty --init-config` pour éditer les raccourcis, `ctrl+tab` / `ctrl+shift+tab`, glisser les barres de split à la souris, reflow des lignes au redimensionnement, zoom de police propre au panneau survolé (barre d'onglets inchangée), installation documentée par `cargo install`. Les images (`img`) passent au plan 7.

## 0.1.0-alpha.80 — 2026-10-09 · « Corrections de revue »

- Renommage : l'onglet s'élargit à chaque frappe (le nom n'est plus tronqué) ; l'édition s'arrête sur toute autre action, tout clic dans la barre ou dans un panneau (fini le `git push` tapé dans le nom de l'onglet, ou le mauvais onglet renommé après en avoir ouvert un).
- Couleurs d'onglet : un fond choisi sans couleur de texte reçoit un texte lisible ; les couleurs de texte claire et sombre suivent la luminance, y compris sur un thème clair.
- Zoom : un seul cran par événement de molette (moins de rechargements de polices) ; `ctrl+shift+0` revient aussi à la taille configurée (`0` demande shift en AZERTY).

## 0.1.0-alpha.79 — 2026-10-09 · « Icône partout »

- Icône `res/rustty.svg` partout : PNG 16–512, `.ico` et `.icns` générés par `scripts/icons.sh` dans `assets/icons/` ; icône de fenêtre, identifiant d'application `rustty` (Wayland `app_id`, X11 `WM_CLASS`), icône embarquée dans l'exécutable Windows (`build.rs` + `winresource`), `rustty --install-desktop` installe le lanceur et les icônes hicolor sous `~/.local/share`.
- Journal : par défaut les dépendances ne parlent qu'en cas d'erreur (plus d'avertissement d'`arboard` au lancement sous GNOME).
- README et `docs/e2e.md` : nouvelles options, renommage, zoom, icône.

## 0.1.0-alpha.78 — 2026-10-09 · « Barres, couleurs et zoom »

- `rustty` : barres de split dessinées dans l'interstice (épaisseur et couleur de `[splits]`, ou couleur tirée au sort par division ; sans barre, panneaux bord à bord), couleurs aléatoires des onglets avec texte lisible, zoom de police par `ctrl+molette`, `ctrl+plus`, `ctrl+minus`, `ctrl+0` (4 à 72 points, retour à la taille configurée), rendu extrait dans `render.rs`.

## 0.1.0-alpha.77 — 2026-10-09 · « Renommage des onglets »

- `rustty` : renommage des onglets au double clic ou par `ctrl+shift+alt+t` — édition dans la barre (Entrée valide, Échap annule, retour arrière), caractères de contrôle ignorés, 64 caractères au plus, nom vide = titre du shell ; le nom remplace `{title}` dans le gabarit.

## 0.1.0-alpha.76 — 2026-10-09 · « Accents et titres d'onglet »

- `rustty` : tirage reproductible de couleurs d'accent parmi les 12 couleurs vives de la palette (jamais deux fois de suite la même) ; chaque onglet et chaque division reçoivent la leur à la création ; un onglet peut porter un nom personnalisé.

## 0.1.0-alpha.75 — 2026-10-09 · « Style de la barre d'onglets »

- `rustty-render` : la barre d'onglets suit `[tabs]` — marges horizontale et verticale, espacement entre onglets, couleurs configurées — et accepte une couleur d'accent par onglet (assombrie quand l'onglet est inactif) avec un texte automatiquement contrasté (`Rgba::luminance`, `readable_on`). Rendu par défaut inchangé.

## 0.1.0-alpha.74 — 2026-10-09 · « Barres de split »

- `rustty-layout` : chaque division porte un identifiant stable (`SplitId`, celui de la fenêtre qu'elle a créée) et `TabLayout::dividers` rend le rectangle de chaque barre entre panneaux.

## 0.1.0-alpha.73 — 2026-10-09 · « Options de personnalisation »

- `rustty-config` : section `[splits]` (barre avec ou sans, épaisseur, couleur, couleur aléatoire par division), marges horizontale et verticale et espacement des onglets, section `[tabs.colors]` (couleurs d'onglet et tirage aléatoire), actions `rename_tab`, `increase_font_size`, `decrease_font_size`, `reset_font_size` avec leurs raccourcis par défaut, noms de touches `plus`, `minus`, `equal`, bornes validées.

## 0.1.0-alpha.72 — 2026-10-09 · « Plan 5 : personnalisation »

- Plan d'implémentation des demandes du 2026-10-09 : barre de split (avec ou sans, largeur, couleur fixe ou aléatoire par split), marges et espacement des onglets, renommage des onglets (double clic, `ctrl+shift+alt+t`), couleurs d'onglet fixes ou aléatoires avec texte contrasté, zoom de police (ctrl+molette, ctrl++, ctrl+-, ctrl+0), icône `res/rustty.svg` partout (fenêtre, barre des tâches, exécutable Windows, menus Linux), journal sans avertissements de dépendances. L'affichage d'images (`img`) fera l'objet du plan 6.

## 0.1.0-alpha.71 — 2026-10-09 · « Corrections de revue »

- `rustty-pty` : la libération d'un `Pty` ne bloque plus l'appelant — le shell est tué et moissonné dans un thread détaché (SIGHUP, grâce, SIGKILL) ; le thread lecteur capture une panique du traitement et la signale par `PtyEvent::Failed` au lieu de disparaître.
- `rustty` : un panneau dont le lecteur a paniqué est fermé avec un bandeau d'erreur, l'application continue ; la molette au pixel (pavé tactile) accumule les fractions de ligne au lieu de les perdre.

## 0.1.0-alpha.70 — 2026-10-09 · « Harnais pty et écrivain cédé »

- `rustty-pty` : le harnais de tests répond aux demandes de ConPTY par l'écrivain disponible (y compris celui cédé par `take_writer`), ce qui faisait échouer un test sur Windows.

## 0.1.0-alpha.69 — 2026-10-09 · « Le terminal s'ouvre »

- `rustty` : fenêtre winit + surface wgpu, un shell par panneau, clavier (raccourcis puis encodage), souris (barre d'onglets avec ✕ au survol, sélection avec copie, collage, molette, rapports aux applications), onglets et splits, opacité, bandeaux, rechargement de la configuration à chaud, fin de shell avec `--hold`, icône et titre avec la version.
- CI : bibliothèques Wayland et xkbcommon sur Linux pour compiler winit.
- Docs : `docs/e2e.md` (parcours manuel) et README (lancement, raccourcis).

## 0.1.0-alpha.68 — 2026-10-09 · « Surface et frame »

- `rustty` : surface wgpu (format non-sRGB ou vue linéaire, composition alpha prémultipliée pour l'opacité, acquisition robuste), et construction pure de la `Frame` (panneaux, onglets, sélection, fond avec opacité).

## 0.1.0-alpha.67 — 2026-10-09 · « Rechargement de la config »

- `rustty` : surveillance du fichier de configuration (`notify`, répertoire parent) et rechargement sûr — fichier absent = défauts, fichier invalide = ancienne config conservée avec le message d'erreur.

## 0.1.0-alpha.66 — 2026-10-09 · « TermWindow »

- `rustty` : réveils de l'interface (`UserEvent`, `Wake`) et `TermWindow` — `Term` sous verrou, thread lecteur qui alimente l'émulation, renvoie ses réponses au PTY et réveille la boucle, thread écrivain, resize, défilement, sondage de fin de shell.

## 0.1.0-alpha.65 — 2026-10-09 · « Modèle d'interface »

- `rustty` : modèle pur des décisions d'interface — actions de la configuration, clics sur la barre d'onglets, confirmation de fermeture quand des programmes tournent, fermeture de la fenêtre, fin de shell avec ou sans `--hold`, opacité bornée — rendues sous forme d'effets testés.

## 0.1.0-alpha.64 — 2026-10-09 · « Titres et bandeau »

- `rustty` : gabarit des titres d'onglet, titre de fenêtre avec la version, bandeau d'une ligne (erreur, information, confirmation) rendu en chrome.

## 0.1.0-alpha.63 — 2026-10-09 · « Onglets et panneaux »

- `rustty` : `Tab` (arbre de panneaux ↔ terminaux) et `Workspace` (onglets, actif, nouveau/fermer/suivant/précédent/numéro, split, focus directionnel, redimensionnement, rotation, zoom), purs et testés.

## 0.1.0-alpha.62 — 2026-10-09 · « Géométrie de fenêtre »

- `rustty` : barre d'onglets (haut, bas, cachée, seuil `min_tabs`), zone de contenu, rectangles des panneaux avec interstice, cellule et panneau sous la souris.

## 0.1.0-alpha.61 — 2026-10-09 · « Sélection »

- `rustty` : sélection en cellules (ancre, tête, ordre de lecture) et extraction du texte depuis un instantané (caractères larges, lignes repliées, fins de ligne nettoyées).

## 0.1.0-alpha.60 — 2026-10-09 · « Rapports souris »

- `rustty` : encodage des événements souris pour les applications (modes X10, normal, bouton, tous mouvements ; legacy et SGR ; modificateurs ; molette).

## 0.1.0-alpha.59 — 2026-10-09 · « Encodage clavier »

- `rustty` : encodage xterm des touches (contrôle, alt, touches nommées, F1–F12, paramètre de modificateurs, mode curseur application) et collage assaini avec encadrement (mode 2004).

## 0.1.0-alpha.58 — 2026-10-09 · « Touches → raccourcis »

- `rustty` : traduction des touches winit en combinaisons de la configuration (minuscules, touches nommées, F1–F12, modificateurs).

## 0.1.0-alpha.57 — 2026-10-09 · « Écrivain PTY »

- `rustty-pty` : `PtyWriter` + `spawn_writer` (thread écrivain alimenté par un canal, l'interface ne bloque jamais), `Pty::take_writer`, `Pty::has_running_children` (groupe de premier plan sur Unix).

## 0.1.0-alpha.56 — 2026-10-09 · « Crate rustty »

- Nouvelle crate binaire `rustty` : ligne de commande (`--config`, `--hold`, `--version`, `--help`), journalisation `tracing` pilotée par `RUSTTY_LOG`, chargement de la configuration avec repli sur les défauts.

## 0.1.0-alpha.55 — 2026-10-08 · « Plan 4 : le binaire »

- Plan d'implémentation du binaire `rustty` (14 tâches) : ligne de commande, thread écrivain PTY, clavier (raccourcis, encodage xterm, collage), souris (rapports, sélection), géométrie, onglets et espace de travail, titres et bandeau, modèle d'interface à effets, `TermWindow`, rechargement de la configuration, surface wgpu et frame, boucle winit.

## 0.1.0-alpha.54 — 2026-10-08 · « Lignes sous ConPTY »

- `rustty-pty` : le test des 2 000 lignes extrait les numéros indépendamment des séquences de contrôle que ConPTY intercale sur la même ligne ; vérifie toujours l'ordre strict, sans perte ni doublon.

## 0.1.0-alpha.53 — 2026-10-08 · « ConPTY répondu »

- `rustty-pty` : le harnais de tests répond aux demandes de position du curseur (`ESC[6n`) comme un vrai terminal ; ConPTY n'émet rien tant qu'il n'a pas cette réponse, ce qui bloquait tous les tests Windows. Sur macOS, la sortie pompée en continu a levé le blocage.

## 0.1.0-alpha.52 — 2026-10-08 · « Tests pty bornés »

- `rustty-pty` : les tests d'intégration pompent la sortie dans un thread et bornent chaque attente (`try_wait` sondé) ; un blocage devient un échec qui montre la sortie lue, au lieu de suspendre la CI (macOS et Windows restaient coincés sur une lecture bloquante).

## 0.1.0-alpha.51 — 2026-10-08 · « CI bornée »

- CI : délai de 30 minutes par job et tests exécutés un à la fois, pour qu'un test bloqué (GPU ou pty) échoue en nommant le coupable au lieu d'occuper le runner six heures.

## 0.1.0-alpha.50 — 2026-10-08 · « Corrections de revue »

- `rustty-render` : le repli par couverture teste les polices du système sans copier leurs octets, à chasse fixe d'abord, et mémorise les échecs par bloc Unicode (plus de gel par icône inconnue).
- `rustty-render` : après une reconstruction d'atlas en cours d'image, les instances sont recréées en une seconde passe (plus de glyphes corrompus pendant une image).
- `rustty-pty` : un shell tué par un signal est rapporté `Signaled` (et non `Exited(1)`).
- `rustty-pty` : contrat de fin de flux documenté pour Windows (ConPTY ne ferme le tube qu'à la libération du `Pty` : sonder `try_wait`).
- Écart assumé avec la spec pour l'alpha : atlas RGBA8 unique, cache par `(char, variante)`, reconstruction totale au débordement (R8, clé par glyphe et éviction LRU viendront plus tard).

## 0.1.0-alpha.49 — 2026-10-08 · « Barre d'onglets »

- `rustty-render` : disposition de la barre d'onglets, boutons de fermeture ✕ sur pastille arrondie avec couleurs de survol, bouton « + », test de clic ; image de référence.
- README : statut des cinq crates et régénération des images de référence.

## 0.1.0-alpha.48 — 2026-10-08 · « Renderer »

- `rustty-render` : `Renderer` assemble fonds, glyphes (cache + atlas reconstruit au débordement), décorations et chrome en une passe ; opacité du fond par l'alpha ; images de référence comparées hors écran sur les trois OS.

## 0.1.0-alpha.47 — 2026-10-08 · « Instances de grille »

- `rustty-render` : `Frame`/`PaneFrame`/`Chrome` décrivent une image à dessiner ; `pane_instances` transforme un `Snapshot` en fonds fusionnés, demandes de glyphes, soulignements, barrés et curseur (bloc plein ou creux, barre, souligné), avec rognage à la zone visible.

## 0.1.0-alpha.46 — 2026-10-08 · « Pipeline de glyphes »

- `rustty-render` : texture d'atlas RGBA8 avec envoi par région, quads texturés monochromes (couleur d'instance) ou couleur (emoji), échantillonnage au pixel près.

## 0.1.0-alpha.45 — 2026-10-08 · « Pipeline de quads »

- `rustty-render` : rectangles colorés instanciés en coordonnées pixels avec mélange alpha, vérifiés par relecture.

## 0.1.0-alpha.44 — 2026-10-08 · « Contexte GPU hors écran »

- `rustty-render` : `GpuContext::headless` (matériel ou rendu logiciel), cible hors écran `Offscreen` avec relecture des pixels, passe d'effacement. CI Linux équipée de lavapipe pour exécuter les tests GPU.

## 0.1.0-alpha.43 — 2026-10-08 · « Atlas de glyphes »

- `rustty-render` : `AtlasPacker` place les bitmaps par étagères avec une marge d'un pixel ; coordonnées de texture normalisées.

## 0.1.0-alpha.42 — 2026-10-08 · « Glyphes procéduraux »

- `rustty-render` : lignes de boîte, blocs, trames et symboles powerline dessinés par le renderer, nets quelle que soit la police.

## 0.1.0-alpha.41 — 2026-10-08 · « Rastérisation »

- `rustty-render` : `Rasterizer` (swash) produit des bitmaps RGBA8 pour contours, contours couleur et emojis bitmap ; `FontSet::glyph` cherche le glyphe dans les variantes, puis dans les polices du système qui couvrent le caractère, puis dans la police embarquée.

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
