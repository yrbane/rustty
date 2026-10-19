# Bandeaux de panneau, glisser-déposer et menus d'onglet — conception

Date : 2026-10-19 · Plan visé : plan 9 · Version de départ : 0.1.0-alpha.121
(alpha.120 = dossier courant hérité, PR #10).

## Intention

seb veut réorganiser son espace de travail à la souris, « comme sur
Terminator » : attraper un panneau par une barre de titre et le poser
ailleurs, réordonner les onglets en les glissant, et trouver au clic droit sur
un onglet ou un bandeau les actions qui le concernent. Aujourd'hui, la
disposition ne se construit qu'en divisant ; rien ne se déplace.

Réussite : seb compose sa disposition à la souris sans fermer ni relancer de
shell, et chaque panneau d'un onglet divisé se repère d'un coup d'œil par la
couleur et le titre de son bandeau.

## Objectifs

1. **Bandeau de panneau.** Dans un onglet qui a au moins deux panneaux visibles,
   chaque panneau porte en haut un bandeau d'une rangée : le titre du panneau
   à gauche, un bouton ✕ qui apparaît au survol à droite.
2. **Déplacer un panneau** en le tirant par son bandeau, vers un autre panneau
   du même onglet ou d'un autre onglet.
3. **Réordonner les onglets** en les tirant le long de la barre d'onglets.
4. **Menu contextuel d'onglet** (clic droit sur un onglet).
5. **Menu contextuel de bandeau** (clic droit sur un bandeau).

## Non-objectifs

- Pas de bandeau dans un onglet à un seul panneau, ni sur un panneau agrandi
  (zoom) : l'onglet joue déjà ce rôle (décision de seb).
- Pas de détachement d'un panneau ou d'un onglet vers une nouvelle fenêtre du
  système ; lâcher hors de la fenêtre ne fait rien.
- Pas de raccourcis clavier pour déplacer un panneau ou un onglet.
- Pas de sauvegarde de la disposition entre deux lancements.
- Pas de nouvelle section de configuration : le bandeau n'est pas désactivable
  et sa couleur suit les réglages de `[splits]` existants (YAGNI).
- Pas de groupes de diffusion de frappe (autre fonction de Terminator).
- Pas de fusion d'un onglet dans un autre en lâchant un onglet sur un panneau :
  tirer un onglet ne sert qu'à le réordonner (*choix de Claude*).

## Anti-objectifs (échec même si « ça marche »)

- Un déplacement qui tue, relance ou vide un shell : le `TermId`, son PTY, son
  historique et ses images restent intacts ; seul l'arbre de disposition change.
- Un simple clic sur un bandeau ou un onglet interprété comme un glisser.
- Le contenu du terminal décalé ou mal cliqué : la sélection, les clics transmis
  aux applications et la position du curseur texte restent exacts sous le bandeau.
- Une perte de la colonne/rangée réelle : le terminal est redimensionné d'une
  rangée en moins quand le bandeau apparaît, et la reprend quand il disparaît.

## Contraintes

- Règles du projet inchangées : TDD, SOLID, fichiers d'environ 300 lignes au
  plus (`input.rs`, `window_state.rs`, `model.rs`, `rustty-layout/src/node.rs`
  et `rustty-render/src/chrome/mod.rs` sont déjà à la limite : les nouvelles
  responsabilités vont dans de nouveaux modules), code en anglais, commentaires
  et commits en français, une version par commit.
- Les opérations sur l'arbre vivent dans `rustty-layout` (crate pure, testée
  par propriétés) ; l'interface ne manipule jamais `Node` directement.
- Rendu testé par images de référence hors écran, comme la barre d'onglets.
- Linux, macOS et Windows.

## Comportement détaillé (décidé avec seb)

### Bandeau

- Visible seulement quand l'onglet affiche au moins deux panneaux (pas de
  zoom en cours).
- **Couleur** : celle de la division qui contient directement le panneau,
  c'est-à-dire la même règle que la barre de split (`appearance.rs`) appliquée
  au parent immédiat du panneau dans l'arbre : couleur tirée au sort de la
  division si `splits.random_colors`, sinon `splits.color`, sinon `ansi[8]`.
- **Contenu** : le titre du panneau (titre personnalisé s'il existe, sinon le
  titre annoncé par le shell), tronqué avec « … ». Au survol du bandeau, un
  bouton ✕ apparaît à droite, dessiné comme celui des onglets ; un clic dessus
  ferme le panneau (avec la confirmation habituelle si un programme tourne).
- **Clic gauche** sur le bandeau : focalise le panneau. **Double clic** :
  renomme le panneau (même édition en ligne que les onglets).
