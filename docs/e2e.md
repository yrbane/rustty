# Parcours de vérification manuelle (E2E)

À dérouler sur la machine de développement après chaque changement du binaire,
et sur chaque OS avant une release. Lancer `cargo run -p rustty`.

## Fenêtre et shell
- [ ] La fenêtre s'ouvre avec le titre « rustty <version> », l'icône du crabe, le fond à l'opacité de la config.
- [ ] Un prompt du shell apparaît ; taper `ls -la` puis Entrée affiche le listing ; `exit` ferme la fenêtre.
- [ ] `cargo run -p rustty -- --hold`, puis `exit` : le panneau reste avec le bandeau « [processus terminé : code 0] » ; une touche le ferme.
- [ ] Redimensionner la fenêtre : le shell voit la nouvelle taille (`tput cols`, `tput lines`), `htop` se réorganise.

## Clavier
- [ ] Lettres, accents, majuscules, `ctrl+c` interrompt `sleep 10`, `ctrl+d` ferme le shell.
- [ ] Flèches dans `bash` (historique), `Home`/`End`, `ctrl+left` saute un mot ; `shift+tab` dans `fzf` ou `vim` (`:map`) ; F1–F12 dans `htop` (F10 quitte).
- [ ] `vim` : flèches en mode insertion, `Échap`, `:q`.

## Onglets
- [ ] `ctrl+shift+t` ouvre un onglet (titre « 2: shell » puis le titre du shell) ; la barre apparaît selon `min_tabs`.
- [ ] Survol du bouton ✕ : il passe en couleur de survol ; clic : l'onglet se ferme ; clic du milieu sur un titre : idem ; clic sur « + » : nouvel onglet.
- [ ] `ctrl+shift+q` ferme l'onglet actif ; avec `sleep 100` en cours et `confirm_close_with_running_children = true`, le bandeau de confirmation apparaît ; Échap annule, Entrée ferme.
- [ ] Dernier onglet fermé : la fenêtre se ferme.

## Renommage, zoom, icône
- [ ] Double clic sur un onglet : édition du nom (curseur ▌), Entrée valide, Échap annule, nom vide = titre du shell ; `ctrl+shift+alt+t` fait de même sur l'onglet actif.
- [ ] `ctrl+molette`, `ctrl+plus`, `ctrl+minus` changent la taille de police, `ctrl+0` revient à celle de la config.
- [ ] Après `--install-desktop`, la fenêtre a l'icône du crabe dans la barre des tâches et rustty apparaît dans les menus.

