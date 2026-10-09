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
- [ ] `ctrl+1` et `ctrl+9` envoient le chiffre au programme (`cat -v`, puis les touches) ; `alt+1`…`alt+5` changent toujours d'onglet.

Configuration et fichiers
- [ ] `cargo run -p rustty -- --config /nonexistent` : bandeau « /nonexistent introuvable », la config par défaut s'applique.
- [ ] `--init-config` et `--install-desktop` : aucun fichier `*.tmp` ne reste dans `~/.config/rustty` ni `~/.local/share`.
- [ ] `[window] osc52_clipboard = false` : `printf '\e]52;c;%s\a' $(echo -n hi | base64)` ne modifie pas le presse-papiers ; avec `true`, il contient `hi`.

Barre d'onglets
- [ ] Ouvrir beaucoup d'onglets puis réduire la fenêtre : le dernier onglet visible est tronqué avec `…` (au moins 3 cellules de titre), sinon il est omis.

## Robustesse
- [ ] Fenêtre réduite à quelques pixels puis agrandie : pas de panique.
- [ ] `cargo run -p rustty` : rien sur la sortie d'erreur pendant le parcours ; `RUSTTY_LOG=rustty=debug` pour le détail.