- **Nom de panneau** (*choix de Claude*) : rangé par terminal (`TermId`), il
  remplace le titre du shell partout où ce titre sert — bandeau, et titre de
  l'onglet quand ce panneau est focalisé, sauf si l'onglet a lui-même un nom.
  Il suit le panneau quand il change d'onglet et survit à la disparition du
  bandeau. Un nom vide rend le titre automatique.

### Glisser un panneau

- On presse sur le bandeau (hors ✕) et on bouge d'au moins 4 px : le glisser
  commence ; en dessous, c'est un clic.
- Au-dessus d'un autre panneau, la moitié la plus proche du curseur est
  éclairée (haut, bas, gauche ou droite : les deux diagonales du panneau
  délimitent les quatre zones, comme Terminator).
- Au lâcher : le panneau quitte sa place (son voisin prend l'espace, comme une
  fermeture) et divise la cible du côté éclairé, à 50 %. Il devient focalisé.
- Survoler un onglet de la barre pendant 0,5 s affiche cet onglet ; on peut
  alors lâcher sur l'un de ses panneaux.
- Lâcher directement sur un onglet de la barre ajoute le panneau à droite de
  tout l'onglet (nouvelle division verticale à la racine, 50 %), et affiche cet
  onglet.
- Lâcher directement sur son propre onglet est permis : le panneau passe à
  droite de tout l'onglet (*choix de Claude*, cohérent avec les autres onglets).
- Lâcher sur le panneau lui-même, hors de tout panneau ou onglet, ou appuyer
  sur Échap : la disposition ne change pas et l'onglet de départ est réaffiché
  si un survol en avait affiché un autre (*choix de Claude*).
- Comme seul un onglet d'au moins deux panneaux a des bandeaux, l'onglet de
  départ garde toujours au moins un panneau.

### Réordonner les onglets

- Presser un onglet et bouger d'au moins 4 px : le glisser commence ; un repère
  vertical montre où l'onglet s'insérera. Au lâcher, l'onglet prend cette place
  et l'onglet actif reste le même onglet.
- Échap ou lâcher hors de la barre : rien ne change.

### Menu d'un onglet (clic droit sur l'onglet)

Renommer · Changer la couleur · Nouvel onglet · Fermer l'onglet · Fermer les
autres onglets.

- Les actions visent l'onglet cliqué, pas forcément l'onglet actif.
- **Changer la couleur** (décision de seb) : ouvre à la place un nuancier — les
  12 couleurs vives de `ACCENT_SLOTS`, chacune avec sa pastille, puis
  « Couleur par défaut ». La couleur choisie est rangée sur l'onglet et
  s'affiche même quand `[tabs.colors] random = false` ; « Couleur par défaut »
  efface ce choix et rend la couleur prévue par la configuration. Le nuancier
  se pilote comme le menu (survol, clic, Échap).
- « Fermer les autres onglets » demande confirmation si un programme tourne
  dans l'un d'eux, comme la fermeture d'un onglet.

### Menu d'un bandeau (clic droit sur le bandeau)

Diviser horizontalement · Diviser verticalement · Agrandir le panneau ·
Déplacer dans un nouvel onglet · Renommer le panneau · Fermer le panneau.

- Les actions visent le panneau du bandeau cliqué (il est focalisé d'abord).
- Les divisions héritent du dossier courant du panneau (alpha.120).
- « Déplacer dans un nouvel onglet » crée un onglet juste après l'onglet
  courant, qui contient ce seul panneau, et l'affiche.

## Ce qui est à construire (repères pour le constructeur)

- `rustty-layout` : détacher une feuille sans la détruire ; insérer une feuille
  existante à côté d'une autre, d'un côté donné (haut/bas/gauche/droite) ;
  insérer à la racine ; connaître la division parente d'une feuille. Les
  invariants existants (proptest) doivent rester vrais.
- `Tab`/`Workspace` : retirer un terme d'un onglet sans le tuer, l'insérer dans
  un autre, créer un onglet autour d'un terme existant, déplacer un onglet,
  cibler un onglet ou un panneau qui n'est pas l'actif. Nettoyer les couleurs
  de division orphelines.
- Géométrie : la rangée du bandeau est retirée du rectangle donné à la grille
  (dimensionnement, clics, sélection, dessin) et un test de visée distingue
  bandeau, ✕ et contenu.
- Menus : la liste des entrées et la cible deviennent des paramètres du menu
  contextuel existant (aujourd'hui figés sur le menu du panneau).
- Une échéance « survol de 0,5 s » branchée sur `about_to_wait`.

## Laissé au constructeur

- Hauteur exacte du bandeau (une rangée à la taille de police de base), marges,
  forme du ✕, couleur du texte (règle de lisibilité automatique des onglets).
- Apparence de la moitié éclairée et du repère d'insertion d'onglet.
- Découpage en tâches et en modules.
