# Hyperliens — conception

Date : 2026-11-06. Chantier v0.2 n° 1 (spec générale : `2026-10-08-rustty-design.md`, § 8).

## 1. Intention

Un lien qui apparaît dans un terminal doit s'ouvrir d'un geste dans le
navigateur par défaut, dans un nouvel onglet — c'est le navigateur qui
décide de l'onglet, rustty lui confie l'URL par l'ouvreur du bureau
(`xdg-open`, `open`, `rundll32 url.dll,FileProtocolHandler`), comme le font
déjà les boutons réglages et dossier de la barre de titre.

Deux sortes de liens, décidées par l'auteur :

- **Liens déclarés** par les programmes avec OSC 8 (`ls --hyperlink`, `gcc`,
  `gh`, `systemd`, `rg --hyperlink-format`) : le texte affiché et la cible
  peuvent différer.
- **URL en clair** dans le texte (`https://…`, `file://…`, `mailto:…`),
  détectées par rustty comme le fait kitty, parce qu'elles sont partout
  (sorties de `cargo`, logs, pages de manuel) alors que OSC 8 reste rare.

Au survol, le lien se souligne, le curseur devient une main, et après un
court délai une infobulle montre la cible — pour qu'un texte « voir la doc »
ne cache pas où il mène. Le geste d'ouverture (clic simple ou ctrl+clic)
se choisit dans le TOML.

## 2. Objectifs, non-objectifs, anti-objectifs

Objectifs :

- OSC 8 mémorisé par cellule dans `rustty-vt`, survivant aux SGR, au
  défilement, à l'historique, au redimensionnement (reflow) et à la
  sélection.
- Détection des URL en clair au survol, bornée à la ligne logique sous la
  souris (lignes `wrapped` jointes), jamais à tout l'écran.
- Survol : soulignement du lien entier, curseur main, infobulle de cible.
- Ouverture par l'ouvreur du bureau, geste configurable, schémas d'URL
  autorisés configurables.
- Rien ne change pour un terminal qui n'émet ni OSC 8 ni URL.

Non-objectifs (plus tard ou jamais) :

- Navigation des liens au clavier (les « hints » de kitty).
- Chemins de fichiers sans schéma (`src/main.rs:12`) cliquables.
- Un programme d'ouverture par schéma (`[links] handlers`).
- Liens dans les titres d'onglet, les bandeaux ou la barre de titre.
- Aperçu de la page, favicon, historique des liens ouverts.

Anti-objectifs (échec même si « ça marche ») :

- Ouvrir un lien sans geste explicite, ou un lien que l'utilisateur ne
  voyait pas (texte trompeur sans infobulle, schéma exotique).
- Avaler un clic qui devait poser une sélection ou aller à l'application
  qui capture la souris.
- Ralentir le rendu ou le survol en scannant tout l'écran à chaque
  mouvement.
- Faire grossir sensiblement la mémoire des cellules ou laisser la table
  des liens croître sans borne.

## 3. Contraintes

- Rust stable, crates `rustty-vt` (pur, sans E/S), `rustty-render`
  (dessin), `rustty-config` (TOML), `rustty` (fenêtre). Code en anglais,
  commentaires en français, TDD.
- `Cell` est `Copy` et compacte (16 octets aujourd'hui) ; le lien ajoute au
  plus 4 octets.
- Pas de nouvelle dépendance pour la détection d'URL (pas de `regex`,
  pas de `url`) : un balayage à la main suffit et reste lisible.
- Le survol, le clic et l'infobulle réutilisent la mécanique existante :
  `HoverTarget`, `Selection`, `SpellPopup` (mise en page et dessin),
  `spawn_detached`, bandeau d'erreur, `CursorIcon`.