## Panneaux (plan 6)
- [ ] Glisser une barre de split à la souris : curseur ↔ ou ↕ au survol, les panneaux suivent, la barre ne dépasse pas 10 % / 90 %.
- [ ] Une longue ligne (`seq -s ' ' 1 200`) se replie quand le panneau rétrécit et se déplie quand il s'élargit ; le prompt reste au bon endroit.
- [ ] Deux panneaux : `ctrl+molette` au-dessus de l'un ne change que lui ; la barre d'onglets ne change pas ; `ctrl+0` le remet à la taille de la config.
- [ ] `ctrl+tab` / `ctrl+shift+tab` passent d'un onglet à l'autre.
- [ ] `rustty --init-config` écrit `~/.config/rustty/rustty.toml` (refuse s'il existe) ; changer un raccourci dans `[keys]` agit sans redémarrer.
- [ ] `cargo install --path crates/rustty` puis `rustty` depuis le `PATH`.

## Menu contextuel
- [ ] Clic droit dans un panneau : le menu s'ouvre au point du clic, avec les raccourcis à droite ; près d'un bord il reste entièrement visible.
- [ ] « Copier » est grisé sans sélection ; avec une sélection, il copie.
- [ ] « Fermer le panneau » ferme le panneau cliqué (pas le panneau actif) ; avec `sleep 100` en cours, la confirmation s'affiche.
- [ ] « Diviser », « Agrandir le panneau », « Renommer l'onglet », « Nouvel onglet » agissent sur le panneau cliqué.
- [ ] Échap, un clic ailleurs, un redimensionnement ou la perte du focus ferment le menu.
- [ ] Dans `vim` avec `:set mouse=a`, le clic droit va à vim ; `shift`+clic droit ouvre le menu.

## Splits
- [ ] `ctrl+shift+e` divise verticalement (nouveau panneau à droite, focalisé), `ctrl+shift+o` horizontalement (en dessous).
- [ ] `shift+flèches` déplacent le focus ; `ctrl+flèches` redimensionnent ; les raccourcis de rotation et de zoom de la config fonctionnent.
- [ ] Un clic dans un panneau le focalise ; le curseur du panneau non focalisé est creux.
- [ ] `exit` dans un panneau ne ferme que ce panneau.

## Souris
- [ ] Glisser-sélectionner copie dans le presse-papiers (coller ailleurs) ; la surbrillance suit la souris ; une touche efface la sélection.
- [ ] Clic du milieu colle ; `ctrl+shift+v` colle ; dans `cat`, un collage multi-lignes arrive avec des `\r`.
- [ ] Molette : défile l'historique (`seq 1 500` puis molette) ; dans `less` ou `vim`, la molette déplace le contenu (écran alternatif).
- [ ] Dans `vim` avec `:set mouse=a`, clic et molette sont transmis à vim ; `shift`+glisser sélectionne quand même.

## Configuration
- [ ] Modifier `opacity` dans le fichier : le fond change sans redémarrer ; mettre `opacity = 7` : bandeau d'erreur, ancienne config conservée ; corriger : le bandeau disparaît.
- [ ] Changer `font.size` : la grille se recalcule.
- [ ] Un raccourci `{ opacity = +0.05 }` ajouté dans `[keys]` change l'opacité par pas.
- [ ] `[splits] width = 6`, puis `random_colors = true` : chaque nouveau split a sa barre colorée ; `border = false` : panneaux bord à bord.
- [ ] `[tabs] padding_horizontal = 3`, `padding_vertical = 6`, `spacing = 4` : la barre s'élargit et s'aère ; `[tabs.colors] random = true` : chaque onglet a sa couleur, texte lisible.

## Images
- [ ] `rustty img assets/logo.png` dans un panneau : l'image s'affiche, le curseur se place dessous.
- [ ] `rustty img https://upload.wikimedia.org/wikipedia/commons/4/47/PNG_transparency_demonstration_1.png` : l'image distante s'affiche (test réseau manuel).
- [ ] `rustty img /inexistant.png` : message d'erreur français sur stderr, rien d'affiché, code de sortie 1 ; `rustty img` seul : erreur d'usage.
- [ ] Afficher plusieurs images puis défiler dans l'historique (`shift+page_up`) : les images défilent avec le texte et reviennent au retour en bas.
- [ ] `ctrl+molette` sur le panneau : l'image suit la taille de la police (autant de cellules, proportions conservées).
- [ ] `clear` : l'image disparaît de l'écran.
- [ ] Rétrécir le panneau (split ou fenêtre) : l'image est rognée à la largeur, sans panique.

## Correctifs des tâches 8 à 11 (plan 8)
Menu contextuel
- [ ] Un bandeau (erreur de config, confirmation) reste visible sous le menu ouvert, qui ne le recouvre pas.
- [ ] Menu ouvert : le curseur de la souris et la surbrillance de l'entrée survolée suivent le pointeur.
- [ ] Fenêtre très étroite : les libellés du menu sont tronqués avec `…`, sans débordement.
- [ ] Menu ouvert, clic droit ailleurs dans la fenêtre : le menu se rouvre à la nouvelle position.
- [ ] Pendant une confirmation de fermeture (programme en cours), le clic droit n'ouvre aucun menu.
- [ ] `--hold` : ouvrir le menu, laisser le shell sortir (`exit` dans un autre panneau n'est pas suffisant, tuer le shell du panneau) ; le menu se ferme à la mort du panneau.

Zoom et sélection
- [ ] `font.size = 80`, puis `ctrl+plus` : la police ne rétrécit pas (plafond respecté).
- [ ] Souris hors de la fenêtre, `ctrl+plus` / `ctrl+minus` : le panneau focalisé est zoomé.
- [ ] Panneau zoomé (`ctrl+plus`), défilement au pavé tactile : le défilement reste fluide et proportionné.
- [ ] Sélectionner du texte, puis redimensionner la fenêtre ou zoomer : la sélection est effacée.
- [ ] Sélectionner du texte, puis défiler à la molette : la sélection est effacée.

Souris et clavier
- [ ] Dans une application qui capte la souris (`htop`, ou vim avec `:set mouse=a`), appuyer dans le panneau, relâcher hors du panneau : le relâchement est transmis, aucun bouton « collé ».
- [ ] Même application : appuyer dans le panneau, alt-tab vers une autre fenêtre, relâcher, revenir et bouger la souris : aucun glisser n'est envoyé (le relâchement part à la perte du focus).
- [ ] `ctrl+1` et `ctrl+9` envoient le chiffre au programme (`cat -v`, puis les touches) ; `alt+1`…`alt+5` changent toujours d'onglet.

Configuration et fichiers
- [ ] `cargo run -p rustty -- --config /nonexistent` : bandeau « /nonexistent introuvable », la config par défaut s'applique.
- [ ] `--init-config` et `--install-desktop` : aucun fichier `*.tmp` ne reste dans `~/.config/rustty` ni `~/.local/share`.
- [ ] `[window] osc52_clipboard = false` : `printf '\e]52;c;%s\a' $(echo -n hi | base64)` ne modifie pas le presse-papiers ; avec `true`, il contient `hi`.

Barre d'onglets
- [ ] Ouvrir beaucoup d'onglets puis réduire la fenêtre : le dernier onglet visible est tronqué avec `…` (au moins 3 cellules de titre), sinon il est omis.

## Bandeaux et glisser-déposer (plan 9)
Prérequis : un onglet à deux panneaux (`ctrl+shift+e`), `[splits] random_colors = true` pour distinguer les couleurs.

Bandeau
- [ ] Onglet à un seul panneau : aucun bandeau. Deux panneaux : chacun a un bandeau d'une rangée en haut, et le terminal perd une rangée (`tput lines`). Fermer l'un des deux : le bandeau disparaît et la rangée est rendue.
- [ ] `ctrl+shift+z` (zoom) : le bandeau disparaît ; second appui : il revient.
- [ ] Le bandeau a la couleur de la division qui contient le panneau (aléatoire avec `random_colors`, sinon `splits.color`, sinon `ansi[8]`) ; texte lisible.
- [ ] Le titre du bandeau est le titre du shell (`printf '\e]0;essai\a'`), tronqué avec `…` quand le bandeau est étroit.
- [ ] Survol du bandeau : un ✕ apparaît à droite et passe en couleur de survol ; clic : le panneau se ferme (confirmation avec `sleep 100` en cours). Sur un bandeau de moins de 6 cellules, le ✕ n'apparaît pas.
- [ ] Clic sur le bandeau : le panneau est focalisé. Double clic : édition du nom (curseur ▌), Entrée valide (le bandeau, et l'onglet si ce panneau est focalisé sans nom d'onglet, affichent le nom), Échap annule, nom vide = titre du shell.
- [ ] Le nom suit le panneau quand il change d'onglet et survit à la disparition du bandeau.
- [ ] Une couleur de division reste stable : ouvrir et fermer des panneaux ne repeint pas les autres bandeaux.

Glisser un panneau
- [ ] Presser sur le bandeau et bouger de moins de 4 px, relâcher : simple clic (focus), rien ne bouge.
- [ ] Bouger de plus de 4 px : le glisser commence ; au-dessus d'un autre panneau, la moitié la plus proche (haut, bas, gauche, droite, séparées par les diagonales) est éclairée ; au lâcher, le panneau s'y installe à 50 % et prend le focus. Le shell garde son historique et ses processus (`sleep 100` continue).
- [ ] Un panneau seul dans son onglet n'a pas de bandeau : rien à tirer.
- [ ] Pendant le glisser, survoler un autre onglet moins de 0,5 s ne l'affiche pas ; au bout de 0,5 s il s'affiche (une seule fois par survol) et on peut lâcher sur l'un de ses panneaux.
- [ ] Lâcher sur un onglet de la barre : le panneau passe à droite de tout cet onglet (division verticale à la racine, 50 %), qui s'affiche.
- [ ] Lâcher sur son propre onglet : le panneau passe à droite de tout l'onglet.
- [ ] Échap pendant le glisser : rien ne change, Échap n'est pas transmis au terminal, et l'onglet de départ est réaffiché si un survol en avait affiché un autre.
- [ ] Lâcher sur le panneau tiré lui-même, ou hors de tout panneau et onglet (y compris hors de la fenêtre) : rien ne change, onglet de départ réaffiché.
- [ ] Pendant le glisser, ouvrir un menu ou perdre le focus (alt-tab) annule le glisser.

Réordonner les onglets
- [ ] Presser un onglet, bouger de moins de 4 px, relâcher : simple clic (activation) ; double clic : renommage.
- [ ] Tirer un onglet au-delà de 4 px : un repère vertical montre la place d'insertion ; au lâcher il prend cette place et l'onglet actif reste le même.
- [ ] Échap, ou lâcher hors de la barre : l'ordre ne change pas.

Menus
- [ ] Clic droit sur un onglet (pas forcément l'actif) : menu « Renommer », « Changer la couleur », « Nouvel onglet », « Fermer l'onglet », « Fermer les autres onglets » ; les actions visent l'onglet cliqué.
- [ ] « Fermer les autres onglets » avec `sleep 100` dans l'un d'eux : confirmation.
- [ ] « Changer la couleur » : le nuancier de 12 pastilles plus « Couleur par défaut » remplace le menu, se pilote au survol, au clic et à Échap ; la couleur choisie s'affiche sur l'onglet même avec `[tabs.colors] random = false` ; « Couleur par défaut » rend la couleur de la configuration.
- [ ] Clic droit sur un bandeau : le panneau est focalisé, puis menu « Diviser horizontalement », « Diviser verticalement », « Agrandir le panneau », « Déplacer dans un nouvel onglet », « Renommer le panneau », « Fermer le panneau ». Les divisions héritent du dossier courant du panneau.
- [ ] « Déplacer dans un nouvel onglet » : un onglet juste après l'onglet courant contient ce seul panneau (sans bandeau), et s'affiche.

Points de revue
- [ ] Panneau zoomé (`ctrl+molette` au-dessus de l'un des deux panneaux) : le bandeau garde sa hauteur de base ; cliquer juste sous le bandeau place le curseur et la sélection en rangée 0 du terminal.
- [ ] Application qui capte la souris (`htop`, ou vim avec `:set mouse=a`) : un clic sur le bandeau ne lui est jamais transmis ; un clic sur la première rangée du contenu est rapporté en rangée 1.
- [ ] Tirer un panneau puis tuer son shell (`kill -9` depuis un autre panneau), ou fermer la cible, pendant le glisser : le glisser s'annule, sans panique.
- [ ] Pendant une confirmation de fermeture ou un renommage en cours, tenter de tirer un panneau ou un onglet : refusé, rien ne bouge.
- [ ] Réduire un panneau à moins de deux rangées de haut : contenu de hauteur 0 sans panique, terminal à 1×1 au minimum.

## Infobulle de correction (plan 10)
Prérequis : `languagetool --http --port 8099` (ou Claude Code avec le mod [ortho](https://github.com/yrbane/claude-ortho) 0.2+, qui le lance), `[spellcheck] enabled = true`.

Survol
- [ ] `printf 'ma \e[4;31mpoulle\e[0m est là\n'` puis souris immobile sur « poulle » : une boîte à bordure apparaît au-dessus du mot, alignée sur sa première colonne, avec le message de LanguageTool puis les suggestions (« poule », « poulie »…, 6 au plus). Rien ne s'affiche sur un mot souligné correct (`\e[4mbonjour\e[0m`) ni sur un mot non souligné.
- [ ] Mot souligné sur la première rangée : la boîte s'ouvre en dessous. Mot près du bord droit : la boîte est décalée pour rester dans la fenêtre.
- [ ] Bouger la souris à l'intérieur du mot : la boîte reste, sans clignoter ni refaire de requête (journal `RUSTTY_LOG=rustty=debug`). Entrer dans la boîte : la suggestion sous la souris passe sur fond de sélection.
- [ ] Quitter le mot et la boîte, appuyer sur une touche, redimensionner la fenêtre, changer la taille de police : la boîte disparaît. `clear` dans le panneau : elle disparaît aussi ; un simple redessin qui garde le mot (`tput cup` ailleurs) la laisse ouverte.
- [ ] Deux mots soulignés : passer de l'un à l'autre ferme la première boîte et ouvre la seconde.

Clic
- [ ] Dans Claude Code avec le mod ortho : taper « ma poulle » dans la saisie, survoler le souligné rouge, cliquer « poule » : le brouillon devient « ma poule », le curseur reste en place, le souligné disparaît après la nouvelle vérification, et rien du jeton n'apparaît.
- [ ] Même mot fautif deux fois dans le brouillon : cliquer la suggestion de la seconde occurrence ne corrige que celle-là.
- [ ] Clic sur le message (pas sur une suggestion) : la boîte reste ouverte, rien n'est envoyé. Clic hors de la boîte : elle se ferme et le clic agit normalement (sélection, menu).
- [ ] Dans un shell sans le mod (`printf` ci-dessus) : le clic colle le jeton tel quel (caractères à usage privé) ; c'est attendu et documenté.

Serveur
- [ ] LanguageTool arrêté : aucune boîte, et au plus une requête par 30 s (journal `debug`) ; relancé : la boîte revient au survol suivant.
- [ ] `[spellcheck] enabled = false` (rechargement à chaud) : plus aucune requête ni boîte. `url = "ftp://x"` : erreur de configuration nommant `spellcheck.url`.

## Barre de titre (plan 11)
Prérequis : `[titlebar] enabled = true` (le défaut) ; la fenêtre n'a plus de décorations système.

Affichage
- [ ] Au lancement : une barre de 28 px logiques en haut, fond sombre à reflet, liseré en bas, titre du shell centré ; la barre d'onglets (quand elle est visible) et les panneaux commencent sous elle, même avec `[tabs] position = "bottom"`.
- [ ] Le titre suit le panneau actif (`printf '\e]0;coucou\a'`) et porte la version ; un titre trop long est tronqué avec `…`.
- [ ] Survoler une pastille : son fond de survol et son icône apparaissent ; la quitter les efface. Rien ne change sur le fond de la barre.
- [ ] HiDPI (si disponible, `WINIT_X11_SCALE_FACTOR=2` ou un écran à 200 %) : la barre, les pastilles et la marge doublent ; changer d'écran en cours de route recalcule le style.

Boutons
- [ ] Réduire : la fenêtre va dans la barre des tâches. Agrandir : la fenêtre remplit l'écran et l'icône devient « restaurer » ; cliquer de nouveau restaure. Agrandir par le bureau (double clic du système, raccourci du WM) met aussi l'icône à jour.
- [ ] Fermer sans programme en cours : la fenêtre se ferme. Avec `sleep 1000` lancé : la bannière de confirmation apparaît ; `Entrée` ferme, `Échap` annule.
- [ ] Appuyer sur une pastille, glisser hors d'elle, relâcher : rien ne se passe. Clic droit ou du milieu sur une pastille : rien.
- [ ] À gauche : trois icônes grises (engrenage, palette, dossier) ; les survoler éclaircit l'icône sur un fond blanc translucide. Le carré d'agrandir reste bien à l'intérieur de sa pastille.
- [ ] Réglages : le fichier de configuration s'ouvre dans `code` (clé `editor` par défaut). Sans fichier (`mv ~/.config/rustty/rustty.toml /tmp/`), il est créé depuis le modèle complet puis ouvert ; un fichier existant n'est jamais réécrit.
- [ ] `editor = ""` : `VISUAL=gedit rustty` ouvre dans gedit ; sans `$VISUAL` ni `$EDITOR`, l'ouvreur du bureau (`xdg-open`) choisit l'application. `editor = "programme-inexistant"` : bandeau d'erreur « ouvrir la configuration : … », l'interface reste vivante.
- [ ] Dossier : l'explorateur s'ouvre sur le dossier courant du panneau actif (`cd /tmp` avant de cliquer) ; `file_manager = "nautilus --new-window"` change de programme. Juste après `ssh` ou avec un shell dont le dossier ne se lit pas : le dossier personnel.
- [ ] `[keys]` `"ctrl+shift+alt+o" = "open_config"` et `"ctrl+shift+alt+d" = "open_cwd"` font la même chose que les deux boutons.
- [ ] Sans dossier `~/.config/rustty/` au lancement : cliquer réglages crée dossier et fichier, puis une modification enregistrée dans l'éditeur est rechargée à chaud sans redémarrer.
- [ ] `icon_size = 24` puis sauvegarde : les icônes d'outils grandissent à chaud (les pastilles gardent une icône aux quatre cinquièmes de leur taille) ; HiDPI : les icônes doublent avec le reste.
- [ ] `[titlebar.buttons.settings] visible = false` : l'icône disparaît et la zone de titre s'élargit vers la gauche.
- [ ] `background = "#f0f0f0"` : le titre et les trois icônes d'outils deviennent sombres ; les pastilles gardent leurs couleurs. Survoler un bouton d'outil assombrit son fond (voile noir), le survol reste visible.
- [ ] Renommer l'onglet actif (menu contextuel) : le titre de la fenêtre reprend ce nom ; le retirer rend le titre du terminal.

Titre et couleur (jalon 3)
- [ ] Double clic sur le titre : la barre montre un tampon pré-rempli du nom de fenêtre (vide la première fois) suivi de `▌` ; taper `deploy` puis Entrée : la barre et la barre des tâches du bureau (`alt+tab`) affichent « deploy — rustty X ». Pendant la saisie, le titre du bureau montre aussi le tampon et le curseur.
- [ ] Échap pendant la saisie : l'ancien nom reste. Vider le nom puis Entrée : retour au titre du shell (ou au nom de l'onglet s'il en a un). Plus de 64 caractères : la saisie s'arrête à 64.
- [ ] `ctrl+shift+alt+w` ouvre la même saisie ; cliquer un panneau ou la barre pendant la saisie l'annule. Un simple clic sur le titre ne fait rien ; deux clics espacés de plus d'un demi-seconde non plus.
- [ ] Un nom de fenêtre prime sur le nom d'onglet et sur le titre du shell (`printf '\e]0;coucou\a'` ne change plus la barre).
- [ ] Bouton couleur : le nuancier s'ouvre juste sous le bouton, douze pastilles nommées, puis « Par défaut » et « Couleur du fichier ». Cliquer « Bleu » : le fond de la barre devient bleu (titre et icônes d'outils restent lisibles), onglets et panneaux ne bougent pas. Échap ou un clic ailleurs ferme le nuancier ; `[keys]` `"ctrl+shift+alt+c" = "pick_titlebar_color"` l'ouvre au clavier.
- [ ] Couleur de session puis modifier `[colors] blue` et sauvegarder : la barre prend le nouveau bleu. Modifier `[titlebar] background` : la barre garde le bleu de session ; « Couleur du fichier » rend la couleur du fichier.
- [ ] « Par défaut » avec une couleur de session : `background = "#rrggbb"` apparaît dans `[titlebar]` du fichier, les commentaires et les autres clés sont intacts (comparer avec `diff`), la barre garde la couleur sans clignoter, et la couleur de session est oubliée (modifier `background` dans le fichier change maintenant la barre). Sans couleur de session, « Par défaut » ne fait rien.
- [ ] « Par défaut » sans fichier (`mv ~/.config/rustty/rustty.toml /tmp/`) : le fichier est créé depuis le modèle complet avec la couleur, puis une modification enregistrée est rechargée à chaud. Avec `[titlebar]` écrit en table en ligne (`titlebar = { enabled = true }`) : la clé s'ajoute dans la table en ligne.
- [ ] Fichier en lecture seule (`chmod 444` sur le dossier `~/.config/rustty`) ou contenant une erreur de syntaxe : bandeau « enregistrer la couleur : … », aucun fichier temporaire ne reste, la couleur de session est conservée.
- [ ] Fermer un panneau ou un onglet pendant que le nuancier de la barre est ouvert : il reste ouvert et fonctionne.
- [ ] Déplacer la fenêtre par la barre, relâcher, puis cliquer un onglet : il s'active ; cliquer dans un panneau et relâcher : la sélection ne colle pas à la souris.
- [ ] Depuis une pastille survolée, sortir la souris d'un coup par le bord de la fenêtre : l'icône de survol s'efface. Idem en changeant de fenêtre (`alt+tab`) la souris sur une pastille.

Déplacement et redimensionnement
- [ ] Glisser la barre (titre ou fond) de plus de 4 px déplace la fenêtre ; un clic simple ne la déplace pas et ne fait rien. Clic droit sur la barre : rien.
- [ ] Un menu d'onglet ouvert ou un renommage en cours se ferme en cliquant la barre.
- [ ] Les huit bords et coins : le curseur change (↔, ↕, diagonales) dans la marge de 6 px et le glisser redimensionne dans la bonne direction ; juste après la marge, le curseur redevient ordinaire.
- [ ] Fenêtre agrandie : aucun curseur de redimensionnement, aucun redimensionnement, le déplacement par la barre reste possible selon le compositeur.
- [ ] `resize_border = 0` : plus aucun bord ne redimensionne.

Configuration à chaud
- [ ] `enabled = false` puis sauvegarde : les décorations du système reviennent, la barre disparaît, onglets et panneaux remontent ; `enabled = true` : l'inverse. `height`, couleurs, `visible` des boutons : pris en compte au rechargement.
- [ ] `[keys]` `"ctrl+shift+m" = "minimize_window"` et `"ctrl+shift+u" = "toggle_maximize"` : les raccourcis agissent comme les pastilles.

## Robustesse
- [ ] Fenêtre réduite à quelques pixels puis agrandie : pas de panique, la barre de titre seule reste visible.
- [ ] `cargo run -p rustty` : rien sur la sortie d'erreur pendant le parcours ; `RUSTTY_LOG=rustty=debug` pour le détail.
