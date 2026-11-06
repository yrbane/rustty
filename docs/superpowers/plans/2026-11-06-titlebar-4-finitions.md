# Barre de titre — jalon 4 « Finitions » : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clore la barre de titre maison : une capture de fenêtre complète produite par le renderer, les finitions de comportement différées par les revues des jalons 2 et 3, la documentation finale, et les vitrines publiques (profil GitHub et site) à jour avec une capture fraîche.

**Architecture:** La « capture » est une image de référence de plus (`tests/golden/window.png`), composée hors écran par les mêmes fonctions de chrome que la fenêtre (barre de titre, barre d'onglets, bandeaux de panneau, texte) : reproductible en CI, jamais prise à l'écran. Un script Python (`scripts/showcase.py`, PIL) compose la carte vitrine 1200×630 à partir du logo et de cette image. Les finitions sont de petites fonctions pures testées. Les vitrines sont deux dépôts voisins (`~/Dev/yrbane`, `~/Dev/yrbane.github.io`).

**Tech Stack:** Rust 2024, wgpu hors écran (lavapipe en CI), Python 3 + Pillow, WebP.

**Spec:** `docs/superpowers/specs/2026-11-01-titlebar-design.md` (jalon 4 ; sections « Tests », « Anti-objectifs ») ; règles de publication de `~/.claude/CLAUDE.md` (étape 7, vitrines).

## Global Constraints

- Règles du dépôt : code en anglais, commentaires et commits en français, TDD, aucune mention d'assistant dans les commits, `git add` ciblé.
- Chaque commit : bump alpha dans `Cargo.toml` racine + `cargo update -w` + entrée `## 0.1.0-alpha.N — 2026-11-06 · « titre »` en tête de `CHANGELOG.md`. Plan : alpha.165, puis une version par tâche.
- Avant chaque commit : `cargo fmt --all`, `cargo clippy --workspace --all-targets` (0 avertissement), `cargo test --workspace`.
- Anti-objectif : les images de référence existantes restent vertes (`title_bar.png`, `tab_bar.png`, `pane_header.png`…) ; la nouvelle image est ajoutée, aucune n'est régénérée.
- Aucune capture d'écran du bureau (impossible sur GNOME Wayland ici) : toute image vient du renderer hors écran ou de PIL.
- Ne jamais tuer un processus nommé `rustty`.
- Vitrines : « fun assumé » dans le ton existant des cartes, design du site inchangé, capture en WebP dans `yrbane.github.io/shots/`, réutilisée par le README de profil.

## Review Focus

1. Un texte de capture avec des caractères hors police embarquée (accents, « — ») doit rester lisible dans `window.png` : la police DejaVu embarquée couvre le latin étendu ; le test de capture utilise « déploiement — rustty » (tâche 1).
2. Chemin d'éditeur entre guillemets contenant des espaces (`"C:\Program Files\Code\code.exe" --wait`) : lancé en un seul programme et un argument (test tâche 3 `quoted_programs_keep_their_spaces`).
3. `$VISUAL=vim` (éditeur de terminal) : un éditeur de terminal lancé détaché sans console ne fait rien ; l'ouvreur du bureau prend le relais (test tâche 3 `terminal_editors_yield_to_the_desktop_opener`).
4. Double clic sur le titre, ou renommage d'onglet, pendant la confirmation de fermeture : la confirmation garde le clavier, rien ne s'ouvre (test tâche 3 `no_rename_while_a_close_is_pending`).
5. Erreur de syntaxe TOML au moment de « Par défaut » : le bandeau tient sur une ligne lisible (test tâche 3 `the_parse_error_banner_is_one_line`).

---

### Task 1: Capture de fenêtre complète hors écran (`window.png`)

**Files:**
- Modify: `crates/rustty-render/tests/offscreen.rs` (nouveau test `window_mockup_matches_golden`)
- Create: `crates/rustty-render/tests/golden/window.png` (généré par le test, vérifié à l'œil)
- Modify: `README.md` (la capture sous le logo : `<img src="crates/rustty-render/tests/golden/window.png" …>`), `CHANGELOG.md`, `Cargo.toml`

**Interfaces:**
- Consumes: `TitleBarStyle::from_config`, `layout_title_bar`, `title_bar_chrome`, `TitleBarState { title, hover, maximized, background }` ; `TabBarStyle::from_config`, `layout_tab_bar`, `tab_bar_chrome`, `tab_bar_height`, `TabSpec { title, active, accent }` ; `pane_header_chrome(&[PaneHeaderSpec], &TabBarStyle, metrics)` ; `Frame`, `PaneFrame`, `Chrome { quads, shapes, texts, icons }` ; `support::{renderer, assert_matches_golden, PADDING}`.
- Produces: `tests/golden/window.png`, largeur `64 * m.width`, hauteur `title_bar_height + tab_bar_height + 8 * m.height + 2 * PADDING`.

- [ ] **Step 1: Écrire le test (RED : l'image de référence n'existe pas)**

```rust
#[test]
fn window_mockup_matches_golden() {
    // Barre de titre (titre « déploiement — rustty 0.1 », survol du bouton
    // couleur, couleur de session None), barre d'onglets « 1: déploiement »
    // (actif, accent bleu Rgba::new(0.20, 0.45, 0.80, 1.0)) et « 2: logs »,
    // deux panneaux côte à côte sous leurs bandeaux (focalisé « 1: zsh »
    // avec ✕ visible, « 2: tail -f » non focalisé), texte :
    //   gauche : "$ cargo test --workspace\x1b[32m ok\x1b[0m\n$ " (curseur visible)
    //   droite : "\x1b[33mwarn\x1b[0m rien à signaler\n"
    // Le chrome des trois composants est concaténé champ par champ dans un
    // seul `Chrome` (quads, shapes, texts, icons), comme le fait render.rs.
    assert_matches_golden("window", &px, w, h);
    // Garde-fous indépendants de l'image : le bouton couleur survolé a un fond
    // plus clair que la barre, le séparateur de barre est à y = style.height.
}
```

Les assertions de pixel : `pixel(&px, w, color_button.x + 2, color_button.y + 2)` plus clair (somme RGB) que `pixel(&px, w, w / 2, 2)` ; `pixel(&px, w, 0, style.height) == [0, 0, 0, 255]`.

- [ ] **Step 2: Lancer le test**

Run: `cargo test -p rustty-render --test offscreen window_mockup 2>&1 | tail -5`
Expected: l'image de référence manquante est créée (le support la crée quand elle manque) et le test passe ; sinon FAIL sur une assertion de pixel à corriger dans la composition (pas dans le test).

- [ ] **Step 3: Vérifier l'image à l'œil**

Agrandir ×2 avec PIL dans le scratchpad et lire le PNG : six boutons Material, titre centré, deux onglets dont un bleu, deux bandeaux, texte vert/jaune lisible. Si la composition est fausse (chevauchement, texte coupé), corriger le test puis `UPDATE_GOLDEN=1 cargo test -p rustty-render --test offscreen window_mockup` — uniquement cette image.

- [ ] **Step 4: Suite complète, README, commit** (alpha.166, « Capture de fenêtre complète produite par le renderer »)

Run: `cargo test --workspace 2>&1 | grep -E 'test result|FAILED' | sort | uniq -c`
Expected: tout `ok`, images existantes inchangées (`git status` ne montre aucune autre image).

```bash
git add crates/rustty-render/tests/offscreen.rs crates/rustty-render/tests/golden/window.png README.md Cargo.toml Cargo.lock CHANGELOG.md
```

---

### Task 2: Script de composition de la carte vitrine

**Files:**
- Create: `scripts/showcase.py` (exécutable ; `python3 -I`, PIL seulement)
- Modify: `README.md` (section « Développement » : une ligne sur le script), `CHANGELOG.md`, `Cargo.toml`

**Interfaces:**
- Consumes: `assets/logo.png`, `crates/rustty-render/tests/golden/window.png`, police `/usr/share/fonts/TTF/DejaVuSansMono.ttf` repliée sur `crates/rustty-render/assets/DejaVuSansMono.ttf`.
- Produces: `scripts/showcase.py OUT.webp` écrit une image 1200×630 (fond `#1e1e2e` comme la carte actuelle, logo à gauche, titre « rustty », tagline sur trois lignes, `window.png` agrandie ×2 au plus proche sans dépasser la colonne droite, légende « Fenêtre composée hors écran par le renderer, comparée pixel à pixel sur Linux, macOS et Windows »), qualité WebP 90.

- [ ] **Step 1: Test (RED)** — un test shell dans le script lui-même n'a pas de sens ; le test est `scripts/showcase_test.py` (unittest, `python3 -I -m unittest scripts/showcase_test.py`) :

```python
def test_card_has_the_expected_size_and_includes_the_window_capture(self):
    out = tmp / "card.webp"; showcase.compose(ROOT, out)
    im = Image.open(out); self.assertEqual(im.size, (1200, 630))
    # la capture est posée en entier : son pixel de séparateur (noir) se retrouve dans la carte
    self.assertTrue(showcase.contains_capture(ROOT, im))
```

`compose(root: Path, out: Path) -> None` et `contains_capture(root: Path, card: Image) -> bool` (vrai si la rangée du séparateur de la capture, mise à l'échelle, est noire dans la carte à la position où `compose` l'a posée ; `compose` rend cette position par `capture_box(root) -> (x, y, w, h)`).

- [ ] **Step 2: Lancer le test**

Run: `python3 -I -m unittest scripts/showcase_test.py 2>&1 | tail -3`
Expected: `ModuleNotFoundError: showcase` ou `AttributeError`.

- [ ] **Step 3: Écrire `scripts/showcase.py`** (fonctions `capture_box`, `compose`, `contains_capture`, `main(argv)`), puis relancer : `OK`.

- [ ] **Step 4: Produire la carte dans le scratchpad et la regarder** (`python3 -I scripts/showcase.py /tmp/…/rustty.webp`, puis Read du PNG converti). Ajuster typographie/marges si besoin, relancer le test.

- [ ] **Step 5: Commit** (alpha.167, « Script de composition de la carte vitrine »)

---

### Task 3: Finitions de comportement différées par les revues

**Files:**
- Modify: `crates/rustty/src/external.rs` (`split_command` avec guillemets, `is_terminal_editor`), `crates/rustty/src/external_tests.rs`
- Modify: `crates/rustty/src/model/close.rs` (garde `pending_close` sur `start_rename`, `start_pane_rename`, `start_window_rename`), `crates/rustty/src/model_tests.rs`
- Modify: `crates/rustty/src/config_write.rs` (`WriteError::Parse` affiché sur une ligne), `crates/rustty/src/config_write_tests.rs` (+ test du nettoyage du temporaire)
- Modify: `docs/e2e.md` (trois cases : chemin entre guillemets, `$VISUAL=vim`, double clic pendant confirmation), `CHANGELOG.md`, `Cargo.toml`

**Interfaces:**
- Produces: `external::split_command(value: &str) -> Option<Vec<String>>` (reste privé ; `"…"` groupe des espaces, les guillemets sont retirés) ; `external::is_terminal_editor(program: &str) -> bool` (nom de fichier, sans chemin ni `.exe`, parmi `vi vim nvim nano micro hx helix emacs ed`) ; `editor_command` ignore une valeur dont le programme est un éditeur de terminal et passe à la suivante.

- [ ] **Step 1: Tests (RED)**

```rust
// external_tests.rs
#[test]
fn quoted_programs_keep_their_spaces() {
    assert_eq!(
        editor_command("\"C:\\Program Files\\Code\\code.exe\" --wait", None, None),
        vec!["C:\\Program Files\\Code\\code.exe", "--wait"]
    );
    assert_eq!(editor_command("\"\"", None, None), opener(), "guillemets vides = blanc");
}

#[test]
fn terminal_editors_yield_to_the_desktop_opener() {
    assert!(is_terminal_editor("/usr/bin/vim"));
    assert!(is_terminal_editor("nvim.exe"));
    assert!(!is_terminal_editor("code"));
    assert_eq!(editor_command("", Some("vim"), Some("nano -w")), opener());
    assert_eq!(editor_command("", Some("vim"), Some("gedit")), vec!["gedit"]);
}

// model_tests.rs
#[test]
fn no_rename_while_a_close_is_pending() {
    let (mut m, _) = Model::new(0.9, false);
    m.pending_close = Some(CloseRequest::Window);
    assert!(m.start_window_rename().is_empty());
    assert!(m.start_rename(0).is_empty());
    assert!(m.renaming.is_none());
}

// config_write_tests.rs
#[test]
fn the_parse_error_banner_is_one_line() {
    let err = with_titlebar_background("[titlebar\nheight = 28\n", Rgba8::new(0, 0, 0, 0xff)).unwrap_err();
    let text = err.to_string();
    assert!(text.starts_with("configuration invalide : "), "{text}");
    assert!(!text.contains('\n'), "{text}");
}

#[test]
fn a_rename_failure_removes_the_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("rustty.toml");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("occupant"), "").unwrap(); // dossier non vide : rename échoue
    assert!(write_atomically(&target, "x").is_err());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "pas de .tmp-<pid>");
}
```

- [ ] **Step 2: Lancer**

Run: `cargo test -p rustty quoted_programs terminal_editors no_rename_while the_parse_error a_rename_failure 2>&1 | tail -8` (un filtre à la fois si cargo refuse plusieurs motifs)
Expected: erreurs de compilation (`is_terminal_editor`) puis échecs d'assertion.

- [ ] **Step 3: Implémenter** — `split_command` : découpe maison (espace hors guillemets sépare, `"` ouvre/ferme, pas d'échappement) ; `is_terminal_editor` ; `editor_command` : `.filter(|cmd| !is_terminal_editor(&cmd[0]))` après `split_command` ; `WriteError::Parse` : `#[error("configuration invalide : {}", first_line(.0))]` avec `fn first_line(e: &TomlError) -> String` (texte jusqu'au premier `\n`, `message()` de toml_edit si disponible) ; garde `if self.pending_close.is_some() { return Vec::new(); }` dans les trois `start_*`.

- [ ] **Step 4: Suite complète, E2E, commit** (alpha.168, « Finitions : éditeurs de terminal, chemins entre guillemets, renommage et bandeau »)

---

### Task 4: Documentation finale de la barre de titre

**Files:**
- Modify: `README.md` : statut alpha (ajouter « barre de titre maison : icônes Material, renommage de la fenêtre, nuancier ») ; puce « Barre de titre » relue d'un bloc ; paragraphe `[titlebar]` relu ; mention de `scripts/showcase.py` et `scripts/material-icons.sh` dans « Développement » ; nombre de tests (« 830+ »).
- Modify: `docs/e2e.md` : la section « Barre de titre (plan 11) » devient « Barre de titre » avec sous-titres « Affichage », « Boutons », « Titre et couleur », « Déplacement et redimensionnement », « Configuration à chaud » (retirer les mentions de jalons).
- Modify: `docs/superpowers/specs/2026-11-01-titlebar-design.md` : sous « Jalons », une ligne « État : les quatre jalons sont livrés (alpha.145 → alpha.169) ».
- Modify: `CHANGELOG.md`, `Cargo.toml`.

- [ ] **Step 1: Rédiger, vérifier `cargo test --workspace` (test de documentation), commit** (alpha.169, « Barre de titre : documentation finale »)

---

### Task 5: Vitrines publiques (après fusion et release de la branche)

Préalable : PR fusionnée dans `main`, CI verte, tag `v0.1.0-alpha.169`, pré-release publiée (procédure habituelle).

**Files (autres dépôts) :**
- `~/Dev/yrbane.github.io/shots/rustty.webp` : régénérée par `python3 -I scripts/showcase.py ~/Dev/yrbane.github.io/shots/rustty.webp` depuis la racine de rustty.
- `~/Dev/yrbane.github.io/index.html` : `fc-desc` de la carte rustty : ajouter, dans le ton existant, « Et une <strong>barre de titre maison</strong> 🪟 : icônes Material, double clic pour renommer la fenêtre, nuancier pour colorer la barre et « Par défaut » qui l'écrit dans le TOML sans toucher au reste, réglages qui ouvre la config dans l'éditeur, dossier qui ouvre le répertoire courant » ; `alt` de l'image mis à jour (fenêtre composée hors écran : barre de titre, onglets, bandeaux).
- `~/Dev/yrbane/README.md` : même ajout en anglais dans le paragraphe rustty (« a **home-made title bar** 🪟 : Material icons, double-click to rename the window, a color swatch for the bar whose “Default” entry writes it to the TOML without touching the rest, a settings button that opens the config in your editor and a folder button that opens the cwd »), « 830+ tests », `alt` de l'image.

- [ ] **Step 1: Générer la capture, relire la carte (Read du WebP converti en PNG).**
- [ ] **Step 2: Modifier les deux fichiers ; vérifier que `index.html` reste valide (`python3 -c "import html.parser…"` ou simple relecture du diff).**
- [ ] **Step 3: Commits et push** : `git -C ~/Dev/yrbane.github.io commit -am "rustty alpha.169 : barre de titre maison, capture composée hors écran"` ; idem profil. Vérifier ensuite `https://yrbane.github.io/shots/rustty.webp` (ctx_execute, HEAD 200) après le déploiement Pages.

---

## Après les tâches 1 à 4

Revue finale par un sous-agent (modèle le plus capable) sur `git merge-base main HEAD..HEAD` ; correctifs Critical/Important en une passe TDD ; `superpowers:finishing-a-development-branch` : PR, CI verte, fusion, tag, pré-release (notes = section CHANGELOG), `cargo install --path crates/rustty --force` ; puis tâche 5.