- Mêmes règles de publication que le reste du projet (version alpha par
  commit, CHANGELOG, CI trois OS, pré-release, vitrines si l'UI change).

## 4. Décisions de l'auteur

- Ouverture via l'ouvreur du bureau ; le navigateur par défaut ouvre un
  nouvel onglet.
- Liens OSC 8 **et** URL en clair.
- Geste d'ouverture choisi dans le TOML.
- Infobulle montrant la cible au survol, après un court délai, pour les
  deux sortes de liens, en réutilisant l'infobulle du correcteur.

## 5. Choix de conception (mes appels, à contester)

### 5.1 Mémorisation dans `rustty-vt`

- `Cell { c, style, link: Option<LinkId> }`, `LinkId = NonZeroU16` : les
  2 octets tiennent dans le bourrage actuel de `Cell` (4 + 10 arrondis à
  16), qui reste à 16 octets — un test `cell_stays_compact` le garantit.
  `Cell::new(c, style)` garde sa signature avec `link: None` ;
  `Cell::with_link(c, style, link)` en plus.
- `Term.current_link: Option<LinkId>` indépendant du style SGR : `SGR 0`
  ne le touche pas ; seul `OSC 8 ; ; ST` (URI vide) le remet à `None`.
  `RIS` et le reset complet le remettent à `None` et vident la table.
- `Term.links: Arc<LinkTable>` partagée avec les `Snapshot` sans copie par
  image (copie à l'écriture via `Arc::make_mut`). `LinkTable` : `Vec<Link>`
  indexée par `LinkId`, `Link { uri: String, id_param: Option<String> }`,
  déduplication par la paire `(uri, id_param)` : la même paire rend le même
  `LinkId`. Écran principal et écran alternatif partagent la table.
- Borne : 65 535 liens par terminal (capacité de `NonZeroU16`). Table
  pleine : compactage — balayage de l'historique et des deux grilles pour
  recenser les `LinkId` encore référencés, renumérotation des cellules,
  table reconstruite (coût d'un balayage complet, payé seulement en
  atteignant la borne ; aussi à `ED 3`, qui vide l'historique). Si la
  table reste pleine après compactage, un nouvel OSC 8 est ignoré (texte
  imprimé sans lien), un lien déjà connu reste utilisable.
- Analyse de `OSC 8 ; params ; URI` : seul le paramètre `id=` est lu
  (les autres, séparés par `:`, sont ignorés). URI refusée si elle dépasse
  2 083 octets, contient un caractère de contrôle (`< 0x20`, `0x7f`) ou
  n'est pas de l'UTF-8 valide : la séquence est alors traitée comme une
  fermeture (`current_link = None`), jamais comme un lien tronqué.
- Les cellules effacées (`ED`, `EL`, `ECH`, insertion de blancs,
  redimensionnement) reçoivent le gabarit sans lien. Le reflow recopie les
  cellules, donc les liens suivent le texte.
- `Snapshot.links: Arc<LinkTable>` et `Snapshot::link_uri(id) -> Option<&str>`.

### 5.2 Détection des URL en clair (crate `rustty`, module pur `links`)

- Déclenchée au survol d'une cellule sans `link` OSC 8 (OSC 8 a priorité),
  sur la **ligne logique** : la ligne de la souris jointe aux lignes
  précédentes et suivantes tant que `wrapped` est vrai, **historique
  compris** même hors de la vue : nouvelle API pure
  `Term::logical_line(visible_row) -> LogicalLine { first_row: isize, lines:
  Vec<Line> }` (`first_row` relatif à la vue, négatif dans l'historique),
  lue par `TermWindow` sous le verrou du terminal. Une URL coupée par le
  bord de la vue est ainsi lue entière ; sa plage est tronquée à la vue
  pour le soulignement, jamais pour l'URI ouverte. Borne : 64 lignes
  jointes de part et d'autre, au-delà la ligne logique est tenue pour
  finie.
- Une URL commence par un schéma de `[links] schemes` (comparaison sans
  casse) suivi de `://`, ou de `:` pour `mailto`, précédé d'un début de
  ligne ou d'un caractère qui n'est ni lettre, ni chiffre, ni `/`.
  Elle s'étend sur les caractères qui ne sont ni blancs, ni contrôle, ni
  `<`, `>`, `"`, `'`, `` ` ``, `{`, `}`, ni espace de largeur nulle.
- Fin de l'URL : les `.`, `,`, `;`, `:`, `!`, `?` finaux sont retirés ;
  un `)`, `]` ou `}` final n'est gardé que si l'URL contient l'ouvrant
  correspondant non refermé (`https://fr.wikipedia.org/wiki/Rust_(langage)`
  reste entier, `(voir https://exemple.org)` s'arrête avant `)`).
- Le résultat est une plage de cellules `(row_start, col_start, row_end,
  col_end)` plus l'URI ; le survol ne retient que la plage qui contient la
  cellule sous la souris. Pas de cache : une ligne logique se balaie en
  microsecondes, et le survol ne rebalaie que quand la cellule change.
- `[links] detect_urls = false` désactive cette détection ; OSC 8 reste.

### 5.3 Survol

- `HoverTarget` inchangé ; un nouvel état `LinkHover { term, run, uri,
  since: Instant }` dans `OsWindow` (`link_input.rs`, pendant de
  `spell_input.rs`), mis à jour par `cursor_moved` quand la cellule
  change, effacé quand la souris quitte le lien, le panneau ou la fenêtre.
- `run` : pour OSC 8, les cellules contiguës portant le même `LinkId` à
  partir de la cellule survolée, en traversant les lignes `wrapped` ; pour
  une URL en clair, la plage détectée. Pas « toutes les occurrences à
  l'écran ».
- Rendu : `PaneFrame` reçoit `link_hover: Option<CellRange>` ; `grid.rs`
  dessine sur ces cellules un soulignement de la couleur de leur premier
  plan, même épaisseur que `SGR 4` (une cellule déjà soulignée ne change
  pas d'aspect — accepté). Curseur `CursorIcon::Pointer` sur le lien,
  rétabli en quittant (`update_cursor_icon` le sait).
- Un lien sous la souris n'empêche pas le correcteur : les deux survols
  coexistent, l'infobulle du correcteur a priorité si elle est ouverte.

### 5.4 Infobulle

- Après `400 ms` de survol immobile sur le même lien (réveil par
  `WaitUntil`, comme l'échéance de survol d'onglet), une infobulle
  s'affiche sous la ligne du lien, ancrée à sa première cellule, avec le
  texte de la cible tronqué à 60 cellules (`…` au milieu pour garder le
  domaine et la fin). En mode `ctrl`, suffixe ` · ctrl+clic`.
- Même mise en page et même dessin que `SpellPopup` sans rangée de
  remplacement : extraire de `spell/popup.rs` un `Tooltip { anchor,
  message }` partagé, le correcteur gardant ses rangées par-dessus.
- L'infobulle ne capte pas la souris : si la souris y entre, elle a quitté
  le lien et l'infobulle se ferme. Elle se ferme aussi au clic, à la
  molette, à la frappe et à la perte de focus.
- Un même lien survolé à nouveau rouvre l'infobulle après le même délai.

### 5.5 Clic

- `[links] click = "ctrl"` (défaut) ou `"plain"`.
- Ouverture à la **relâche** du bouton gauche quand : l'appui et la
  relâche sont sur le même lien (même `run`), la sélection née de l'appui
  est vide (pas de glisser), et la condition de modificateur tient — en
  mode `ctrl`, ctrl enfoncé à l'appui ; en mode `plain`, aucun
  modificateur requis.
- En mode `ctrl`, l'appui avec ctrl sur un lien ne démarre pas de sélection
  (le clic ne clignote pas). En mode `plain`, l'appui démarre la sélection
  comme d'habitude ; un glisser sélectionne, un clic immobile ouvre.
- Application qui capture la souris (`MouseMode != None`) : le clic lui est
  envoyé, sauf si shift est enfoncé (règle existante) ; alors les règles
  ci-dessus s'appliquent avec shift en plus.
- Machine d'états pure `LinkClick` (pendant de `TitleDrag`) testée sans
  fenêtre : `press(run, mods) → release(run, selection_empty, mods) →
  Option<Open>`.
- Ouverture : `Effect::OpenLink(String)` ; l'effet vérifie le schéma contre
  `[links] schemes`, puis `spawn_detached(opener(), uri)` (`spawn_detached`
  accepte un `&OsStr`/`&str` en cible, pas seulement un `Path`). Échec :
  bandeau « ouvrir le lien : … » comme pour l'éditeur. Un lien OSC 8 dont
  le schéma n'est pas autorisé n'est ni souligné, ni ouvert, ni annoncé :
  texte ordinaire.
- Double clic sur un lien : le premier clic ouvre ; une seconde relâche sur
  le même lien dans le délai de double clic (`DOUBLE_CLICK`, déjà utilisé
  par les onglets) n'ouvre pas une seconde fois. `LinkClick` retient le
  dernier lien ouvert et l'instant.
- Application qui capture la souris : le survol suit la même règle que le
  clic — soulignement, main et infobulle seulement tant que shift est
  enfoncé, réévalués à chaque changement de modificateur (pas seulement de
  cellule) ; le suffixe de l'infobulle devient ` · shift+clic` ou
  ` · shift+ctrl+clic`. Sans shift, rien ne promet un clic qui partirait à
  l'application.

### 5.6 Configuration

```toml
[links]
# Geste d'ouverture : "ctrl" (ctrl+clic) ou "plain" (clic simple).
click = "ctrl"
# Souligner et ouvrir aussi les URL en clair du texte, pas seulement les
# liens OSC 8 déclarés par les programmes.
detect_urls = true
# Schémas d'URL reconnus et ouverts ; tout autre lien reste du texte.
schemes = ["http", "https", "file", "mailto", "ftp", "ssh", "git"]
```

- Validation : `click` ∈ {ctrl, plain} ; `schemes` non vide, chaque schéma
  en lettres ASCII minuscules, chiffres, `+`, `-`, `.`, commençant par une
  lettre, sans doublon. Erreurs nommées comme les autres clés.
- Rechargement à chaud : pris en compte au survol suivant.
- Modèle `rustty.example.toml` et README (section Configuration + souris)
  documentent la section ; `docs/e2e.md` reçoit une section « Liens ».

### 5.7 Sécurité

- Liste blanche de schémas pour l'ouverture ; pas de `javascript:`, pas de
  schémas d'applications (`ms-msdt:`) sans choix explicite dans le TOML.
- La cible exacte est visible dans l'infobulle avant tout clic.
- URI OSC 8 bornée et sans caractère de contrôle ; aucun échappement
  shell : l'URI est un argument unique de l'ouvreur (pas de `cmd /C`).
- `file://` est ouvert par l'ouvreur du bureau (dossier ou fichier) ;
  l'URI est passée telle quelle, sans résolution par rustty.

## 6. Tests

- `rustty-vt` : OSC 8 avec et sans `id`, URI vide ferme, `SGR 0` ne ferme
  pas, même `(uri, id)` → même `LinkId`, URI trop longue ou avec contrôle
  → fermeture, borne 65 535, `RIS` vide la table, effacements sans lien,
  `resize`/reflow conservent les liens, `Snapshot::link_uri`.
- `rustty` `links` (pur) : détection sur des lignes de test (ponctuation
  finale, parenthèses, guillemets, `mailto:`, schéma inconnu, casse,
  ligne `wrapped` jointe, `detect_urls = false`), plage OSC 8 contiguë à
  travers une ligne `wrapped`, priorité OSC 8 sur l'URL en clair.
- `LinkClick` : les huit combinaisons (mode × modificateur × glisser),
  relâche sur un autre lien, capture souris avec et sans shift, double
  clic qui n'ouvre qu'une fois, survol réévalué au changement de
  modificateur sous capture.
- `Term::logical_line` : ligne dans l'historique, à cheval sur la vue, borne
  de 64 lignes ; compactage de la table pleine et à `ED 3`.
- Infobulle : délai respecté (`since + 400 ms`), texte tronqué au milieu,
  suffixe en mode `ctrl`, fermeture en quittant.
- `rustty-config` : analyse, validation (`click` invalide, schéma avec
  majuscule ou vide, doublon), clés documentées dans le modèle.
- `rustty-render` : image de référence `link_hover` (grille avec une URL
  soulignée au survol et l'infobulle sous elle), comparée en CI.
- `docs/e2e.md` : `ls --hyperlink=always`, `printf '\e]8;;https://…\e\\texte\e]8;;\e\\'`, URL en clair en fin de phrase avec point, URL coupée par un retour à la ligne automatique, ctrl+clic et clic simple selon le TOML, `vim` (souris capturée) avec et sans shift, infobulle, schéma non autorisé.

## 7. Jalons

1. **Liens déclarés** : OSC 8 dans `rustty-vt`, survol (soulignement,
   curseur), clic configurable, `Effect::OpenLink`, section `[links]`
   (`click`, `schemes`), infobulle. Version publiable : `ls --hyperlink`
   fonctionne.
2. **URL en clair** : détection sur la ligne logique, `detect_urls`,
   image de référence `link_hover`, documentation finale et vitrines.

## 8. Laissé au bâtisseur

- Forme exacte de `LinkTable` (`Vec` + `HashMap<(String, Option<String>),
  LinkId>` ou équivalent), de `CellRange` et de `LogicalLine`.
- Découpage de `Tooltip` hors de `SpellPopup` sans changer le rendu du
  correcteur (image de référence existante inchangée).
- Où vit le délai de 400 ms (constante, pas de clé TOML).
- Ordre des vérifications dans `cursor_moved` entre lien, correcteur et
  bandeaux, tant que l'infobulle du correcteur garde la priorité.
