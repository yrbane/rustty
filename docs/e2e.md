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

## Robustesse
- [ ] Fenêtre réduite à quelques pixels puis agrandie : pas de panique.
- [ ] `cargo run -p rustty` : rien sur la sortie d'erreur pendant le parcours ; `RUSTTY_LOG=rustty=debug` pour le détail.
