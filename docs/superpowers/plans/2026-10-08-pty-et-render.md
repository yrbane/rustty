# Plan d'implémentation — `rustty-pty` et `rustty-render`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer `rustty-pty` (lancer un shell dans un pseudo-terminal et échanger des octets sur les trois OS) et `rustty-render` (dessiner un `Snapshot` de `rustty-vt` et le chrome de l'interface avec wgpu, testé hors écran par comparaison d'images), prêts à être assemblés par le binaire du plan 4.

**Architecture:** `rustty-pty` enveloppe `portable-pty` derrière une façade `Pty` (lecture bloquante clonable, écriture, resize, état du processus) et un thread lecteur optionnel qui pousse des `PtyEvent` dans un canal. `rustty-render` sépare la résolution des couleurs (`Palette`), les polices (`FontSet` via `fontdb` + métriques de cellule, `Rasterizer` via `swash`, glyphes de boîte et powerline dessinés procéduralement), un atlas de glyphes (packer CPU + texture GPU), deux pipelines wgpu (quads colorés instanciés, quads texturés), une couche pure qui transforme un `Frame` en listes d'instances (`grid.rs`, `chrome.rs`), et le `Renderer` qui orchestre. Le contexte GPU est créé sans fenêtre pour les tests (`GpuContext::headless`) et une cible hors écran relit les pixels. Une police de test libre embarquée (DejaVu Sans Mono) rend les images de référence identiques sur les trois OS.

**Tech Stack:** Rust 1.96 (edition 2024) ; `portable-pty` 0.9 ; `wgpu` 30, `bytemuck`, `pollster` ; `fontdb` 0.24, `swash` 0.2 ; `image` 0.25 (PNG des tests) ; `thiserror`.

**Spec:** `docs/superpowers/specs/2026-10-08-rustty-design.md`, sections 2.1, 2.2, 3.4, 3.5, 5, 6.

## Global Constraints

- Code et identifiants en **anglais** ; commentaires, commits, CHANGELOG en **français**. Aucune mention d'assistant dans les commits.
- Chaque commit bumpe `[workspace.package] version` du `Cargo.toml` racine : la version après le commit du plan est `0.1.0-alpha.35`, la tâche 1 produit `alpha.36`, la tâche N produit `alpha.(35+N)`. Entrée en tête de `CHANGELOG.md` au format `## 0.1.0-alpha.N — 2026-10-08 · « titre »`.
- `cargo fmt --all`, puis `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace` doivent passer avant chaque commit, depuis la racine. `rustfmt` fait autorité sur la mise en forme.
- **Les signatures des crates externes font autorité** : le code de ce plan vise `wgpu` 30, `swash` 0.2.10, `fontdb` 0.24, `portable-pty` 0.9. Si une signature diffère à la compilation, lire la doc de la version résolue (`cargo doc --open` ou docs.rs), adapter au plus près, et consigner l'écart dans le rapport. Ne jamais verrouiller une version plus ancienne pour faire coller le plan.
- `git add` ciblé, jamais `-A`. Les fichiers binaires ajoutés (police de test, images de référence) sont listés explicitement.
- Un fichier = une responsabilité ; aucun fichier au-delà de ~300 lignes hors tests. Clippy strict refuse le code mort : un champ ou une méthode n'apparaît que dans la tâche qui le consomme.
- Les tests GPU **s'exécutent** sur la CI des trois OS : Linux via lavapipe (`mesa-vulkan-drivers`, installé par le workflow), Windows via l'adaptateur de repli WARP (DirectX 12), macOS via Metal. Un test GPU qui ne trouve aucun adaptateur l'écrit sur `stderr` et retourne sans échouer, pour qu'un poste de développement sans GPU reste utilisable ; la CI, elle, en a toujours un.
- Déterminisme du rendu : police de test embarquée, échantillonnage `Nearest`, glyphes posés à des coordonnées entières, format de texture non-sRGB (`Rgba8Unorm`) pour les tests. Les comparaisons d'images tolèrent une différence moyenne ≤ 1,0 par canal et ≤ 0,5 % de pixels différant de plus de 8.
- Convention de rendu : origine en haut à gauche, pixels, `y` vers le bas. Les couleurs `Rgba` sont des `f32` dans [0, 1], non pré-multipliées ; le mélange alpha est classique (`src_alpha`, `one_minus_src_alpha`).

## Review Focus

1. **Shell qui meurt immédiatement** (programme inexistant, `exit 3` au lancement) : `spawn` rend une erreur nommant le programme, ou `try_wait` rend le code de sortie ; aucun blocage, aucun zombie. Tests dans la tâche 2 (`unknown_program_is_an_error`, `exit_code_is_reported`).
2. **Sortie volumineuse** (plusieurs Mo d'un coup) : le thread lecteur découpe en blocs, l'ordre est préservé, rien n'est perdu ni dupliqué. Test dans la tâche 3 (`large_output_is_delivered_in_order`).
3. **Police demandée absente** : repli silencieux sur une police à chasse fixe du système, puis sur la police embarquée ; jamais de panique, jamais de cellule de largeur 0. Tests dans la tâche 5 (`unknown_family_falls_back`, `cell_metrics_are_positive_for_the_system_monospace`).
4. **Caractère hors police** (emoji, CJK, caractère de contrôle, U+FFFD) : rendu par une police de repli si elle existe, sinon une cellule vide, jamais de panique ni d'index hors atlas. Tests dans la tâche 6 (`missing_glyph_is_none_not_panic`) et la tâche 13 (`unknown_characters_render_as_blank`).
5. **Viewport minuscule ou nul** (0×0, 1×1, plus petit qu'une cellule) : `Renderer::render` ne panique pas, ne crée pas de texture de taille 0, et le découpage en cellules rend 0 colonne ou 0 ligne proprement. Tests dans la tâche 12 (`grid_geometry_survives_tiny_rects`) et la tâche 13 (`render_survives_tiny_viewport`).

## Structure des modules

```
crates/rustty-pty/src/
├── lib.rs           # réexports
├── shell.rs         # Shell (programme + arguments, défaut par OS), environnement injecté
├── size.rs          # PtySize (cellules + pixels)
├── error.rs         # PtyError
├── pty.rs           # Pty : spawn, reader, write, resize, try_wait, kill (portable-pty)
└── reader.rs        # spawn_reader : thread lecteur → canal de PtyEvent

crates/rustty-render/src/
├── lib.rs           # réexports
├── color.rs         # Rgba, Palette (vt Color → Rgba, bold_is_bright, inverse, dim, hidden)
├── font/
│   ├── mod.rs       # Variant, FontError, réexports
│   ├── metrics.rs   # CellMetrics (largeur, hauteur, ligne de base, soulignement)
│   ├── loader.rs    # FontSet : fontdb, 4 variantes, repli, police embarquée, glyphe + repli par couverture
│   └── raster.rs    # Rasterizer (swash) → GlyphBitmap RGBA8
├── builtin.rs       # glyphes de boîte, blocs et powerline dessinés procéduralement
├── atlas.rs         # AtlasPacker (étagères, CPU) + AtlasRegion
├── gpu.rs           # GpuContext (headless ou adaptateur fourni), Offscreen (cible + relecture)
├── pipeline/
│   ├── mod.rs
│   ├── quad.rs      # QuadPipeline, QuadInstance, QuadBatch
│   └── glyph.rs     # GlyphPipeline, GlyphInstance, GlyphBatch, AtlasTexture
├── shaders/
│   ├── quad.wgsl
│   └── glyph.wgsl
├── frame.rs         # Frame, PaneFrame, Chrome, Text, HoverTarget (ce que le binaire fournit)
├── grid.rs          # géométrie des cellules + instances (fonds, glyphes demandés, décorations) — pur
├── chrome.rs        # barre d'onglets : disposition, boutons ✕, rectangles de clic — pur
└── renderer.rs      # Renderer : cache de glyphes, atlas GPU, passes de rendu
crates/rustty-render/tests/
├── fonts/DejaVuSansMono.ttf   # police de test (licence Bitstream Vera, libre)
├── fonts/LICENSE-DejaVu
├── golden/*.png               # images de référence
└── offscreen.rs               # tests de rendu hors écran
```

---

### Task 1 : Crate `rustty-pty`, shell, taille et erreurs

**Files:**
- Create: `crates/rustty-pty/Cargo.toml`
- Create: `crates/rustty-pty/src/lib.rs`
- Create: `crates/rustty-pty/src/shell.rs`
- Create: `crates/rustty-pty/src/size.rs`
- Create: `crates/rustty-pty/src/error.rs`
- Modify: `Cargo.toml` (racine : `portable-pty` dans `[workspace.dependencies]`)

**Interfaces:**
- Produces:
  - `pub struct Shell { pub program: String, pub args: Vec<String> }` ; `Shell::new(program, args)` ; `Shell::default_for_platform() -> Shell` (`$SHELL` sinon `/bin/sh` sur Unix ; `powershell.exe` sur Windows) ; `Shell::from_env_or_default(shell_env: Option<&str>) -> Shell` (testable sans toucher à l'environnement).
  - `pub fn default_env() -> Vec<(String, String)>` : `TERM=xterm-256color`, `COLORTERM=truecolor`.
  - `pub struct PtySize { pub cols: u16, pub rows: u16, pub pixel_width: u16, pub pixel_height: u16 }` ; `PtySize::new(cols, rows)` (pixels 0) ; `PtySize::with_pixels(cols, rows, w, h)` ; `cols`/`rows` **jamais 0** : `new` les borne à 1.
  - `pub enum PtyError { Open(String), Spawn { program: String, source: String }, Io(std::io::Error) }` (`thiserror`, messages en français).

- [ ] **Step 1: Écrire les tests**

`crates/rustty-pty/src/shell.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_variable_wins_over_platform_default() {
        let s = Shell::from_env_or_default(Some("/usr/bin/fish"));
        assert_eq!(s.program, "/usr/bin/fish");
        assert!(s.args.is_empty());
    }

    #[test]
    fn empty_env_variable_is_ignored() {
        let s = Shell::from_env_or_default(Some("   "));
        assert_eq!(s, Shell::from_env_or_default(None));
    }

    #[cfg(unix)]
    #[test]
    fn unix_default_is_sh() {
        assert_eq!(Shell::from_env_or_default(None).program, "/bin/sh");
    }

    #[cfg(windows)]
    #[test]
    fn windows_default_is_powershell() {
        assert_eq!(Shell::from_env_or_default(None).program, "powershell.exe");
    }

    #[test]
    fn default_env_declares_a_256_color_truecolor_terminal() {
        let env = default_env();
        assert!(env.contains(&("TERM".to_string(), "xterm-256color".to_string())));
        assert!(env.contains(&("COLORTERM".to_string(), "truecolor".to_string())));
    }
}
```

`crates/rustty-pty/src/size.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_no_pixel_size() {
        let s = PtySize::new(80, 24);
        assert_eq!((s.cols, s.rows, s.pixel_width, s.pixel_height), (80, 24, 0, 0));
    }

    #[test]
    fn zero_cells_are_clamped_to_one() {
        let s = PtySize::new(0, 0);
        assert_eq!((s.cols, s.rows), (1, 1));
        assert_eq!(PtySize::with_pixels(0, 5, 10, 10).cols, 1);
    }
}
```

`crates/rustty-pty/src/error.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_error_names_the_program() {
        let e = PtyError::Spawn { program: "nope".into(), source: "introuvable".into() };
        assert!(e.to_string().contains("nope"));
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cd /home/seb/Dev/rustty && cargo test -p rustty-pty`
Expected: `did not match any packages`.

- [ ] **Step 3: Implémenter**

`Cargo.toml` racine, `[workspace.dependencies]`, ajouter :

```toml
portable-pty = "0.9"
wgpu = "30"
bytemuck = { version = "1", features = ["derive"] }
pollster = "1"
fontdb = "0.24"
swash = "0.2"
image = { version = "0.25", default-features = false, features = ["png"] }
```

`crates/rustty-pty/Cargo.toml` :

```toml
[package]
name = "rustty-pty"
description = "Lancement d'un shell dans un pseudo-terminal et échange d'octets, sur les trois OS"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
portable-pty.workspace = true
thiserror.workspace = true

[lints]
workspace = true
```

`crates/rustty-pty/src/lib.rs` :

```rust
//! Pseudo-terminal : lance le shell, lit ce qu'il écrit, lui transmet les
//! frappes, le redimensionne. Le thread lecteur est fourni ; le `Term` qui
//! interprète les octets vit ailleurs (`rustty-vt`).

pub mod error;
pub mod shell;
pub mod size;

pub use error::PtyError;
pub use shell::{Shell, default_env};
pub use size::PtySize;
```

`crates/rustty-pty/src/shell.rs` :

```rust
//! Le programme à lancer dans le pseudo-terminal et l'environnement qu'on lui donne.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shell {
    pub program: String,
    pub args: Vec<String>,
}

impl Shell {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self { program: program.into(), args }
    }

    /// `$SHELL` s'il est défini et non vide, sinon le shell par défaut de l'OS.
    pub fn default_for_platform() -> Self {
        let from_env = std::env::var("SHELL").ok();
        Self::from_env_or_default(from_env.as_deref())
    }

    /// Même logique que `default_for_platform`, mais testable.
    pub fn from_env_or_default(shell_env: Option<&str>) -> Self {
        match shell_env.map(str::trim).filter(|s| !s.is_empty()) {
            Some(program) => Self::new(program, Vec::new()),
            None => Self::new(platform_default(), Vec::new()),
        }
    }
}

#[cfg(unix)]
fn platform_default() -> &'static str {
    "/bin/sh"
}

#[cfg(windows)]
fn platform_default() -> &'static str {
    "powershell.exe"
}

/// Variables ajoutées à l'environnement hérité : ce que les programmes
/// doivent savoir du terminal qui les héberge.
pub fn default_env() -> Vec<(String, String)> {
    vec![("TERM".into(), "xterm-256color".into()), ("COLORTERM".into(), "truecolor".into())]
}
```

`crates/rustty-pty/src/size.rs` :

```rust
//! Taille du pseudo-terminal, en cellules et en pixels.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PtySize {
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl PtySize {
    /// Taille en cellules seulement ; un terminal de 0 cellule n'existe pas.
    pub fn new(cols: u16, rows: u16) -> Self {
        Self::with_pixels(cols, rows, 0, 0)
    }

    pub fn with_pixels(cols: u16, rows: u16, pixel_width: u16, pixel_height: u16) -> Self {
        Self { cols: cols.max(1), rows: rows.max(1), pixel_width, pixel_height }
    }
}
```

`crates/rustty-pty/src/error.rs` :

```rust
//! Erreurs du pseudo-terminal, en français, avec le programme concerné.

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("impossible d'ouvrir un pseudo-terminal : {0}")]
    Open(String),
    #[error("impossible de lancer « {program} » : {source}")]
    Spawn { program: String, source: String },
    #[error("erreur d'entrée-sortie sur le pseudo-terminal : {0}")]
    Io(#[from] std::io::Error),
}
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Si clippy signale `platform_default` comme code mort sur un OS, c'est que le `cfg` ne couvre pas cet OS : seuls `unix` et `windows` sont visés par la v0.1.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.36"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.36 — 2026-10-08 · « Crate pty »

- `rustty-pty` : `Shell` (défaut `$SHELL`, `/bin/sh` ou `powershell.exe`), environnement `TERM`/`COLORTERM`, `PtySize` bornée à une cellule, `PtyError`.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-pty CHANGELOG.md
git commit -m "rustty-pty : shell, taille et erreurs (0.1.0-alpha.36)"
```

---

### Task 2 : `Pty` — lancement, lecture, écriture, resize, état du processus

**Files:**
- Create: `crates/rustty-pty/src/pty.rs`
- Create: `crates/rustty-pty/tests/spawn.rs`
- Modify: `crates/rustty-pty/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct Pty` (non `Clone`) ; `Pty::spawn(shell: &Shell, size: PtySize, env: &[(String, String)], cwd: Option<&Path>) -> Result<Pty, PtyError>`.
  - `pub fn reader(&self) -> Result<Box<dyn Read + Send>, PtyError>` (clonable, bloquant ; `Ok(0)` à la fin du processus).
  - `pub fn write(&mut self, bytes: &[u8]) -> Result<(), PtyError>` (écriture complète + `flush`).
  - `pub fn resize(&self, size: PtySize) -> Result<(), PtyError>`.
  - `pub enum ExitStatus { Exited(u32), Signaled }` ; `pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, PtyError>` ; `pub fn wait(&mut self) -> Result<ExitStatus, PtyError>` ; `pub fn kill(&mut self) -> Result<(), PtyError>` ; `pub fn process_id(&self) -> Option<u32>`.
  - `impl Drop for Pty` : tue le processus s'il tourne encore (un terminal fermé ne laisse pas de shell orphelin).

- [ ] **Step 1: Écrire les tests**

`crates/rustty-pty/tests/spawn.rs` :

```rust
//! Tests d'intégration : un vrai pseudo-terminal, un vrai shell du système.

use std::io::Read;
use std::time::{Duration, Instant};

use rustty_pty::{ExitStatus, Pty, PtySize, Shell, default_env};

/// Le shell de test et la commande qui affiche un argument puis sort avec un code.
fn echo_then_exit(text: &str, code: u32) -> Shell {
    if cfg!(windows) {
        Shell::new("cmd.exe", vec!["/C".into(), format!("echo {text}& exit {code}")])
    } else {
        Shell::new("/bin/sh", vec!["-c".into(), format!("echo {text}; exit {code}")])
    }
}

/// Lit jusqu'à voir `needle` ou jusqu'au délai ; rend tout ce qui a été lu.
fn read_until(reader: &mut dyn Read, needle: &str, timeout: Duration) -> String {
    let start = Instant::now();
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    while start.elapsed() < timeout {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                out.extend_from_slice(&buf[..n]);
                if String::from_utf8_lossy(&out).contains(needle) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[test]
fn echo_output_is_readable() {
    let mut pty = Pty::spawn(&echo_then_exit("bonjour", 0), PtySize::new(80, 24), &default_env(), None).unwrap();
    let mut reader = pty.reader().unwrap();
    let out = read_until(&mut *reader, "bonjour", Duration::from_secs(10));
    assert!(out.contains("bonjour"), "sortie lue : {out:?}");
    assert_eq!(pty.wait().unwrap(), ExitStatus::Exited(0));
}

#[test]
fn exit_code_is_reported() {
    let mut pty = Pty::spawn(&echo_then_exit("x", 3), PtySize::new(80, 24), &default_env(), None).unwrap();
    let mut reader = pty.reader().unwrap();
    let _ = read_until(&mut *reader, "x", Duration::from_secs(10));
    let status = pty.wait().unwrap();
    assert_eq!(status, ExitStatus::Exited(3));
    assert_eq!(pty.try_wait().unwrap(), Some(ExitStatus::Exited(3)), "try_wait après wait rend le même état");
}

#[test]
fn unknown_program_is_an_error() {
    let shell = Shell::new("rustty-programme-qui-n-existe-pas", Vec::new());
    let result = Pty::spawn(&shell, PtySize::new(80, 24), &default_env(), None);
    match result {
        Err(e) => assert!(e.to_string().contains("rustty-programme-qui-n-existe-pas"), "{e}"),
        Ok(mut pty) => {
            // Certains systèmes ne signalent l'échec qu'au premier wait : le
            // processus doit alors être terminé avec un code non nul.
            let status = pty.wait().unwrap();
            assert_ne!(status, ExitStatus::Exited(0), "un programme inexistant ne réussit pas");
        }
    }
}

#[test]
fn written_input_is_echoed_back_by_an_interactive_shell() {
    let shell = if cfg!(windows) { Shell::new("cmd.exe", Vec::new()) } else { Shell::new("/bin/sh", Vec::new()) };
    let mut pty = Pty::spawn(&shell, PtySize::new(80, 24), &default_env(), None).unwrap();
    let mut reader = pty.reader().unwrap();
    pty.write(b"echo marqueur-rustty\r\n").unwrap();
    let out = read_until(&mut *reader, "marqueur-rustty", Duration::from_secs(10));
    assert!(out.contains("marqueur-rustty"), "sortie lue : {out:?}");
    pty.write(b"exit\r\n").unwrap();
    let status = pty.wait().unwrap();
    assert_eq!(status, ExitStatus::Exited(0));
}

#[test]
fn resize_is_accepted_while_running() {
    let shell = if cfg!(windows) { Shell::new("cmd.exe", Vec::new()) } else { Shell::new("/bin/sh", Vec::new()) };
    let mut pty = Pty::spawn(&shell, PtySize::new(80, 24), &default_env(), None).unwrap();
    pty.resize(PtySize::with_pixels(100, 40, 800, 600)).unwrap();
    pty.kill().unwrap();
    let _ = pty.wait();
}

#[test]
fn env_is_passed_to_the_child() {
    let shell = if cfg!(windows) {
        Shell::new("cmd.exe", vec!["/C".into(), "echo %RUSTTY_PROBE%".into()])
    } else {
        Shell::new("/bin/sh", vec!["-c".into(), "echo $RUSTTY_PROBE".into()])
    };
    let mut env = default_env();
    env.push(("RUSTTY_PROBE".into(), "valeur-sonde".into()));
    let mut pty = Pty::spawn(&shell, PtySize::new(80, 24), &env, None).unwrap();
    let mut reader = pty.reader().unwrap();
    let out = read_until(&mut *reader, "valeur-sonde", Duration::from_secs(10));
    assert!(out.contains("valeur-sonde"), "{out:?}");
    let _ = pty.wait();
}

#[test]
fn dropping_a_pty_kills_the_child() {
    let shell = if cfg!(windows) { Shell::new("cmd.exe", Vec::new()) } else { Shell::new("/bin/sh", Vec::new()) };
    let pty = Pty::spawn(&shell, PtySize::new(80, 24), &default_env(), None).unwrap();
    let mut reader = pty.reader().unwrap();
    drop(pty);
    // Le lecteur cloné voit la fin du flux : Ok(0) ou une erreur, jamais un blocage infini.
    let out = read_until(&mut *reader, "\u{0}jamais", Duration::from_secs(10));
    let _ = out;
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-pty --test spawn`
Expected: `unresolved imports rustty_pty::{ExitStatus, Pty}`.

- [ ] **Step 3: Implémenter**

`crates/rustty-pty/src/pty.rs` :

```rust
//! Façade sur `portable-pty` : un processus dans un pseudo-terminal, son
//! flux de lecture clonable, son flux d'écriture, sa taille et son état.

use std::io::{Read, Write};
use std::path::Path;

use portable_pty::{Child, CommandBuilder, MasterPty, PtySystem, native_pty_system};

use crate::error::PtyError;
use crate::shell::Shell;
use crate::size::PtySize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitStatus {
    Exited(u32),
    /// Terminé par un signal (Unix) ou tué.
    Signaled,
}

pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    program: String,
}

impl Pty {
    /// Ouvre un pseudo-terminal de `size` et y lance `shell` avec `env` en plus
    /// de l'environnement hérité, dans `cwd` s'il est donné.
    pub fn spawn(shell: &Shell, size: PtySize, env: &[(String, String)], cwd: Option<&Path>) -> Result<Self, PtyError> {
        let system = native_pty_system();
        let pair = system
            .openpty(portable_pty::PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: size.pixel_width,
                pixel_height: size.pixel_height,
            })
            .map_err(|e| PtyError::Open(e.to_string()))?;
        let mut cmd = CommandBuilder::new(&shell.program);
        cmd.args(&shell.args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Spawn { program: shell.program.clone(), source: e.to_string() })?;
        // Le côté esclave appartient à l'enfant : on le ferme ici pour que la
        // fin du processus se traduise par une fin de flux côté lecteur.
        drop(pair.slave);
        let writer = pair.master.take_writer().map_err(|e| PtyError::Open(e.to_string()))?;
        Ok(Self { master: pair.master, writer, child, program: shell.program.clone() })
    }

    /// Un lecteur bloquant indépendant ; `read` rend `Ok(0)` ou une erreur
    /// quand le processus a fermé le terminal.
    pub fn reader(&self) -> Result<Box<dyn Read + Send>, PtyError> {
        self.master.try_clone_reader().map_err(|e| PtyError::Open(e.to_string()))
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, size: PtySize) -> Result<(), PtyError> {
        self.master
            .resize(portable_pty::PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: size.pixel_width,
                pixel_height: size.pixel_height,
            })
            .map_err(|e| PtyError::Open(e.to_string()))
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// `None` tant que le processus tourne.
    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, PtyError> {
        Ok(self.child.try_wait()?.map(convert_status))
    }

    pub fn wait(&mut self) -> Result<ExitStatus, PtyError> {
        Ok(convert_status(self.child.wait()?))
    }

    pub fn kill(&mut self) -> Result<(), PtyError> {
        self.child.kill()?;
        Ok(())
    }

    pub fn program(&self) -> &str {
        &self.program
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn convert_status(status: portable_pty::ExitStatus) -> ExitStatus {
    if status.success() {
        ExitStatus::Exited(0)
    } else {
        match status.exit_code() {
            // portable-pty encode « tué par un signal » avec un code supérieur
            // à 255 sur Unix (128 + signal) ; on ne distingue pas le signal.
            code if code > 255 => ExitStatus::Signaled,
            code => ExitStatus::Exited(code),
        }
    }
}
```

Points à vérifier contre la doc de `portable-pty` 0.9 pendant l'implémentation : le nom exact des méthodes `take_writer`, `try_clone_reader`, `resize`, `process_id`, `try_wait`, `wait`, `kill`, et le type renvoyé par `exit_code()` (`u32`). `ExitStatus::success()` et `exit_code()` existent sur `portable_pty::ExitStatus`. Si `exit_code()` ne rend pas plus de 255 pour un signal, utiliser `ExitStatus::signal()` si disponible, sinon considérer tout code ≥ 128 après `kill` comme `Signaled` et le noter dans le rapport.

`lib.rs` : ajouter `pub mod pty;` et `pub use pty::{ExitStatus, Pty};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok` ; les sept tests d'intégration prennent quelques secondes. Si `dropping_a_pty_kills_the_child` bloque, c'est que le côté esclave n'est pas fermé : vérifier le `drop(pair.slave)`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.37"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.37 — 2026-10-08 · « Pseudo-terminal »

- `rustty-pty` : `Pty::spawn` lance le shell dans un pseudo-terminal ; lecture clonable, écriture, redimensionnement, code de sortie, `kill`, et le processus est tué à la destruction. Tests avec un vrai shell sur Unix et Windows.
```

```bash
git add Cargo.toml crates/rustty-pty CHANGELOG.md
git commit -m "rustty-pty : lancement, lecture, écriture et état du processus (0.1.0-alpha.37)"
```

---

### Task 3 : Thread lecteur et événements

**Files:**
- Create: `crates/rustty-pty/src/reader.rs`
- Modify: `crates/rustty-pty/src/lib.rs`
- Modify: `crates/rustty-pty/tests/spawn.rs`

**Interfaces:**
- Produces:
  - `pub enum PtyEvent { Data(Vec<u8>), Eof }` (`Eof` émis une fois, en dernier, à la fin du flux ou sur erreur de lecture).
  - `pub fn spawn_reader(reader: Box<dyn Read + Send>, tx: std::sync::mpsc::Sender<PtyEvent>) -> std::thread::JoinHandle<()>` : lit par blocs de `READ_CHUNK` (64 KiB), envoie chaque bloc tel quel, s'arrête si le récepteur est fermé.
  - `pub const READ_CHUNK: usize = 64 * 1024`.
  - `pub fn spawn_reader_with<F: FnMut(PtyEvent) + Send + 'static>(reader, sink: F) -> JoinHandle<()>` : même boucle avec un rappel au lieu d'un canal (ce que le binaire branchera sur `EventLoopProxy`).

- [ ] **Step 1: Écrire les tests**

`crates/rustty-pty/src/reader.rs`, bas de fichier (tests unitaires sur un `Read` en mémoire) :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::mpsc;

    #[test]
    fn delivers_data_then_eof_in_order() {
        let data: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let (tx, rx) = mpsc::channel();
        let handle = spawn_reader(Box::new(Cursor::new(data.clone())), tx);
        handle.join().unwrap();
        let mut received = Vec::new();
        let mut eof_seen = false;
        for ev in rx.iter() {
            match ev {
                PtyEvent::Data(chunk) => {
                    assert!(!eof_seen, "aucune donnée après Eof");
                    assert!(!chunk.is_empty() && chunk.len() <= READ_CHUNK);
                    received.extend_from_slice(&chunk);
                }
                PtyEvent::Eof => eof_seen = true,
            }
        }
        assert!(eof_seen);
        assert_eq!(received, data, "ordre et intégrité préservés");
    }

    #[test]
    fn empty_stream_yields_only_eof() {
        let (tx, rx) = mpsc::channel();
        spawn_reader(Box::new(Cursor::new(Vec::new())), tx).join().unwrap();
        assert_eq!(rx.iter().count(), 1);
    }

    #[test]
    fn stops_quietly_when_the_receiver_is_gone() {
        let data = vec![1u8; 300_000];
        let (tx, rx) = mpsc::channel();
        drop(rx);
        spawn_reader(Box::new(Cursor::new(data)), tx).join().unwrap();
    }

    #[test]
    fn callback_variant_sees_the_same_events() {
        let data = b"abc".to_vec();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink_seen = seen.clone();
        spawn_reader_with(Box::new(Cursor::new(data)), move |ev| sink_seen.lock().unwrap().push(ev))
            .join()
            .unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert!(matches!(&seen[0], PtyEvent::Data(d) if d == b"abc"));
        assert!(matches!(seen[1], PtyEvent::Eof));
    }
}
```

Ajouter dans `crates/rustty-pty/tests/spawn.rs` :

```rust
#[test]
fn large_output_is_delivered_in_order() {
    // 2 000 lignes numérotées : bien plus qu'un bloc de lecture.
    let shell = if cfg!(windows) {
        Shell::new("cmd.exe", vec!["/C".into(), "for /L %i in (1,1,2000) do @echo ligne-%i".into()])
    } else {
        Shell::new("/bin/sh", vec!["-c".into(), "i=1; while [ $i -le 2000 ]; do echo ligne-$i; i=$((i+1)); done".into()])
    };
    let mut pty = Pty::spawn(&shell, PtySize::new(200, 50), &default_env(), None).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = rustty_pty::spawn_reader(pty.reader().unwrap(), tx);
    let mut out = Vec::new();
    for ev in rx.iter() {
        match ev {
            rustty_pty::PtyEvent::Data(d) => out.extend_from_slice(&d),
            rustty_pty::PtyEvent::Eof => break,
        }
    }
    handle.join().unwrap();
    let text = String::from_utf8_lossy(&out);
    let mut expected = 1;
    for line in text.lines().map(str::trim).filter(|l| l.starts_with("ligne-")) {
        assert_eq!(line, format!("ligne-{expected}"), "ordre ou perte de lignes");
        expected += 1;
    }
    assert_eq!(expected, 2001, "les 2000 lignes sont arrivées");
    let _ = pty.wait();
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-pty`
Expected: module `reader` introuvable / `spawn_reader` inconnu.

- [ ] **Step 3: Implémenter**

`crates/rustty-pty/src/reader.rs` :

```rust
//! Thread lecteur : vide le pseudo-terminal par blocs et pousse des
//! événements vers l'interface, qui les donne au `Term`.

use std::io::Read;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

/// Taille d'un bloc de lecture. Assez grand pour absorber une sortie
/// massive, assez petit pour rester réactif.
pub const READ_CHUNK: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PtyEvent {
    Data(Vec<u8>),
    /// Fin du flux : le processus a fermé le terminal (ou lecture impossible).
    Eof,
}

/// Lit `reader` jusqu'à la fin et envoie chaque bloc dans `tx`, puis `Eof`.
pub fn spawn_reader(reader: Box<dyn Read + Send>, tx: Sender<PtyEvent>) -> JoinHandle<()> {
    spawn_reader_with(reader, move |ev| {
        // Un récepteur disparu est la seule raison d'échec : on l'ignore et
        // la boucle s'arrêtera au prochain tour via `send` qui échoue encore.
        let _ = tx.send(ev);
    })
}

/// Même boucle, avec un rappel : le binaire y branche son réveil de boucle
/// d'événements. Le rappel reçoit `Eof` en dernier.
pub fn spawn_reader_with<F>(mut reader: Box<dyn Read + Send>, mut sink: F) -> JoinHandle<()>
where
    F: FnMut(PtyEvent) + Send + 'static,
{
    std::thread::Builder::new()
        .name("rustty-pty-reader".into())
        .spawn(move || {
            let mut buf = vec![0u8; READ_CHUNK];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => sink(PtyEvent::Data(buf[..n].to_vec())),
                }
            }
            sink(PtyEvent::Eof);
        })
        .expect("création du thread lecteur")
}
```

Le test `stops_quietly_when_the_receiver_is_gone` vérifie seulement que le thread se termine : avec un récepteur fermé, `send` échoue silencieusement et la lecture continue jusqu'à `Eof`, ce qui est acceptable (le flux est fini de toute façon quand le processus meurt, et le `Pty` tue le processus à sa destruction). Note : la boucle **continue** malgré un récepteur fermé ; ce n'est pas un arrêt anticipé, et c'est voulu pour garder le code simple.

`lib.rs` : `pub mod reader;` et `pub use reader::{PtyEvent, READ_CHUNK, spawn_reader, spawn_reader_with};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Sur Windows, le `for /L` de `cmd.exe` écrit `ligne-1`…`ligne-2000` ; `str::trim` absorbe les `\r`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.38"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.38 — 2026-10-08 · « Thread lecteur »

- `rustty-pty` : `spawn_reader` lit le pseudo-terminal par blocs de 64 Kio et pousse `PtyEvent::Data` puis `Eof` dans un canal ou un rappel ; 2 000 lignes livrées dans l'ordre sans perte.
```

```bash
git add Cargo.toml crates/rustty-pty CHANGELOG.md
git commit -m "rustty-pty : thread lecteur et événements (0.1.0-alpha.38)"
```

---

### Task 4 : Crate `rustty-render` et palette de couleurs

**Files:**
- Create: `crates/rustty-render/Cargo.toml`
- Create: `crates/rustty-render/src/lib.rs`
- Create: `crates/rustty-render/src/color.rs`

**Interfaces:**
- Consumes: `rustty_vt::{Color, Style, Attrs}`, `rustty_config::{Colors, Rgb}`.
- Produces:
  - `pub struct Rgba { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }` (Copy, PartialEq, Default = transparent) ; `Rgba::new(r, g, b, a)`, `Rgba::from_rgb(Rgb) -> Rgba` (alpha 1), `Rgba::with_alpha(self, a)`, `Rgba::to_array(self) -> [f32; 4]`, `Rgba::to_u8(self) -> [u8; 4]` (arrondi), `Rgba::dim(self, factor: f32)` (multiplie r, g, b).
  - `pub struct Palette { pub foreground: Rgba, pub background: Rgba, pub cursor: Rgba, pub selection: Rgba, pub ansi: [Rgba; 16], pub bold_is_bright: bool }` ; `Palette::from_config(colors: &Colors, bold_is_bright: bool) -> Palette`.
  - `Palette::resolve(&self, color: Color, default: Rgba) -> Rgba` : `Default` → `default` ; `Indexed(0..=15)` → `ansi` ; `Indexed(16..=231)` → cube 6×6×6 (`16 + 36r + 6g + b`, niveaux `[0, 95, 135, 175, 215, 255]`) ; `Indexed(232..=255)` → gris `8 + 10·(n−232)` ; `Rgb` → tel quel.
  - `Palette::cell_colors(&self, style: &Style) -> (Rgba, Rgba)` (avant-plan, arrière-plan) : applique `bold_is_bright` (BOLD + `Indexed(0..=7)` → `+8`), `INVERSE` (échange), `DIM` (avant-plan × 0,66), `HIDDEN` (avant-plan = arrière-plan).

- [ ] **Step 1: Écrire les tests**

`crates/rustty-render/src/color.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustty_config::Colors;
    use rustty_vt::{Attrs, Color, Style};

    fn palette() -> Palette {
        Palette::from_config(&Colors::default(), false)
    }

    #[test]
    fn rgba_conversions() {
        let c = Rgba::from_rgb(Rgb::new(255, 0, 128));
        assert_eq!(c.to_array(), [1.0, 0.0, 128.0 / 255.0, 1.0]);
        assert_eq!(c.to_u8(), [255, 0, 128, 255]);
        assert_eq!(c.with_alpha(0.5).a, 0.5);
        assert_eq!(c.dim(0.5).to_u8(), [128, 0, 64, 255]);
    }

    #[test]
    fn default_colors_come_from_config() {
        let p = palette();
        assert_eq!(p.background.to_u8(), [0x1e, 0x1e, 0x2e, 255]);
        assert_eq!(p.ansi[1].to_u8(), [0xf3, 0x8b, 0xa8, 255]);
        assert_eq!(p.resolve(Color::Default, p.foreground), p.foreground);
        assert_eq!(p.resolve(Color::Indexed(1), p.foreground), p.ansi[1]);
    }

    #[test]
    fn cube_and_grayscale_indices() {
        let p = palette();
        assert_eq!(p.resolve(Color::Indexed(16), p.foreground).to_u8(), [0, 0, 0, 255]);
        assert_eq!(p.resolve(Color::Indexed(231), p.foreground).to_u8(), [255, 255, 255, 255]);
        assert_eq!(p.resolve(Color::Indexed(196), p.foreground).to_u8(), [255, 0, 0, 255], "16 + 36*5 = rouge pur");
        assert_eq!(p.resolve(Color::Indexed(232), p.foreground).to_u8(), [8, 8, 8, 255]);
        assert_eq!(p.resolve(Color::Indexed(255), p.foreground).to_u8(), [238, 238, 238, 255]);
        assert_eq!(p.resolve(Color::Rgb(1, 2, 3), p.foreground).to_u8(), [1, 2, 3, 255]);
    }

    #[test]
    fn cell_colors_apply_inverse_dim_hidden_and_bold_is_bright() {
        let mut p = palette();
        let plain = Style { fg: Color::Indexed(1), bg: Color::Indexed(4), attrs: Attrs::empty() };
        assert_eq!(p.cell_colors(&plain), (p.ansi[1], p.ansi[4]));
        let inverse = Style { attrs: Attrs::INVERSE, ..plain };
        assert_eq!(p.cell_colors(&inverse), (p.ansi[4], p.ansi[1]));
        let hidden = Style { attrs: Attrs::HIDDEN, ..plain };
        assert_eq!(p.cell_colors(&hidden), (p.ansi[4], p.ansi[4]));
        let dim = Style { fg: Color::Rgb(100, 100, 100), attrs: Attrs::DIM, ..plain };
        assert_eq!(p.cell_colors(&dim).0.to_u8(), [66, 66, 66, 255]);
        let bold = Style { attrs: Attrs::BOLD, ..plain };
        assert_eq!(p.cell_colors(&bold).0, p.ansi[1], "sans bold_is_bright, le gras ne change pas la couleur");
        p.bold_is_bright = true;
        assert_eq!(p.cell_colors(&bold).0, p.ansi[9]);
        let bold_rgb = Style { fg: Color::Rgb(1, 2, 3), attrs: Attrs::BOLD, ..plain };
        assert_eq!(p.cell_colors(&bold_rgb).0.to_u8(), [1, 2, 3, 255], "seules les couleurs 0–7 s'éclaircissent");
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render`
Expected: `did not match any packages`.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/Cargo.toml` :

```toml
[package]
name = "rustty-render"
description = "Rendu wgpu de la grille de terminal et du chrome de l'interface"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
rustty-config = { path = "../rustty-config" }
rustty-vt = { path = "../rustty-vt" }
thiserror.workspace = true

[dev-dependencies]

[lints]
workspace = true
```

`crates/rustty-render/src/lib.rs` :

```rust
//! Rendu GPU : transforme un `Snapshot` de `rustty-vt` et le chrome de
//! l'interface en passes wgpu. Testé hors écran, sans fenêtre.

pub mod color;

pub use color::{Palette, Rgba};
```

`crates/rustty-render/src/color.rs` :

```rust
//! Résolution des couleurs : des `Color` abstraites de l'émulation vers des
//! `Rgba` concrètes, selon la palette de la configuration.

use rustty_config::{Colors, Rgb};
use rustty_vt::{Attrs, Color, Style};

/// Couleur flottante non pré-multipliée, composantes dans [0, 1].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgb(rgb: Rgb) -> Self {
        Self::new(f32::from(rgb.r) / 255.0, f32::from(rgb.g) / 255.0, f32::from(rgb.b) / 255.0, 1.0)
    }

    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_u8(self) -> [u8; 4] {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        [q(self.r), q(self.g), q(self.b), q(self.a)]
    }

    /// Assombrit les composantes de couleur, pas l'alpha.
    pub fn dim(self, factor: f32) -> Self {
        Self::new(self.r * factor, self.g * factor, self.b * factor, self.a)
    }
}

/// Facteur appliqué à l'avant-plan d'une cellule « atténuée » (SGR 2).
pub const DIM_FACTOR: f32 = 0.66;

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub foreground: Rgba,
    pub background: Rgba,
    pub cursor: Rgba,
    pub selection: Rgba,
    pub ansi: [Rgba; 16],
    pub bold_is_bright: bool,
}

impl Palette {
    pub fn from_config(colors: &Colors, bold_is_bright: bool) -> Self {
        Self {
            foreground: Rgba::from_rgb(colors.foreground),
            background: Rgba::from_rgb(colors.background),
            cursor: Rgba::from_rgb(colors.cursor),
            selection: Rgba::from_rgb(colors.selection_background),
            ansi: colors.palette.map(Rgba::from_rgb),
            bold_is_bright,
        }
    }

    /// Couleur concrète ; `default` sert pour `Color::Default` (avant-plan
    /// ou arrière-plan selon l'appelant).
    pub fn resolve(&self, color: Color, default: Rgba) -> Rgba {
        match color {
            Color::Default => default,
            Color::Indexed(n @ 0..=15) => self.ansi[usize::from(n)],
            Color::Indexed(n @ 16..=231) => {
                const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
                let i = usize::from(n - 16);
                Rgba::from_rgb(Rgb::new(LEVELS[i / 36], LEVELS[(i / 6) % 6], LEVELS[i % 6]))
            }
            Color::Indexed(n) => {
                let v = 8 + 10 * (n - 232);
                Rgba::from_rgb(Rgb::new(v, v, v))
            }
            Color::Rgb(r, g, b) => Rgba::from_rgb(Rgb::new(r, g, b)),
        }
    }

    /// Avant-plan et arrière-plan effectifs d'une cellule, attributs appliqués.
    pub fn cell_colors(&self, style: &Style) -> (Rgba, Rgba) {
        let attrs = style.attrs;
        let fg_color = match style.fg {
            Color::Indexed(n @ 0..=7) if self.bold_is_bright && attrs.contains(Attrs::BOLD) => Color::Indexed(n + 8),
            other => other,
        };
        let mut fg = self.resolve(fg_color, self.foreground);
        let mut bg = self.resolve(style.bg, self.background);
        if attrs.contains(Attrs::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }
        if attrs.contains(Attrs::DIM) {
            fg = fg.dim(DIM_FACTOR);
        }
        if attrs.contains(Attrs::HIDDEN) {
            fg = bg;
        }
        (fg, bg)
    }
}
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. `(100 × 0,66).round() = 66` ; `128 × 0,5 = 64`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.39"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.39 — 2026-10-08 · « Crate render et palette »

- `rustty-render` : `Rgba` et `Palette` (16 couleurs ANSI, cube 6×6×6, gris, vraies couleurs ; gras vif, inversion, atténuation, texte caché).
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-render CHANGELOG.md
git commit -m "rustty-render : crate et palette de couleurs (0.1.0-alpha.39)"
```

---

### Task 5 : Polices — chargement, variantes, repli, métriques de cellule

**Files:**
- Create: `crates/rustty-render/assets/DejaVuSansMono.ttf` (copie de `/usr/share/fonts/TTF/DejaVuSansMono.ttf`, 343 140 octets)
- Create: `crates/rustty-render/assets/LICENSE-DejaVu` (copie de `/usr/share/licenses/ttf-dejavu/LICENSE`)
- Create: `crates/rustty-render/src/font/mod.rs`
- Create: `crates/rustty-render/src/font/metrics.rs`
- Create: `crates/rustty-render/src/font/loader.rs`
- Modify: `crates/rustty-render/src/lib.rs`
- Modify: `crates/rustty-render/Cargo.toml` (`fontdb`, `swash`)

**Interfaces:**
- Consumes: `rustty_vt::Attrs`.
- Produces:
  - `pub enum Variant { Regular, Bold, Italic, BoldItalic }` ; `Variant::from_attrs(attrs: Attrs) -> Variant` ; `Variant::ALL: [Variant; 4]` ; `Variant::index(self) -> usize`.
  - `pub enum FontError { NoMonospace, Invalid(String) }`.
  - `pub struct CellMetrics { pub width: u32, pub height: u32, pub baseline: u32, pub underline_y: u32, pub underline_thickness: u32, pub strike_y: u32 }` ; `CellMetrics::from_font(font: &swash::FontRef, size_px: f32) -> CellMetrics` (jamais de 0 en largeur/hauteur).
  - `pub struct FaceData { pub data: Arc<dyn AsRef<[u8]> + Send + Sync>, pub index: u32 }` ; `FaceData::font_ref(&self) -> Option<swash::FontRef<'_>>`.
  - `pub struct FontSet` ; `FontSet::load(family: &str, size_px: f32) -> Result<FontSet, FontError>` (polices système : la famille demandée, sinon une liste de familles à chasse fixe connues, sinon la première face `monospaced` de la base, sinon la police embarquée) ; `FontSet::embedded(size_px) -> FontSet` (DejaVu Sans Mono embarquée pour les quatre variantes) ; `FontSet::from_bytes(data: Vec<u8>, size_px) -> Result<FontSet, FontError>` ; `size_px()`, `metrics()`, `face(&self, v: Variant) -> &FaceData`, `family_name() -> &str`.
  - `pub const EMBEDDED_FONT: &[u8] = include_bytes!("../../assets/DejaVuSansMono.ttf");`

- [ ] **Step 1: Écrire les tests**

`crates/rustty-render/src/font/metrics.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::loader::EMBEDDED_FONT;

    #[test]
    fn metrics_of_the_embedded_font_are_sane() {
        let font = swash::FontRef::from_index(EMBEDDED_FONT, 0).unwrap();
        let m = CellMetrics::from_font(&font, 16.0);
        assert!(m.width >= 8 && m.width <= 12, "{m:?}");
        assert!(m.height >= 16 && m.height <= 22, "{m:?}");
        assert!(m.baseline > 0 && m.baseline < m.height, "{m:?}");
        assert!(m.underline_y > m.baseline && m.underline_y < m.height, "{m:?}");
        assert!(m.underline_thickness >= 1);
        assert!(m.strike_y > 0 && m.strike_y < m.baseline, "{m:?}");
    }

    #[test]
    fn metrics_scale_with_size() {
        let font = swash::FontRef::from_index(EMBEDDED_FONT, 0).unwrap();
        let small = CellMetrics::from_font(&font, 10.0);
        let big = CellMetrics::from_font(&font, 30.0);
        assert!(big.width > small.width * 2 && big.height > small.height * 2);
    }

    #[test]
    fn tiny_size_never_yields_zero_cells() {
        let font = swash::FontRef::from_index(EMBEDDED_FONT, 0).unwrap();
        let m = CellMetrics::from_font(&font, 0.1);
        assert!(m.width >= 1 && m.height >= 1 && m.underline_y < m.height);
    }
}
```

`crates/rustty-render/src/font/loader.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_font_loads_all_variants() {
        let set = FontSet::embedded(14.0);
        for v in Variant::ALL {
            assert!(set.face(v).font_ref().is_some(), "{v:?}");
        }
        assert_eq!(set.size_px(), 14.0);
        assert!(set.metrics().width > 0);
        assert_eq!(set.family_name(), "DejaVu Sans Mono");
    }

    #[test]
    fn from_bytes_rejects_garbage() {
        assert!(matches!(FontSet::from_bytes(vec![0u8; 100], 12.0), Err(FontError::Invalid(_))));
    }

    #[test]
    fn unknown_family_falls_back() {
        let set = FontSet::load("Police-Inexistante-Rustty-42", 12.0).unwrap();
        assert!(set.metrics().width > 0 && set.metrics().height > 0);
        assert!(set.face(Variant::Bold).font_ref().is_some());
    }

    #[test]
    fn cell_metrics_are_positive_for_the_system_monospace() {
        let set = FontSet::load("monospace", 11.0).unwrap();
        let m = set.metrics();
        assert!(m.width > 0 && m.height > m.baseline && m.baseline > 0, "{m:?}");
    }

    #[test]
    fn variant_from_attrs() {
        use rustty_vt::Attrs;
        assert_eq!(Variant::from_attrs(Attrs::empty()), Variant::Regular);
        assert_eq!(Variant::from_attrs(Attrs::BOLD), Variant::Bold);
        assert_eq!(Variant::from_attrs(Attrs::ITALIC), Variant::Italic);
        assert_eq!(Variant::from_attrs(Attrs::BOLD | Attrs::ITALIC | Attrs::UNDERLINE), Variant::BoldItalic);
        assert_eq!(Variant::BoldItalic.index(), 3);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render`
Expected: module `font` introuvable.

- [ ] **Step 3: Implémenter**

Copier la police et sa licence :

```bash
mkdir -p crates/rustty-render/assets
cp /usr/share/fonts/TTF/DejaVuSansMono.ttf crates/rustty-render/assets/DejaVuSansMono.ttf
cp /usr/share/licenses/ttf-dejavu/LICENSE crates/rustty-render/assets/LICENSE-DejaVu
```

`crates/rustty-render/Cargo.toml`, `[dependencies]` : ajouter `fontdb.workspace = true` et `swash.workspace = true`.

`crates/rustty-render/src/font/mod.rs` :

```rust
//! Polices : découverte (`fontdb`), variantes gras/italique, repli, métriques
//! de cellule, rastérisation (`swash`).

pub mod loader;
pub mod metrics;

use rustty_vt::Attrs;

pub use loader::{EMBEDDED_FONT, FaceData, FontSet};
pub use metrics::CellMetrics;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl Variant {
    pub const ALL: [Variant; 4] = [Variant::Regular, Variant::Bold, Variant::Italic, Variant::BoldItalic];

    pub fn from_attrs(attrs: Attrs) -> Self {
        match (attrs.contains(Attrs::BOLD), attrs.contains(Attrs::ITALIC)) {
            (false, false) => Self::Regular,
            (true, false) => Self::Bold,
            (false, true) => Self::Italic,
            (true, true) => Self::BoldItalic,
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Regular => 0,
            Self::Bold => 1,
            Self::Italic => 2,
            Self::BoldItalic => 3,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("aucune police à chasse fixe trouvée")]
    NoMonospace,
    #[error("police illisible : {0}")]
    Invalid(String),
}
```

`crates/rustty-render/src/font/metrics.rs` :

```rust
//! Géométrie d'une cellule déduite d'une police à une taille donnée.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellMetrics {
    pub width: u32,
    pub height: u32,
    /// Distance du haut de la cellule à la ligne de base.
    pub baseline: u32,
    /// Ligne du soulignement, depuis le haut de la cellule.
    pub underline_y: u32,
    pub underline_thickness: u32,
    /// Ligne du barré, depuis le haut de la cellule.
    pub strike_y: u32,
}

impl CellMetrics {
    pub fn from_font(font: &swash::FontRef<'_>, size_px: f32) -> Self {
        let m = font.metrics(&[]).scale(size_px);
        let advance = font
            .charmap()
            .map('0')
            .into_iter()
            .map(|gid| font.glyph_metrics(&[]).scale(size_px).advance_width(gid))
            .find(|a| *a > 0.0)
            .unwrap_or(m.average_width.max(m.max_width));
        let width = advance.round().max(1.0) as u32;
        let height = (m.ascent + m.descent + m.leading).round().max(1.0) as u32;
        let baseline = m.ascent.round().clamp(1.0, height as f32) as u32;
        let underline_y = (baseline as f32 - m.underline_offset).round().clamp(baseline as f32 + 1.0, (height - 1) as f32) as u32;
        let strike_y = (baseline as f32 - m.strikeout_offset).round().clamp(1.0, (baseline - 1).max(1) as f32) as u32;
        Self {
            width,
            height,
            baseline,
            underline_y: underline_y.min(height - 1),
            underline_thickness: m.stroke_size.round().max(1.0) as u32,
            strike_y,
        }
    }
}
```

Si `swash::Metrics` ne porte pas `strikeout_offset` dans la version résolue, utiliser `x_height / 2` au-dessus de la ligne de base ; si `charmap().map` rend `GlyphId` (u16) et non `Option`, remplacer par `let gid = font.charmap().map('0'); if gid != 0 {...}`. Dans `swash` 0.2, `charmap().map(ch)` rend `u16` (0 = glyphe manquant) : écrire alors `let gid = font.charmap().map('0'); let advance = if gid != 0 { font.glyph_metrics(&[]).scale(size_px).advance_width(gid) } else { 0.0 }; let advance = if advance > 0.0 { advance } else { m.average_width.max(m.max_width) };`. Consigner la forme retenue.

`crates/rustty-render/src/font/loader.rs` :

```rust
//! Chargement des polices : la famille demandée ou un repli à chasse fixe,
//! en quatre variantes, plus la police embarquée comme dernier recours.

use std::sync::Arc;

use fontdb::{Database, Family, Query, Stretch, Style, Weight};

use super::{CellMetrics, FontError, Variant};

/// DejaVu Sans Mono, licence Bitstream Vera : rendu identique sur tous les OS
/// et dernier recours si le système n'a aucune police à chasse fixe.
pub const EMBEDDED_FONT: &[u8] = include_bytes!("../../assets/DejaVuSansMono.ttf");

/// Familles essayées quand celle de la configuration manque, du plus courant
/// au plus ancien, sur les trois OS.
const FALLBACK_FAMILIES: &[&str] = &["DejaVu Sans Mono", "Liberation Mono", "Menlo", "Consolas", "Noto Sans Mono", "Courier New"];

#[derive(Clone)]
pub struct FaceData {
    pub data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    pub index: u32,
}

impl FaceData {
    pub fn font_ref(&self) -> Option<swash::FontRef<'_>> {
        swash::FontRef::from_index((*self.data).as_ref(), self.index as usize)
    }
}

pub struct FontSet {
    db: Database,
    faces: [FaceData; 4],
    family_name: String,
    size_px: f32,
    metrics: CellMetrics,
}

impl FontSet {
    /// Polices système ; `family` d'abord, puis les replis.
    pub fn load(family: &str, size_px: f32) -> Result<Self, FontError> {
        let mut db = Database::new();
        db.load_system_fonts();
        let candidates = std::iter::once(family).chain(FALLBACK_FAMILIES.iter().copied());
        for name in candidates {
            if let Some(regular) = query(&db, name, Variant::Regular) {
                return Self::build(db, name, regular, size_px);
            }
        }
        if let Some(info) = db.faces().find(|f| f.monospaced) {
            let name = info.families.first().map(|(n, _)| n.clone()).unwrap_or_default();
            let id = info.id;
            return Self::build(db, &name, id, size_px);
        }
        Ok(Self::embedded(size_px))
    }

    /// Les quatre variantes pointent sur la police embarquée.
    pub fn embedded(size_px: f32) -> Self {
        Self::from_bytes(EMBEDDED_FONT.to_vec(), size_px).expect("la police embarquée est valide")
    }

    pub fn from_bytes(data: Vec<u8>, size_px: f32) -> Result<Self, FontError> {
        let face = FaceData { data: Arc::new(data), index: 0 };
        let font = face.font_ref().ok_or_else(|| FontError::Invalid("format non reconnu".into()))?;
        let metrics = CellMetrics::from_font(&font, size_px);
        let family_name = font
            .localized_strings()
            .find_by_id(swash::StringId::Family, None)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "embedded".into());
        Ok(Self { db: Database::new(), faces: [face.clone(), face.clone(), face.clone(), face], family_name, size_px, metrics })
    }

    fn build(db: Database, family: &str, regular: fontdb::ID, size_px: f32) -> Result<Self, FontError> {
        let regular_face = load_face(&db, regular)?;
        let variant_face = |v: Variant| query(&db, family, v).and_then(|id| load_face(&db, id).ok()).unwrap_or_else(|| regular_face.clone());
        let faces = [regular_face.clone(), variant_face(Variant::Bold), variant_face(Variant::Italic), variant_face(Variant::BoldItalic)];
        let font = regular_face.font_ref().ok_or_else(|| FontError::Invalid(family.into()))?;
        let metrics = CellMetrics::from_font(&font, size_px);
        Ok(Self { db, faces, family_name: family.to_string(), size_px, metrics })
    }

    pub fn size_px(&self) -> f32 {
        self.size_px
    }

    pub fn metrics(&self) -> CellMetrics {
        self.metrics
    }

    pub fn face(&self, variant: Variant) -> &FaceData {
        &self.faces[variant.index()]
    }

    pub fn family_name(&self) -> &str {
        &self.family_name
    }

    pub(crate) fn database(&self) -> &Database {
        &self.db
    }
}

fn query(db: &Database, family: &str, variant: Variant) -> Option<fontdb::ID> {
    let (weight, style) = match variant {
        Variant::Regular => (Weight::NORMAL, Style::Normal),
        Variant::Bold => (Weight::BOLD, Style::Normal),
        Variant::Italic => (Weight::NORMAL, Style::Italic),
        Variant::BoldItalic => (Weight::BOLD, Style::Italic),
    };
    let families = [Family::Name(family)];
    db.query(&Query { families: &families, weight, stretch: Stretch::Normal, style })
}

pub(crate) fn load_face(db: &Database, id: fontdb::ID) -> Result<FaceData, FontError> {
    let (data, index) = db.make_shared_face_data(id).ok_or_else(|| FontError::Invalid(format!("{id:?}")))?;
    Ok(FaceData { data, index })
}
```

Notes d'adaptation : `make_shared_face_data` existe dans `fontdb` ≥ 0.14 et rend `Option<(Arc<dyn AsRef<[u8]> + Sync + Send>, u32)>` ; `Family::Name` prend un `&str` ; `swash::FontRef::localized_strings().find_by_id(StringId::Family, None)` donne le nom de famille (sinon prendre `"DejaVu Sans Mono"` en dur pour `embedded` et `family` pour `build`, et consigner). `database()` est consommé par la tâche 6 : si clippy le signale ici en code mort, l'ajouter en tâche 6.

`lib.rs` : `pub mod font;` et `pub use font::{CellMetrics, FaceData, FontError, FontSet, Variant};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Sur un système sans police à chasse fixe (peu probable en CI), `unknown_family_falls_back` passe grâce à la police embarquée.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.40"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.40 — 2026-10-08 · « Polices »

- `rustty-render` : `FontSet` charge la famille configurée ou un repli à chasse fixe (quatre variantes), police DejaVu Sans Mono embarquée en dernier recours et pour les tests, métriques de cellule (largeur, hauteur, ligne de base, soulignement, barré).
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-render/Cargo.toml crates/rustty-render/src crates/rustty-render/assets/DejaVuSansMono.ttf crates/rustty-render/assets/LICENSE-DejaVu CHANGELOG.md
git commit -m "rustty-render : chargement des polices et métriques de cellule (0.1.0-alpha.40)"
```

---

### Task 6 : Rastérisation des glyphes et repli par couverture

**Files:**
- Create: `crates/rustty-render/src/font/raster.rs`
- Modify: `crates/rustty-render/src/font/loader.rs` (`glyph`, faces de repli)
- Modify: `crates/rustty-render/src/font/mod.rs`

**Interfaces:**
- Produces:
  - `pub struct GlyphBitmap { pub width: u32, pub height: u32, pub left: i32, pub top: i32, pub data: Vec<u8>, pub is_color: bool }` — `data` est toujours RGBA8 (`width × height × 4`) ; pour un glyphe monochrome, RGB = 255 et A = couverture ; `left`/`top` : décalage du coin haut-gauche du bitmap par rapport à l'origine (ligne de base), `top` positif vers le haut comme chez swash.
  - `pub struct Rasterizer` ; `Rasterizer::new()` ; `rasterize(&mut self, face: &FaceData, size_px: f32, glyph_id: u16) -> Option<GlyphBitmap>` (`None` si la face est illisible ou si l'image est vide).
  - `pub struct GlyphRef { pub slot: usize, pub glyph_id: u16 }` ; `FontSet::glyph(&mut self, ch: char, variant: Variant) -> Option<GlyphRef>` : la variante demandée, puis les trois autres, puis les faces de la base système qui couvrent `ch` (chargées à la demande, résultat mémorisé par caractère), puis la police embarquée ; `None` si rien ne couvre `ch`. `FontSet::face_by_slot(&self, slot) -> &FaceData` (slots 0–3 = variantes, ≥ 4 = replis, dans l'ordre de découverte).

- [ ] **Step 1: Écrire les tests**

`crates/rustty-render/src/font/raster.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{FontSet, Variant};

    fn glyph_of(set: &mut FontSet, ch: char) -> (FaceData, u16) {
        let g = set.glyph(ch, Variant::Regular).unwrap_or_else(|| panic!("{ch:?} introuvable"));
        (set.face_by_slot(g.slot).clone(), g.glyph_id)
    }

    #[test]
    fn letter_a_has_ink_inside_the_cell() {
        let mut set = FontSet::embedded(16.0);
        let (face, gid) = glyph_of(&mut set, 'A');
        let bmp = Rasterizer::new().rasterize(&face, 16.0, gid).unwrap();
        assert!(bmp.width > 0 && bmp.height > 0);
        assert_eq!(bmp.data.len() as u32, bmp.width * bmp.height * 4);
        assert!(!bmp.is_color);
        let ink: u32 = bmp.data.chunks(4).map(|px| u32::from(px[3])).sum();
        assert!(ink > 255 * 20, "trop peu d'encre : {ink}");
        assert!(bmp.data.chunks(4).all(|px| px[0] == 255 && px[1] == 255 && px[2] == 255), "monochrome = blanc + alpha");
        let m = set.metrics();
        assert!(bmp.width <= m.width + 2 && bmp.height <= m.height + 2, "le A tient dans la cellule : {bmp:?} vs {m:?}");
        assert!(bmp.top > 0 && bmp.top as u32 <= m.baseline, "au-dessus de la ligne de base");
    }

    #[test]
    fn space_has_no_image() {
        let mut set = FontSet::embedded(16.0);
        let (face, gid) = glyph_of(&mut set, ' ');
        assert!(Rasterizer::new().rasterize(&face, 16.0, gid).is_none());
    }

    #[test]
    fn missing_glyph_is_none_not_panic() {
        let mut set = FontSet::embedded(16.0);
        // Un caractère de contrôle peut mapper vers .notdef ou vers rien : ce qui
        // compte est l'absence de panique et un bitmap exploitable ou None.
        let _ = set.glyph('\u{1}', Variant::Regular);
        let r = set.glyph('\u{10FFFF}', Variant::Bold);
        if let Some(g) = r {
            let face = set.face_by_slot(g.slot).clone();
            let _ = Rasterizer::new().rasterize(&face, 16.0, g.glyph_id);
        }
    }

    #[test]
    fn replacement_character_comes_from_the_embedded_font() {
        let mut set = FontSet::embedded(16.0);
        let g = set.glyph('\u{FFFD}', Variant::Regular).expect("DejaVu couvre U+FFFD");
        assert!(g.slot < 4);
    }

    #[test]
    fn coverage_fallback_finds_another_face_or_none() {
        // Sur un système avec une police CJK ou emoji, le slot est ≥ 4 ; sinon None. Jamais de panique.
        let mut set = FontSet::load("Police-Inexistante-Rustty", 16.0).unwrap();
        for ch in ['漢', '😀'] {
            match set.glyph(ch, Variant::Regular) {
                Some(g) => {
                    let face = set.face_by_slot(g.slot).clone();
                    let bmp = Rasterizer::new().rasterize(&face, 16.0, g.glyph_id);
                    assert!(bmp.is_none() || bmp.unwrap().width > 0);
                }
                None => eprintln!("aucune police système ne couvre {ch:?}"),
            }
        }
        let first = set.glyph('漢', Variant::Regular);
        let second = set.glyph('漢', Variant::Regular);
        assert_eq!(first.map(|g| g.slot), second.map(|g| g.slot), "résultat mémorisé");
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render`
Expected: module `raster` introuvable, `glyph` inconnu.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/src/font/raster.rs` :

```rust
//! Rastérisation d'un glyphe en bitmap RGBA8 via swash : contours, contours
//! couleur (COLR) et bitmaps couleur (emoji) sont tous ramenés au même format.

use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;

use super::FaceData;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlyphBitmap {
    pub width: u32,
    pub height: u32,
    /// Décalage horizontal du bitmap par rapport à l'origine du glyphe.
    pub left: i32,
    /// Distance de la ligne de base au haut du bitmap, positive vers le haut.
    pub top: i32,
    /// RGBA8, `width × height × 4`.
    pub data: Vec<u8>,
    pub is_color: bool,
}

pub struct Rasterizer {
    ctx: ScaleContext,
}

impl Default for Rasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rasterizer {
    pub fn new() -> Self {
        Self { ctx: ScaleContext::new() }
    }

    pub fn rasterize(&mut self, face: &FaceData, size_px: f32, glyph_id: u16) -> Option<GlyphBitmap> {
        let font = face.font_ref()?;
        let mut scaler = self.ctx.builder(font).size(size_px).hint(true).build();
        let image = Render::new(&[Source::ColorOutline(0), Source::ColorBitmap(StrikeWith::BestFit), Source::Outline])
            .format(Format::Alpha)
            .render(&mut scaler, glyph_id)?;
        let (width, height) = (image.placement.width, image.placement.height);
        if width == 0 || height == 0 {
            return None;
        }
        let (data, is_color) = match image.content {
            Content::Mask => (image.data.iter().flat_map(|&a| [255, 255, 255, a]).collect(), false),
            Content::Color => (image.data.clone(), true),
            Content::SubpixelMask => (
                image.data.chunks(3).flat_map(|rgb| [255, 255, 255, rgb.iter().copied().max().unwrap_or(0)]).collect(),
                false,
            ),
        };
        Some(GlyphBitmap { width, height, left: image.placement.left, top: image.placement.top, data, is_color })
    }
}
```

Adaptations possibles : dans `swash` 0.2, `Render::render` prend un `GlyphId` (`u16`) ; `Format` vient de `swash::zeno::Format` (réexporté) ; `Content::Color` contient des pixels RGBA déjà pré-multipliés ou non selon la source — pour la v0.1 on les prend tels quels. Si `Content::SubpixelMask` n'existe pas, retirer ce bras.

Dans `font/loader.rs`, ajouter au `struct FontSet` les champs `fallbacks: Vec<FaceData>` (faces de repli chargées, slots ≥ 4) et `coverage: HashMap<char, Option<usize>>` (slot mémorisé par caractère), initialisés vides dans `from_bytes` et `build` (ajouter `use std::collections::HashMap;`). Puis :

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphRef {
    /// 0–3 : variante demandée ou voisine ; ≥ 4 : face de repli.
    pub slot: usize,
    pub glyph_id: u16,
}

impl FontSet {
    pub fn face_by_slot(&self, slot: usize) -> &FaceData {
        if slot < 4 { &self.faces[slot] } else { &self.fallbacks[slot - 4] }
    }

    /// Le glyphe de `ch` : la variante demandée, les autres variantes, puis une
    /// police du système qui couvre `ch`, puis la police embarquée.
    pub fn glyph(&mut self, ch: char, variant: Variant) -> Option<GlyphRef> {
        let order = [variant.index(), 0, 1, 2, 3];
        for slot in order {
            if let Some(gid) = glyph_in(&self.faces[slot], ch) {
                return Some(GlyphRef { slot, glyph_id: gid });
            }
        }
        if let Some(cached) = self.coverage.get(&ch) {
            return cached.map(|slot| GlyphRef { slot, glyph_id: glyph_in(self.face_by_slot(slot), ch).unwrap_or(0) });
        }
        let found = self.find_fallback(ch);
        self.coverage.insert(ch, found.map(|g| g.slot));
        found
    }

    fn find_fallback(&mut self, ch: char) -> Option<GlyphRef> {
        for (i, face) in self.fallbacks.iter().enumerate() {
            if let Some(gid) = glyph_in(face, ch) {
                return Some(GlyphRef { slot: 4 + i, glyph_id: gid });
            }
        }
        let ids: Vec<fontdb::ID> = self.db.faces().map(|f| f.id).collect();
        for id in ids {
            let Ok(face) = load_face(&self.db, id) else { continue };
            if let Some(gid) = glyph_in(&face, ch) {
                self.fallbacks.push(face);
                return Some(GlyphRef { slot: 4 + self.fallbacks.len() - 1, glyph_id: gid });
            }
        }
        let embedded = FaceData { data: Arc::new(EMBEDDED_FONT.to_vec()), index: 0 };
        let gid = glyph_in(&embedded, ch)?;
        self.fallbacks.push(embedded);
        Some(GlyphRef { slot: 4 + self.fallbacks.len() - 1, glyph_id: gid })
    }
}

/// Identifiant du glyphe de `ch` dans `face`, `None` si la face ne le couvre pas.
fn glyph_in(face: &FaceData, ch: char) -> Option<u16> {
    let font = face.font_ref()?;
    match font.charmap().map(ch) {
        0 => None,
        gid => Some(gid),
    }
}
```

Retirer `database()` si plus rien ne l'utilise. Le parcours de toutes les faces du système pour un caractère inconnu peut prendre quelques centaines de millisecondes la première fois (chargement des fichiers) ; le résultat est mémorisé par caractère, et c'est acceptable pour la v0.1 (noté comme optimisation future : index de couverture).

`font/mod.rs` : `pub mod raster;` et `pub use loader::GlyphRef; pub use raster::{GlyphBitmap, Rasterizer};`. `lib.rs` : réexporter `GlyphBitmap, GlyphRef, Rasterizer`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Si `letter_a_has_ink_inside_the_cell` échoue sur `bmp.width <= m.width + 2`, vérifier que `size_px` passé au scaler est bien le même que celui des métriques.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.41"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.41 — 2026-10-08 · « Rastérisation »

- `rustty-render` : `Rasterizer` (swash) produit des bitmaps RGBA8 pour contours, contours couleur et emojis bitmap ; `FontSet::glyph` cherche le glyphe dans les variantes, puis dans les polices du système qui couvrent le caractère, puis dans la police embarquée.
```

```bash
git add Cargo.toml crates/rustty-render CHANGELOG.md
git commit -m "rustty-render : rastérisation des glyphes et repli par couverture (0.1.0-alpha.41)"
```

---

### Task 7 : Glyphes procéduraux — lignes de boîte, blocs, powerline

**Files:**
- Create: `crates/rustty-render/src/builtin.rs`
- Modify: `crates/rustty-render/src/lib.rs`

**Interfaces:**
- Consumes: `CellMetrics`, `GlyphBitmap`.
- Produces: `pub fn builtin_glyph(ch: char, metrics: CellMetrics) -> Option<GlyphBitmap>` : un bitmap monochrome qui remplit exactement la cellule (`width = metrics.width`, `height = metrics.height`, `left = 0`, `top = metrics.baseline`), pour : lignes fines `─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼` (U+2500, 2502, 250C, 2510, 2514, 2518, 251C, 2524, 252C, 2534, 253C), blocs `█ ▀ ▄ ▌ ▐` (U+2588, 2580, 2584, 258C, 2590), trames `░ ▒ ▓` (U+2591–2593, alpha 64/128/192), powerline U+E0B0 (triangle plein vers la droite), U+E0B1 (chevron droit), U+E0B2 (triangle plein vers la gauche), U+E0B3 (chevron gauche), U+E0B4 (demi-disque plein à droite), U+E0B5 (demi-disque creux à droite), U+E0B6 (demi-disque plein à gauche), U+E0B7 (demi-disque creux à gauche). `None` pour tout autre caractère. `pub fn is_builtin(ch: char) -> bool`.

- [ ] **Step 1: Écrire les tests**

`crates/rustty-render/src/builtin.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> CellMetrics {
        CellMetrics { width: 10, height: 20, baseline: 16, underline_y: 18, underline_thickness: 1, strike_y: 10 }
    }

    fn alpha(b: &GlyphBitmap, x: u32, y: u32) -> u8 {
        b.data[((y * b.width + x) * 4 + 3) as usize]
    }

    fn glyph(ch: char) -> GlyphBitmap {
        builtin_glyph(ch, metrics()).unwrap_or_else(|| panic!("{ch:?} n'est pas procédural"))
    }

    #[test]
    fn bitmap_fills_the_cell_and_sits_at_the_cell_top() {
        let b = glyph('█');
        assert_eq!((b.width, b.height, b.left, b.top), (10, 20, 0, 16));
        assert!(!b.is_color);
        assert!(b.data.chunks(4).all(|px| px == [255, 255, 255, 255]));
    }

    #[test]
    fn horizontal_and_vertical_lines_cross_the_whole_cell() {
        let h = glyph('─');
        assert!((0..10).all(|x| alpha(&h, x, 10) == 255), "ligne au milieu, de bord à bord");
        assert_eq!(alpha(&h, 5, 2), 0);
        let v = glyph('│');
        assert!((0..20).all(|y| alpha(&v, 5, y) == 255));
        assert_eq!(alpha(&v, 1, 10), 0);
    }

    #[test]
    fn corners_and_tees_only_draw_their_arms() {
        let c = glyph('┌');
        assert_eq!(alpha(&c, 0, 10), 0, "pas de bras à gauche");
        assert_eq!(alpha(&c, 9, 10), 255, "bras à droite");
        assert_eq!(alpha(&c, 5, 0), 0, "pas de bras en haut");
        assert_eq!(alpha(&c, 5, 19), 255, "bras en bas");
        let t = glyph('├');
        assert_eq!(alpha(&t, 5, 0), 255);
        assert_eq!(alpha(&t, 5, 19), 255);
        assert_eq!(alpha(&t, 9, 10), 255);
        assert_eq!(alpha(&t, 0, 10), 0);
        let x = glyph('┼');
        assert!(alpha(&x, 0, 10) == 255 && alpha(&x, 9, 10) == 255 && alpha(&x, 5, 0) == 255 && alpha(&x, 5, 19) == 255);
    }

    #[test]
    fn half_blocks_and_shades() {
        let top = glyph('▀');
        assert_eq!((alpha(&top, 5, 2), alpha(&top, 5, 17)), (255, 0));
        let left = glyph('▌');
        assert_eq!((alpha(&left, 1, 10), alpha(&left, 8, 10)), (255, 0));
        assert!(glyph('░').data.chunks(4).all(|px| px[3] == 64));
        assert!(glyph('▓').data.chunks(4).all(|px| px[3] == 192));
    }

    #[test]
    fn powerline_shapes() {
        let tri = glyph('\u{E0B0}');
        assert_eq!(alpha(&tri, 0, 0), 255, "base pleine à gauche");
        assert_eq!(alpha(&tri, 9, 0), 0, "coin haut droit vide");
        assert_eq!(alpha(&tri, 9, 10), 255, "pointe au milieu à droite");
        let tri_l = glyph('\u{E0B2}');
        assert_eq!(alpha(&tri_l, 9, 0), 255);
        assert_eq!(alpha(&tri_l, 0, 0), 0);
        let disc = glyph('\u{E0B4}');
        assert_eq!(alpha(&disc, 0, 10), 255, "centre du bord plat");
        assert_eq!(alpha(&disc, 9, 0), 0, "coin vide");
        assert_eq!(alpha(&disc, 0, 0), 255, "le bord plat est plein de haut en bas");
        let ring = glyph('\u{E0B5}');
        assert_eq!(alpha(&ring, 3, 10), 0, "creux à l'intérieur");
        assert!(ring.data.chunks(4).any(|px| px[3] == 255), "contour présent");
        let chevron = glyph('\u{E0B1}');
        assert!(chevron.data.chunks(4).any(|px| px[3] == 255));
        assert_eq!(alpha(&chevron, 0, 10), 0, "le chevron ne remplit pas");
    }

    #[test]
    fn unknown_characters_are_not_builtin() {
        assert!(builtin_glyph('A', metrics()).is_none());
        assert!(!is_builtin('A'));
        assert!(is_builtin('┼') && is_builtin('\u{E0B6}'));
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render`
Expected: module `builtin` introuvable.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/src/builtin.rs` :

```rust
//! Glyphes dessinés par le renderer lui-même, indépendamment de la police :
//! lignes de boîte, blocs et symboles powerline, pour des bordures et une
//! barre d'onglets nettes même sans police patchée.

use crate::font::{CellMetrics, GlyphBitmap};

pub fn is_builtin(ch: char) -> bool {
    matches!(ch, '\u{2500}' | '\u{2502}' | '\u{250C}' | '\u{2510}' | '\u{2514}' | '\u{2518}' | '\u{251C}' | '\u{2524}' | '\u{252C}' | '\u{2534}' | '\u{253C}')
        || matches!(ch, '\u{2588}' | '\u{2580}' | '\u{2584}' | '\u{258C}' | '\u{2590}' | '\u{2591}' | '\u{2592}' | '\u{2593}')
        || ('\u{E0B0}'..='\u{E0B7}').contains(&ch)
}

/// Canevas alpha de la taille d'une cellule.
struct Canvas {
    w: u32,
    h: u32,
    alpha: Vec<u8>,
}

impl Canvas {
    fn new(w: u32, h: u32) -> Self {
        Self { w, h, alpha: vec![0; (w * h) as usize] }
    }

    fn fill_rect(&mut self, x0: u32, y0: u32, x1: u32, y1: u32) {
        for y in y0.min(self.h)..y1.min(self.h) {
            for x in x0.min(self.w)..x1.min(self.w) {
                self.alpha[(y * self.w + x) as usize] = 255;
            }
        }
    }

    /// Remplit chaque pixel dont le centre satisfait `inside`.
    fn fill_where(&mut self, value: u8, inside: impl Fn(f32, f32) -> bool) {
        for y in 0..self.h {
            for x in 0..self.w {
                if inside(x as f32 + 0.5, y as f32 + 0.5) {
                    self.alpha[(y * self.w + x) as usize] = value;
                }
            }
        }
    }

    fn into_bitmap(self, baseline: u32) -> GlyphBitmap {
        GlyphBitmap {
            width: self.w,
            height: self.h,
            left: 0,
            top: baseline as i32,
            data: self.alpha.iter().flat_map(|&a| [255, 255, 255, a]).collect(),
            is_color: false,
        }
    }
}

/// Bras d'une ligne de boîte, depuis le centre de la cellule.
#[derive(Clone, Copy)]
struct Arms {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

fn box_arms(ch: char) -> Option<Arms> {
    let a = |left, right, up, down| Some(Arms { left, right, up, down });
    match ch {
        '\u{2500}' => a(true, true, false, false),
        '\u{2502}' => a(false, false, true, true),
        '\u{250C}' => a(false, true, false, true),
        '\u{2510}' => a(true, false, false, true),
        '\u{2514}' => a(false, true, true, false),
        '\u{2518}' => a(true, false, true, false),
        '\u{251C}' => a(false, true, true, true),
        '\u{2524}' => a(true, false, true, true),
        '\u{252C}' => a(true, true, false, true),
        '\u{2534}' => a(true, true, true, false),
        '\u{253C}' => a(true, true, true, true),
        _ => None,
    }
}

pub fn builtin_glyph(ch: char, metrics: CellMetrics) -> Option<GlyphBitmap> {
    let (w, h) = (metrics.width.max(1), metrics.height.max(1));
    let mut c = Canvas::new(w, h);
    let t = (w as f32 / 8.0).round().max(1.0) as u32;
    let (cx, cy) = (w / 2, h / 2);
    let (x0, y0) = (cx.saturating_sub(t / 2), cy.saturating_sub(t / 2));
    if let Some(arms) = box_arms(ch) {
        if arms.left {
            c.fill_rect(0, y0, cx + t.div_ceil(2), y0 + t);
        }
        if arms.right {
            c.fill_rect(x0, y0, w, y0 + t);
        }
        if arms.up {
            c.fill_rect(x0, 0, x0 + t, cy + t.div_ceil(2));
        }
        if arms.down {
            c.fill_rect(x0, y0, x0 + t, h);
        }
        return Some(c.into_bitmap(metrics.baseline));
    }
    let (wf, hf) = (w as f32, h as f32);
    let (cxf, cyf) = (wf / 2.0, hf / 2.0);
    // Demi-disque : ellipse de largeur w et de hauteur h, centrée sur le bord plat.
    let ellipse = |x: f32, y: f32, center_x: f32| ((x - center_x) / wf).powi(2) + ((y - cyf) / cyf).powi(2);
    let line_px = t as f32;
    match ch {
        '\u{2588}' => c.fill_rect(0, 0, w, h),
        '\u{2580}' => c.fill_rect(0, 0, w, cy),
        '\u{2584}' => c.fill_rect(0, cy, w, h),
        '\u{258C}' => c.fill_rect(0, 0, cx, h),
        '\u{2590}' => c.fill_rect(cx, 0, w, h),
        '\u{2591}' => c.fill_where(64, |_, _| true),
        '\u{2592}' => c.fill_where(128, |_, _| true),
        '\u{2593}' => c.fill_where(192, |_, _| true),
        // Triangle plein vers la droite : base à gauche, pointe au milieu à droite.
        '\u{E0B0}' => c.fill_where(255, |x, y| x / wf <= 1.0 - (y - cyf).abs() / cyf),
        '\u{E0B2}' => c.fill_where(255, |x, y| (wf - x) / wf <= 1.0 - (y - cyf).abs() / cyf),
        // Chevrons : bande fine le long des deux arêtes du triangle.
        '\u{E0B1}' => c.fill_where(255, |x, y| ((x / wf) - (1.0 - (y - cyf).abs() / cyf)).abs() * wf <= line_px),
        '\u{E0B3}' => c.fill_where(255, |x, y| (((wf - x) / wf) - (1.0 - (y - cyf).abs() / cyf)).abs() * wf <= line_px),
        '\u{E0B4}' => c.fill_where(255, |x, y| ellipse(x, y, 0.0) <= 1.0),
        '\u{E0B6}' => c.fill_where(255, |x, y| ellipse(x, y, wf) <= 1.0),
        '\u{E0B5}' => {
            let inner = ((wf - line_px) / wf).powi(2);
            c.fill_where(255, |x, y| {
                let d = ellipse(x, y, 0.0);
                d <= 1.0 && d >= inner
            });
        }
        '\u{E0B7}' => {
            let inner = ((wf - line_px) / wf).powi(2);
            c.fill_where(255, |x, y| {
                let d = ellipse(x, y, wf);
                d <= 1.0 && d >= inner
            });
        }
        _ => return None,
    }
    let _ = cxf;
    Some(c.into_bitmap(metrics.baseline))
}
```

Retirer `let _ = cxf;` si `cxf` est utilisé ailleurs ou supprimer la variable si clippy la juge inutile. Vérifier à la main les assertions des tests avec `w = 10`, `h = 20`, `t = 1`, `cx = 5`, `cy = 10` : `─` remplit `y ∈ [10, 11)` sur `x ∈ [0, 10)` ; `┌` dessine `x ∈ [5, 10)` sur la ligne 10 et `y ∈ [10, 20)` sur la colonne 5, donc `(0, 10)` et `(5, 0)` restent vides. Pour U+E0B4, `(0.5, 10.5)` donne `d ≈ 0.0025 ≤ 1` et `(9.5, 0.5)` donne `0.9025 + 0.9025 > 1`. Pour U+E0B5, `inner = (9/10)² = 0.81` ; `(3.5, 10.5)` : `d = 0.1225 < 0.81`, donc creux. Pour U+E0B0, `(9.5, 10.5)` : `0.95 ≤ 1 − 0.05 = 0.95` ✓ ; `(9.5, 0.5)` : `0.95 ≤ 1 − 0.95 = 0.05` ✗ ; `(0.5, 0.5)` : `0.05 ≤ 0.05` ✓.

`lib.rs` : `pub mod builtin;` et `pub use builtin::{builtin_glyph, is_builtin};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. En cas d'échec d'une assertion de forme, ajuster la géométrie, **pas** le test, sauf s'il contredit la description du glyphe ci-dessus.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.42"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.42 — 2026-10-08 · « Glyphes procéduraux »

- `rustty-render` : lignes de boîte, blocs, trames et symboles powerline dessinés par le renderer, nets quelle que soit la police.
```

```bash
git add Cargo.toml crates/rustty-render CHANGELOG.md
git commit -m "rustty-render : glyphes de boîte, blocs et powerline procéduraux (0.1.0-alpha.42)"
```

---

### Task 8 : Atlas de glyphes — packer CPU

**Files:**
- Create: `crates/rustty-render/src/atlas.rs`
- Modify: `crates/rustty-render/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct AtlasRegion { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }` ; `AtlasRegion::uv(&self, atlas_size: u32) -> [f32; 4]` (`u0, v0, u1, v1` dans [0, 1]).
  - `pub struct AtlasPacker` ; `AtlasPacker::new(size: u32)` ; `size()` ; `insert(&mut self, width, height) -> Option<AtlasRegion>` (placement par étagères, 1 px de marge entre régions, `None` si plein ou si `width`/`height` vaut 0) ; `clear(&mut self)` ; `len() -> usize` (régions placées).
  - `pub const DEFAULT_ATLAS_SIZE: u32 = 2048;`

- [ ] **Step 1: Écrire les tests**

`crates/rustty-render/src/atlas.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_do_not_overlap_and_stay_inside() {
        let mut p = AtlasPacker::new(64);
        let mut regions = Vec::new();
        for (w, h) in [(10, 12), (20, 12), (30, 8), (5, 30), (40, 10), (10, 10)] {
            let r = p.insert(w, h).unwrap();
            assert_eq!((r.width, r.height), (w, h));
            assert!(r.x + r.width <= 64 && r.y + r.height <= 64, "{r:?}");
            regions.push(r);
        }
        for (i, a) in regions.iter().enumerate() {
            for b in &regions[i + 1..] {
                let overlap = a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
                assert!(!overlap, "{a:?} chevauche {b:?}");
            }
        }
        assert_eq!(p.len(), 6);
    }

    #[test]
    fn same_height_glyphs_share_a_shelf() {
        let mut p = AtlasPacker::new(100);
        let a = p.insert(10, 10).unwrap();
        let b = p.insert(10, 10).unwrap();
        assert_eq!(a.y, b.y, "même étagère");
        assert_eq!(b.x, a.x + a.width + 1, "une marge d'un pixel");
    }

    #[test]
    fn full_atlas_returns_none_until_cleared() {
        let mut p = AtlasPacker::new(16);
        assert!(p.insert(15, 15).is_some());
        assert!(p.insert(2, 2).is_none(), "plus de place");
        p.clear();
        assert_eq!(p.len(), 0);
        assert!(p.insert(15, 15).is_some());
    }

    #[test]
    fn oversized_or_empty_requests_are_rejected() {
        let mut p = AtlasPacker::new(32);
        assert!(p.insert(40, 4).is_none());
        assert!(p.insert(4, 40).is_none());
        assert!(p.insert(0, 4).is_none());
        assert!(p.insert(4, 0).is_none());
        assert_eq!(p.len(), 0);
    }

    #[test]
    fn uv_maps_pixels_to_unit_square() {
        let r = AtlasRegion { x: 16, y: 32, width: 16, height: 32 };
        assert_eq!(r.uv(64), [0.25, 0.5, 0.5, 1.0]);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render`
Expected: module `atlas` introuvable.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/src/atlas.rs` :

```rust
//! Placement des bitmaps de glyphes dans une texture carrée, par étagères :
//! simple, rapide, et suffisant pour des glyphes de hauteurs proches.

pub const DEFAULT_ATLAS_SIZE: u32 = 2048;

/// Marge entre deux régions, pour que l'échantillonnage ne bave pas.
const PADDING: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl AtlasRegion {
    /// Coordonnées de texture `[u0, v0, u1, v1]`.
    pub fn uv(&self, atlas_size: u32) -> [f32; 4] {
        let s = atlas_size as f32;
        [self.x as f32 / s, self.y as f32 / s, (self.x + self.width) as f32 / s, (self.y + self.height) as f32 / s]
    }
}

#[derive(Clone, Debug)]
struct Shelf {
    y: u32,
    height: u32,
    next_x: u32,
}

#[derive(Clone, Debug)]
pub struct AtlasPacker {
    size: u32,
    shelves: Vec<Shelf>,
    next_y: u32,
    count: usize,
}

impl AtlasPacker {
    pub fn new(size: u32) -> Self {
        Self { size, shelves: Vec::new(), next_y: 0, count: 0 }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn clear(&mut self) {
        self.shelves.clear();
        self.next_y = 0;
        self.count = 0;
    }

    /// Place une région de `width × height`. Une étagère existante est
    /// réutilisée si le glyphe y tient sans gaspiller plus de la moitié de sa
    /// hauteur ; sinon une nouvelle étagère est ouverte.
    pub fn insert(&mut self, width: u32, height: u32) -> Option<AtlasRegion> {
        if width == 0 || height == 0 || width > self.size || height > self.size {
            return None;
        }
        let padded_w = width + PADDING;
        if let Some(shelf) = self.shelves.iter_mut().find(|s| height <= s.height && height * 2 >= s.height && s.next_x + padded_w <= self.size + PADDING) {
            let region = AtlasRegion { x: shelf.next_x, y: shelf.y, width, height };
            shelf.next_x += padded_w;
            self.count += 1;
            return Some(region);
        }
        let shelf_height = height + PADDING;
        if self.next_y + height > self.size {
            return None;
        }
        let region = AtlasRegion { x: 0, y: self.next_y, width, height };
        self.shelves.push(Shelf { y: self.next_y, height, next_x: padded_w });
        self.next_y += shelf_height;
        self.count += 1;
        Some(region)
    }
}
```

Vérification des tests à la main : atlas 16, `insert(15, 15)` ouvre une étagère `y = 0`, `next_x = 16`, `next_y = 16` ; `insert(2, 2)` : aucune étagère ne convient (`2 × 2 < 15`) et `16 + 2 > 16` → `None` ✓. Deux `insert(10, 10)` : second placé sur la même étagère en `x = 11` ✓.

`lib.rs` : `pub mod atlas;` et `pub use atlas::{AtlasPacker, AtlasRegion, DEFAULT_ATLAS_SIZE};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.43"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.43 — 2026-10-08 · « Atlas de glyphes »

- `rustty-render` : `AtlasPacker` place les bitmaps par étagères avec une marge d'un pixel ; coordonnées de texture normalisées.
```

```bash
git add Cargo.toml crates/rustty-render CHANGELOG.md
git commit -m "rustty-render : packer d'atlas par étagères (0.1.0-alpha.43)"
```

---

### Task 9 : Contexte GPU sans fenêtre, cible hors écran, relecture, CI avec lavapipe

**Files:**
- Create: `crates/rustty-render/src/gpu.rs`
- Create: `crates/rustty-render/tests/common/mod.rs`
- Create: `crates/rustty-render/tests/gpu.rs`
- Modify: `crates/rustty-render/src/lib.rs`
- Modify: `crates/rustty-render/Cargo.toml` (`wgpu`, `pollster`, `bytemuck`)
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces:
  - `pub enum GpuError { NoAdapter(String), Device(String), Readback(String) }`.
  - `pub struct GpuContext { pub instance: wgpu::Instance, pub adapter: wgpu::Adapter, pub device: wgpu::Device, pub queue: wgpu::Queue }` ; `GpuContext::headless() -> Result<GpuContext, GpuError>` (tous les backends ; adaptateur ordinaire, sinon adaptateur de repli logiciel) ; `GpuContext::with_adapter(instance, adapter) -> Result<GpuContext, GpuError>` (le binaire l'appellera avec un adaptateur compatible avec sa surface).
  - `pub const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;`
  - `pub struct Offscreen` ; `Offscreen::new(ctx, width, height)` (0 ⇒ 1) ; `view() -> &wgpu::TextureView` ; `size() -> (u32, u32)` ; `read_rgba(&self, ctx) -> Result<Vec<u8>, GpuError>` (`width × height × 4`, lignes dépaddées).
  - `pub fn clear(ctx: &GpuContext, view: &wgpu::TextureView, color: Rgba)` : une passe qui efface.
  - `tests/common/mod.rs` : `pub fn gpu_or_skip() -> Option<GpuContext>` (écrit `test GPU ignoré : …` sur stderr et rend `None` sans adaptateur).

- [ ] **Step 1: Écrire les tests**

`crates/rustty-render/tests/common/mod.rs` :

```rust
//! Aide partagée des tests GPU : un contexte sans fenêtre, ou l'abandon
//! silencieux du test quand la machine n'a aucun adaptateur.

use rustty_render::GpuContext;

pub fn gpu_or_skip() -> Option<GpuContext> {
    match GpuContext::headless() {
        Ok(ctx) => Some(ctx),
        Err(e) => {
            eprintln!("test GPU ignoré : {e}");
            None
        }
    }
}
```

`crates/rustty-render/tests/gpu.rs` :

```rust
mod common;

use rustty_render::{Offscreen, Rgba, clear};

#[test]
fn clear_and_read_back() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let target = Offscreen::new(&ctx, 8, 4);
    assert_eq!(target.size(), (8, 4));
    clear(&ctx, target.view(), Rgba::new(1.0, 0.0, 0.0, 1.0));
    let px = target.read_rgba(&ctx).unwrap();
    assert_eq!(px.len(), 8 * 4 * 4);
    assert!(px.chunks(4).all(|p| p == [255, 0, 0, 255]), "{:?}", &px[..16]);
}

#[test]
fn readback_handles_row_padding() {
    // 3 px de large : 12 octets par ligne, bien en dessous de l'alignement de 256.
    let Some(ctx) = common::gpu_or_skip() else { return };
    let target = Offscreen::new(&ctx, 3, 2);
    clear(&ctx, target.view(), Rgba::new(0.0, 1.0, 0.0, 1.0));
    let px = target.read_rgba(&ctx).unwrap();
    assert_eq!(px.len(), 3 * 2 * 4);
    assert!(px.chunks(4).all(|p| p == [0, 255, 0, 255]));
}

#[test]
fn zero_size_is_clamped_to_one_pixel() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let target = Offscreen::new(&ctx, 0, 0);
    assert_eq!(target.size(), (1, 1));
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 1.0, 0.5));
    let px = target.read_rgba(&ctx).unwrap();
    assert_eq!(px, vec![0, 0, 255, 128]);
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render --test gpu`
Expected: `unresolved imports rustty_render::{GpuContext, Offscreen, clear}`.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/Cargo.toml`, `[dependencies]` : ajouter `wgpu.workspace = true`, `pollster.workspace = true`, `bytemuck.workspace = true`.

`crates/rustty-render/src/gpu.rs` :

```rust
//! Accès au GPU : contexte sans fenêtre pour les tests et le binaire (qui
//! fournit son propre adaptateur), cible hors écran et relecture des pixels.

use crate::color::Rgba;

#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    #[error("aucun adaptateur graphique disponible : {0}")]
    NoAdapter(String),
    #[error("impossible de créer le périphérique graphique : {0}")]
    Device(String),
    #[error("relecture des pixels impossible : {0}")]
    Readback(String),
}

pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    /// Un adaptateur sans surface : le vrai GPU s'il y en a un, sinon le
    /// rendu logiciel (lavapipe, WARP).
    pub fn headless() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::all(), ..Default::default() });
        let hardware = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: None,
        };
        let software = wgpu::RequestAdapterOptions { force_fallback_adapter: true, ..hardware.clone() };
        let adapter = match pollster::block_on(instance.request_adapter(&hardware)) {
            Ok(a) => a,
            Err(first) => pollster::block_on(instance.request_adapter(&software))
                .map_err(|second| GpuError::NoAdapter(format!("{first} ; repli logiciel : {second}")))?,
        };
        Self::with_adapter(instance, adapter)
    }

    pub fn with_adapter(instance: wgpu::Instance, adapter: wgpu::Adapter) -> Result<Self, GpuError> {
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("rustty"),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).map_err(|e| GpuError::Device(e.to_string()))?;
        Ok(Self { instance, adapter, device, queue })
    }
}

/// Format des cibles hors écran : non-sRGB, pour des pixels relus égaux aux
/// couleurs demandées.
pub const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub struct Offscreen {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl Offscreen {
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rustty-offscreen"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view, width, height }
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Copie la texture dans un tampon lisible et rend les pixels RGBA8,
    /// ligne par ligne, sans le remplissage d'alignement.
    pub fn read_rgba(&self, ctx: &GpuContext) -> Result<Vec<u8>, GpuError> {
        let unpadded = self.width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = unpadded.div_ceil(align) * align;
        let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rustty-readback"),
            size: u64::from(padded) * u64::from(self.height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("rustty-readback") });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &self.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: None },
            },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
        ctx.queue.submit(Some(encoder.finish()));
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        ctx.device.poll(wgpu::PollType::Wait).map_err(|e| GpuError::Readback(e.to_string()))?;
        rx.recv().map_err(|e| GpuError::Readback(e.to_string()))?.map_err(|e| GpuError::Readback(e.to_string()))?;
        let data = slice.get_mapped_range();
        let mut out = Vec::with_capacity((unpadded * self.height) as usize);
        for row in data.chunks(padded as usize) {
            out.extend_from_slice(&row[..unpadded as usize]);
        }
        drop(data);
        buffer.unmap();
        Ok(out)
    }
}

/// Une passe de rendu qui ne fait qu'effacer `view` avec `color`.
pub fn clear(ctx: &GpuContext, view: &wgpu::TextureView, color: Rgba) {
    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("rustty-clear") });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rustty-clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(to_wgpu_color(color)), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }
    ctx.queue.submit(Some(encoder.finish()));
}

pub(crate) fn to_wgpu_color(c: Rgba) -> wgpu::Color {
    wgpu::Color { r: f64::from(c.r), g: f64::from(c.g), b: f64::from(c.b), a: f64::from(c.a) }
}
```

Adaptations possibles contre `wgpu` 30 : `Instance::new` prend `InstanceDescriptor` par valeur (doc 30.0.1) ; `PollType::Wait` peut s'écrire `wgpu::PollType::wait()` dans certaines versions ; `DeviceDescriptor` implémente `Default` ; `RequestAdapterOptions` est `Clone`. Si `Limits::using_resolution` n'existe plus, utiliser `adapter.limits()` directement.

`lib.rs` : `pub mod gpu;` et `pub use gpu::{GpuContext, GpuError, OFFSCREEN_FORMAT, Offscreen, clear};`.

`.github/workflows/ci.yml`, ajouter après `rust-cache` :

```yaml
      - name: Rendu logiciel Vulkan pour les tests GPU
        if: runner.os == 'Linux'
        run: sudo apt-get update && sudo apt-get install -y mesa-vulkan-drivers libvulkan1
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok` ; sur cette machine (Intel, Mesa) les trois tests GPU s'exécutent réellement : vérifier qu'aucun n'affiche `test GPU ignoré` (`cargo test -p rustty-render --test gpu -- --nocapture`). Alpha 0,5 relu vaut 128 (`0.5 × 255 = 127.5` arrondi à 128 par le GPU ; si un pilote rend 127, remplacer l'assertion par `px[3] == 127 || px[3] == 128` et le consigner).

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.44"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.44 — 2026-10-08 · « Contexte GPU hors écran »

- `rustty-render` : `GpuContext::headless` (matériel ou rendu logiciel), cible hors écran `Offscreen` avec relecture des pixels, passe d'effacement. CI Linux équipée de lavapipe pour exécuter les tests GPU.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-render .github/workflows/ci.yml CHANGELOG.md
git commit -m "rustty-render : contexte GPU sans fenêtre, cible hors écran et relecture (0.1.0-alpha.44)"
```

---

### Task 10 : Pipeline de quads colorés

**Files:**
- Create: `crates/rustty-render/src/pipeline/mod.rs`
- Create: `crates/rustty-render/src/pipeline/quad.rs`
- Create: `crates/rustty-render/src/shaders/quad.wgsl`
- Modify: `crates/rustty-render/src/lib.rs`
- Modify: `crates/rustty-render/tests/gpu.rs`

**Interfaces:**
- Produces:
  - `#[repr(C)] pub struct QuadInstance { pub pos: [f32; 2], pub size: [f32; 2], pub color: [f32; 4] }` (`Pod`, `Zeroable`) ; `QuadInstance::new(x: f32, y: f32, w: f32, h: f32, color: Rgba)`.
  - `pub struct QuadPipeline` ; `QuadPipeline::new(device: &wgpu::Device, format: wgpu::TextureFormat)` ; `prepare(&self, device, instances: &[QuadInstance], viewport: (u32, u32)) -> Option<QuadBatch>` (`None` si vide) ; `draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, batch: &'a QuadBatch)`.
  - `pub struct QuadBatch` (tampons + bind group, `len()`).
  - Shader : viewport en pixels passé en uniforme, origine en haut à gauche, 6 sommets par instance, mélange alpha.

- [ ] **Step 1: Écrire les tests**

Ajouter dans `crates/rustty-render/tests/gpu.rs` :

```rust
use rustty_render::{QuadInstance, QuadPipeline};

fn pixel(px: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn quads_are_drawn_in_pixel_coordinates_with_alpha_blending() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let target = Offscreen::new(&ctx, 8, 4);
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 0.0, 1.0));
    let pipeline = QuadPipeline::new(&ctx.device, rustty_render::OFFSCREEN_FORMAT);
    let instances = [
        QuadInstance::new(2.0, 1.0, 4.0, 2.0, Rgba::new(1.0, 0.0, 0.0, 1.0)),
        QuadInstance::new(0.0, 0.0, 2.0, 1.0, Rgba::new(0.0, 0.0, 1.0, 0.5)),
    ];
    let batch = pipeline.prepare(&ctx.device, &instances, target.size()).unwrap();
    assert_eq!(batch.len(), 2);
    let mut encoder = ctx.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pipeline.draw(&mut pass, &batch);
    }
    ctx.queue.submit(Some(encoder.finish()));
    let px = target.read_rgba(&ctx).unwrap();
    assert_eq!(pixel(&px, 8, 2, 1), [255, 0, 0, 255], "coin haut-gauche du quad rouge");
    assert_eq!(pixel(&px, 8, 5, 2), [255, 0, 0, 255], "coin bas-droit inclus");
    assert_eq!(pixel(&px, 8, 6, 1), [0, 0, 0, 255], "hors du quad");
    assert_eq!(pixel(&px, 8, 1, 3), [0, 0, 0, 255]);
    let blended = pixel(&px, 8, 0, 0);
    assert!(blended[2] >= 126 && blended[2] <= 129 && blended[0] == 0, "bleu à 50 % sur noir : {blended:?}");
}

#[test]
fn empty_batch_is_none() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let pipeline = QuadPipeline::new(&ctx.device, rustty_render::OFFSCREEN_FORMAT);
    assert!(pipeline.prepare(&ctx.device, &[], (8, 8)).is_none());
}
```

Ajouter `wgpu` en `[dev-dependencies]` de la crate (`wgpu.workspace = true`) pour que le test construise sa passe.

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render --test gpu`
Expected: `unresolved imports … QuadInstance, QuadPipeline`.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/src/shaders/quad.wgsl` :

```wgsl
// Rectangles colorés instanciés : fonds de cellule, curseur, bordures, barre
// d'onglets. Coordonnées en pixels, origine en haut à gauche.

struct Globals {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;

struct Instance {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn corner(vi: u32) -> vec2<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    return corners[vi];
}

fn to_clip(px: vec2<f32>) -> vec4<f32> {
    let ndc = vec2<f32>(px.x / globals.viewport.x * 2.0 - 1.0, 1.0 - px.y / globals.viewport.y * 2.0);
    return vec4<f32>(ndc, 0.0, 1.0);
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    var out: VsOut;
    out.clip = to_clip(inst.pos + corner(vi) * inst.size);
    out.color = inst.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
```

`crates/rustty-render/src/pipeline/mod.rs` :

```rust
//! Les deux pipelines wgpu : quads colorés et quads texturés par l'atlas.

pub mod quad;

use bytemuck::{Pod, Zeroable};

/// Uniforme partagé : la taille du viewport en pixels.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct Globals {
    pub viewport: [f32; 2],
    pub _pad: [f32; 2],
}

impl Globals {
    pub fn new(viewport: (u32, u32)) -> Self {
        Self { viewport: [viewport.0.max(1) as f32, viewport.1.max(1) as f32], _pad: [0.0; 2] }
    }
}

pub(crate) fn globals_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("rustty-globals"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        }],
    })
}

pub(crate) fn alpha_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL }
}
```

`crates/rustty-render/src/pipeline/quad.rs` :

```rust
//! Rectangles pleins instanciés.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

use super::{Globals, alpha_target, globals_layout};
use crate::color::Rgba;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct QuadInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
}

impl QuadInstance {
    pub fn new(x: f32, y: f32, w: f32, h: f32, color: Rgba) -> Self {
        Self { pos: [x, y], size: [w, h], color: color.to_array() }
    }
}

pub struct QuadPipeline {
    pipeline: wgpu::RenderPipeline,
    globals_layout: wgpu::BindGroupLayout,
}

pub struct QuadBatch {
    instances: wgpu::Buffer,
    count: u32,
    bind_group: wgpu::BindGroup,
    _globals: wgpu::Buffer,
}

impl QuadBatch {
    pub fn len(&self) -> usize {
        self.count as usize
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl QuadPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rustty-quad"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/quad.wgsl").into()),
        });
        let globals_layout = globals_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rustty-quad"),
            bind_group_layouts: &[&globals_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rustty-quad"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<QuadInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(alpha_target(format))],
            }),
            multiview: None,
            cache: None,
        });
        Self { pipeline, globals_layout }
    }

    pub fn prepare(&self, device: &wgpu::Device, instances: &[QuadInstance], viewport: (u32, u32)) -> Option<QuadBatch> {
        if instances.is_empty() {
            return None;
        }
        let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-quad-instances"),
            contents: bytemuck::cast_slice(instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-quad-globals"),
            contents: bytemuck::bytes_of(&Globals::new(viewport)),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustty-quad-globals"),
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        Some(QuadBatch { instances: instances_buffer, count: instances.len() as u32, bind_group, _globals: globals })
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, batch: &'a QuadBatch) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &batch.bind_group, &[]);
        pass.set_vertex_buffer(0, batch.instances.slice(..));
        pass.draw(0..6, 0..batch.count);
    }
}
```

Adaptations possibles : si `RenderPipelineDescriptor` n'a plus de champ `multiview` ou `cache`, les retirer ; si `entry_point` attend `&str` et non `Option`, retirer `Some`. Recréer les tampons à chaque `prepare` est volontairement simple (une image par frame) ; l'optimisation (tampons persistants) est hors v0.1.

`lib.rs` : `pub mod pipeline;` et `pub use pipeline::quad::{QuadBatch, QuadInstance, QuadPipeline};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`, tests GPU exécutés (pas de `ignoré` sur cette machine).

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.45"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.45 — 2026-10-08 · « Pipeline de quads »

- `rustty-render` : rectangles colorés instanciés en coordonnées pixels avec mélange alpha, vérifiés par relecture.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-render CHANGELOG.md
git commit -m "rustty-render : pipeline de quads colorés (0.1.0-alpha.45)"
```

---

### Task 11 : Pipeline de glyphes et texture d'atlas

**Files:**
- Create: `crates/rustty-render/src/pipeline/glyph.rs`
- Create: `crates/rustty-render/src/shaders/glyph.wgsl`
- Modify: `crates/rustty-render/src/pipeline/mod.rs`
- Modify: `crates/rustty-render/src/lib.rs`
- Modify: `crates/rustty-render/tests/gpu.rs`

**Interfaces:**
- Produces:
  - `#[repr(C)] pub struct GlyphInstance { pub pos: [f32; 2], pub size: [f32; 2], pub uv: [f32; 4], pub color: [f32; 4], pub flags: u32, pub _pad: [u32; 3] }` ; `GlyphInstance::new(x, y, w, h, uv: [f32; 4], color: Rgba, is_color: bool)` (`flags` bit 0 = glyphe couleur : la texture fournit RGB, la couleur d'instance n'apporte que l'alpha).
  - `pub struct AtlasTexture` ; `AtlasTexture::new(device, size: u32)` (`Rgba8Unorm`, `TEXTURE_BINDING | COPY_DST`) ; `size()` ; `upload(&self, queue, region: AtlasRegion, rgba: &[u8])` ; `clear(&self, queue)` (remplit de zéros).
  - `pub struct GlyphPipeline` ; `new(device, format)` ; `prepare(&self, device, atlas: &AtlasTexture, instances: &[GlyphInstance], viewport) -> Option<GlyphBatch>` ; `draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, batch: &'a GlyphBatch)`.
  - Échantillonnage `Nearest`, `ClampToEdge`.

- [ ] **Step 1: Écrire les tests**

Ajouter dans `crates/rustty-render/tests/gpu.rs` :

```rust
use rustty_render::{AtlasRegion, AtlasTexture, GlyphInstance, GlyphPipeline};

fn draw_glyphs(ctx: &rustty_render::GpuContext, target: &Offscreen, atlas: &AtlasTexture, instances: &[GlyphInstance]) -> Vec<u8> {
    let pipeline = GlyphPipeline::new(&ctx.device, rustty_render::OFFSCREEN_FORMAT);
    let batch = pipeline.prepare(&ctx.device, atlas, instances, target.size()).unwrap();
    let mut encoder = ctx.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pipeline.draw(&mut pass, &batch);
    }
    ctx.queue.submit(Some(encoder.finish()));
    target.read_rgba(ctx).unwrap()
}

#[test]
fn monochrome_glyph_takes_the_instance_color() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let target = Offscreen::new(&ctx, 6, 6);
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 0.0, 1.0));
    let atlas = AtlasTexture::new(&ctx.device, 8);
    let region = AtlasRegion { x: 0, y: 0, width: 2, height: 2 };
    // Masque : plein en haut à gauche et en bas à droite, vide ailleurs.
    atlas.upload(&ctx.queue, region, &[255, 255, 255, 255, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255, 255, 255]);
    let px = draw_glyphs(&ctx, &target, &atlas, &[GlyphInstance::new(1.0, 2.0, 2.0, 2.0, region.uv(8), Rgba::new(1.0, 0.0, 0.0, 1.0), false)]);
    assert_eq!(pixel(&px, 6, 1, 2), [255, 0, 0, 255], "texel plein → couleur d'instance");
    assert_eq!(pixel(&px, 6, 2, 2), [0, 0, 0, 255], "texel vide → fond inchangé");
    assert_eq!(pixel(&px, 6, 2, 3), [255, 0, 0, 255]);
    assert_eq!(pixel(&px, 6, 0, 0), [0, 0, 0, 255]);
}

#[test]
fn color_glyph_keeps_its_own_colors() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let target = Offscreen::new(&ctx, 4, 4);
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 0.0, 1.0));
    let atlas = AtlasTexture::new(&ctx.device, 4);
    let region = AtlasRegion { x: 1, y: 1, width: 1, height: 1 };
    atlas.upload(&ctx.queue, region, &[0, 255, 0, 255]);
    let px = draw_glyphs(&ctx, &target, &atlas, &[GlyphInstance::new(0.0, 0.0, 1.0, 1.0, region.uv(4), Rgba::new(1.0, 0.0, 0.0, 1.0), true)]);
    assert_eq!(pixel(&px, 4, 0, 0), [0, 255, 0, 255], "la texture l'emporte sur la couleur d'instance");
}

#[test]
fn atlas_clear_wipes_previous_glyphs() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let target = Offscreen::new(&ctx, 2, 2);
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 0.0, 1.0));
    let atlas = AtlasTexture::new(&ctx.device, 2);
    let region = AtlasRegion { x: 0, y: 0, width: 1, height: 1 };
    atlas.upload(&ctx.queue, region, &[255, 255, 255, 255]);
    atlas.clear(&ctx.queue);
    let px = draw_glyphs(&ctx, &target, &atlas, &[GlyphInstance::new(0.0, 0.0, 1.0, 1.0, region.uv(2), Rgba::new(1.0, 1.0, 1.0, 1.0), false)]);
    assert_eq!(pixel(&px, 2, 0, 0), [0, 0, 0, 255]);
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render --test gpu`
Expected: `unresolved imports … AtlasTexture, GlyphInstance, GlyphPipeline`.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/src/shaders/glyph.wgsl` :

```wgsl
// Quads texturés par l'atlas de glyphes. Monochrome : la texture donne la
// couverture (alpha), l'instance la couleur. Couleur (emoji) : la texture
// donne tout, l'instance ne module que l'alpha.

struct Globals {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_samp: sampler;

struct Instance {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec4<f32>,
    @location(3) color: vec4<f32>,
    @location(4) flags: u32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) flags: u32,
};

fn corner(vi: u32) -> vec2<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    return corners[vi];
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    let c = corner(vi);
    let px = inst.pos + c * inst.size;
    let ndc = vec2<f32>(px.x / globals.viewport.x * 2.0 - 1.0, 1.0 - px.y / globals.viewport.y * 2.0);
    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = mix(inst.uv.xy, inst.uv.zw, c);
    out.color = inst.color;
    out.flags = inst.flags;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let t = textureSample(atlas_tex, atlas_samp, in.uv);
    if ((in.flags & 1u) != 0u) {
        return vec4<f32>(t.rgb, t.a * in.color.a);
    }
    return vec4<f32>(in.color.rgb, t.a * in.color.a);
}
```

`crates/rustty-render/src/pipeline/glyph.rs` :

```rust
//! Quads texturés par l'atlas : les glyphes de la grille et du chrome.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

use super::{Globals, alpha_target, globals_layout};
use crate::atlas::AtlasRegion;
use crate::color::Rgba;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct GlyphInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    /// `u0, v0, u1, v1` dans l'atlas.
    pub uv: [f32; 4],
    pub color: [f32; 4],
    /// Bit 0 : glyphe couleur.
    pub flags: u32,
    pub _pad: [u32; 3],
}

impl GlyphInstance {
    pub fn new(x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], color: Rgba, is_color: bool) -> Self {
        Self { pos: [x, y], size: [w, h], uv, color: color.to_array(), flags: u32::from(is_color), _pad: [0; 3] }
    }
}

pub struct AtlasTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: u32,
}

impl AtlasTexture {
    pub fn new(device: &wgpu::Device, size: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rustty-atlas"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view, size }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    /// Écrit `rgba` (`region.width × region.height × 4` octets) dans l'atlas.
    pub fn upload(&self, queue: &wgpu::Queue, region: AtlasRegion, rgba: &[u8]) {
        debug_assert_eq!(rgba.len() as u32, region.width * region.height * 4);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: region.x, y: region.y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(region.width * 4), rows_per_image: None },
            wgpu::Extent3d { width: region.width, height: region.height, depth_or_array_layers: 1 },
        );
    }

    /// Remet tout l'atlas à zéro (après un `AtlasPacker::clear`).
    pub fn clear(&self, queue: &wgpu::Queue) {
        let zeros = vec![0u8; (self.size * self.size * 4) as usize];
        self.upload(queue, AtlasRegion { x: 0, y: 0, width: self.size, height: self.size }, &zeros);
    }
}

pub struct GlyphPipeline {
    pipeline: wgpu::RenderPipeline,
    globals_layout: wgpu::BindGroupLayout,
    atlas_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

pub struct GlyphBatch {
    instances: wgpu::Buffer,
    count: u32,
    globals_group: wgpu::BindGroup,
    atlas_group: wgpu::BindGroup,
    _globals: wgpu::Buffer,
}

impl GlyphBatch {
    pub fn len(&self) -> usize {
        self.count as usize
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl GlyphPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rustty-glyph"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/glyph.wgsl").into()),
        });
        let globals_layout = globals_layout(device);
        let atlas_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rustty-atlas"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rustty-atlas"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rustty-glyph"),
            bind_group_layouts: &[&globals_layout, &atlas_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rustty-glyph"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GlyphInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Uint32],
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(alpha_target(format))],
            }),
            multiview: None,
            cache: None,
        });
        Self { pipeline, globals_layout, atlas_layout, sampler }
    }

    pub fn prepare(&self, device: &wgpu::Device, atlas: &AtlasTexture, instances: &[GlyphInstance], viewport: (u32, u32)) -> Option<GlyphBatch> {
        if instances.is_empty() {
            return None;
        }
        let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-glyph-instances"),
            contents: bytemuck::cast_slice(instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-glyph-globals"),
            contents: bytemuck::bytes_of(&Globals::new(viewport)),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let globals_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustty-glyph-globals"),
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let atlas_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustty-glyph-atlas"),
            layout: &self.atlas_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&atlas.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        Some(GlyphBatch { instances: instances_buffer, count: instances.len() as u32, globals_group, atlas_group, _globals: globals })
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, batch: &'a GlyphBatch) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &batch.globals_group, &[]);
        pass.set_bind_group(1, &batch.atlas_group, &[]);
        pass.set_vertex_buffer(0, batch.instances.slice(..));
        pass.draw(0..6, 0..batch.count);
    }
}
```

`pipeline/mod.rs` : `pub mod glyph;`. `lib.rs` : `pub use pipeline::glyph::{AtlasTexture, GlyphBatch, GlyphInstance, GlyphPipeline};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Le test monochrome dessine le quad en (1, 2) de 2×2 : le texel (0,0) plein colore le pixel (1, 2), le texel (1,0) vide laisse (2, 2) noir, le texel (1,1) plein colore (2, 3).

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.46"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.46 — 2026-10-08 · « Pipeline de glyphes »

- `rustty-render` : texture d'atlas RGBA8 avec envoi par région, quads texturés monochromes (couleur d'instance) ou couleur (emoji), échantillonnage au pixel près.
```

```bash
git add Cargo.toml crates/rustty-render CHANGELOG.md
git commit -m "rustty-render : pipeline de glyphes et texture d'atlas (0.1.0-alpha.46)"
```

---

### Task 12 : `Frame` et géométrie de grille — instances pures

**Files:**
- Create: `crates/rustty-render/src/frame.rs`
- Create: `crates/rustty-render/src/grid.rs`
- Modify: `crates/rustty-render/src/lib.rs`

**Interfaces:**
- Consumes: `rustty_vt::{Snapshot, Cursor, CursorShape, Attrs, Line}`, `Palette`, `CellMetrics`, `Variant`, `QuadInstance`.
- Produces (`frame.rs`) :
  - `pub struct PixelRect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }` (Copy, Default) ; `PixelRect::new`.
  - `pub struct PaneFrame<'a> { pub rect: PixelRect, pub snapshot: &'a Snapshot, pub focused: bool }`.
  - `pub struct ChromeQuad { pub rect: PixelRect, pub color: Rgba }`, `pub struct ChromeText { pub x: u32, pub y: u32, pub text: String, pub color: Rgba }`, `pub struct Chrome { pub quads: Vec<ChromeQuad>, pub texts: Vec<ChromeText> }` (Default).
  - `pub struct Frame<'a> { pub viewport: (u32, u32), pub background: Rgba, pub panes: Vec<PaneFrame<'a>>, pub chrome: Chrome }`.
- Produces (`grid.rs`) :
  - `pub struct GridGeometry { pub origin_x: u32, pub origin_y: u32, pub cols: usize, pub rows: usize }` ; `pub fn grid_geometry(rect: PixelRect, metrics: CellMetrics, padding: u32) -> GridGeometry` (0 colonne/ligne si rien ne tient, jamais de panique).
  - `pub struct GlyphRequest { pub x: f32, pub y: f32, pub ch: char, pub variant: Variant, pub color: Rgba, pub wide: bool }` (position = coin haut-gauche de la cellule).
  - `pub struct PaneInstances { pub backgrounds: Vec<QuadInstance>, pub glyphs: Vec<GlyphRequest>, pub decorations: Vec<QuadInstance> }`.
  - `pub fn pane_instances(pane: &PaneFrame, metrics: CellMetrics, palette: &Palette, padding: u32) -> PaneInstances` : fonds par séquences de cellules de même couleur (les fonds égaux au fond de la palette ne sont **pas** émis : le `clear` de la frame s'en charge, ce qui permet l'opacité) ; une `GlyphRequest` par cellule non vide hors continuation ; soulignement et barré en décorations ; curseur : bloc plein (fenêtre focalisée) dessiné dans `backgrounds` avec `palette.cursor` et le glyphe dessous prend `palette.background` ; bloc creux de 1 px (non focalisée) en décorations ; souligné = 2 px en bas ; barre = 2 px à gauche.
  - `pub const CURSOR_BAR_WIDTH: u32 = 2;`

- [ ] **Step 1: Écrire les tests**

`crates/rustty-render/src/grid.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustty_config::Colors;
    use rustty_vt::Term;

    fn metrics() -> CellMetrics {
        CellMetrics { width: 10, height: 20, baseline: 16, underline_y: 18, underline_thickness: 1, strike_y: 10 }
    }

    fn palette() -> Palette {
        Palette::from_config(&Colors::default(), false)
    }

    fn pane_of<'a>(snapshot: &'a rustty_vt::Snapshot, focused: bool) -> PaneFrame<'a> {
        PaneFrame { rect: PixelRect::new(100, 50, 200, 100), snapshot, focused }
    }

    #[test]
    fn geometry_accounts_for_padding_and_cell_size() {
        let g = grid_geometry(PixelRect::new(100, 50, 100, 50), metrics(), 4);
        assert_eq!((g.origin_x, g.origin_y, g.cols, g.rows), (104, 54, 9, 2));
    }

    #[test]
    fn grid_geometry_survives_tiny_rects() {
        for rect in [PixelRect::new(0, 0, 0, 0), PixelRect::new(0, 0, 1, 1), PixelRect::new(5, 5, 7, 30), PixelRect::new(0, 0, 9, 19)] {
            let g = grid_geometry(rect, metrics(), 4);
            assert!(g.cols == 0 || g.rows == 0 || (g.cols >= 1 && g.rows >= 1), "{rect:?} → {g:?}");
        }
        assert_eq!(grid_geometry(PixelRect::new(0, 0, 9, 19), metrics(), 0).cols, 0);
        assert_eq!(grid_geometry(PixelRect::new(0, 0, 10, 20), metrics(), 0).cols, 1);
    }

    #[test]
    fn backgrounds_are_merged_into_runs_and_default_is_skipped() {
        let mut t = Term::new(10, 1, 0);
        t.input(b"\x1b[44mab\x1b[0mc\x1b[41md");
        let snap = t.snapshot();
        let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
        let runs: Vec<_> = out.backgrounds.iter().filter(|q| q.color != palette().cursor.to_array()).collect();
        assert_eq!(runs.len(), 2, "{:?}", out.backgrounds);
        assert_eq!(runs[0].pos, [100.0, 50.0]);
        assert_eq!(runs[0].size, [20.0, 20.0], "a et b fusionnés");
        assert_eq!(runs[0].color, palette().ansi[4].to_array());
        assert_eq!(runs[1].pos, [130.0, 50.0]);
    }

    #[test]
    fn glyph_requests_skip_blanks_and_wide_continuations() {
        let mut t = Term::new(10, 1, 0);
        t.input("a b\u{1b}[1m漢".as_bytes());
        let snap = t.snapshot();
        let out = pane_instances(&pane_of(&snap, false), metrics(), &palette(), 0);
        let chars: Vec<(char, f32, bool, Variant)> = out.glyphs.iter().map(|g| (g.ch, g.x, g.wide, g.variant)).collect();
        assert_eq!(
            chars,
            vec![('a', 100.0, false, Variant::Regular), ('b', 120.0, false, Variant::Regular), ('漢', 130.0, true, Variant::Bold)]
        );
        assert_eq!(out.glyphs[0].y, 50.0);
        assert_eq!(out.glyphs[0].color, palette().foreground);
    }

    #[test]
    fn focused_block_cursor_inverts_the_cell() {
        let mut t = Term::new(5, 1, 0);
        t.input(b"ab\x1b[D");
        let snap = t.snapshot();
        let p = palette();
        let out = pane_instances(&pane_of(&snap, true), metrics(), &p, 0);
        let cursor_quad = out.backgrounds.last().unwrap();
        assert_eq!((cursor_quad.pos, cursor_quad.size), ([110.0, 50.0], [10.0, 20.0]));
        assert_eq!(cursor_quad.color, p.cursor.to_array());
        let b = out.glyphs.iter().find(|g| g.ch == 'b').unwrap();
        assert_eq!(b.color, p.background, "le glyphe sous le curseur prend la couleur du fond");
        assert!(out.decorations.is_empty());
    }

    #[test]
    fn unfocused_cursor_is_a_hollow_block() {
        let mut t = Term::new(5, 1, 0);
        t.input(b"a");
        let snap = t.snapshot();
        let out = pane_instances(&pane_of(&snap, false), metrics(), &palette(), 0);
        assert_eq!(out.decorations.len(), 4, "quatre bords d'un pixel");
        assert!(out.decorations.iter().all(|q| q.color == palette().cursor.to_array()));
        assert!(out.backgrounds.is_empty());
    }

    #[test]
    fn beam_and_underline_cursor_shapes() {
        let mut t = Term::new(5, 1, 0);
        t.input(b"\x1b[6 q");
        let snap = t.snapshot();
        let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
        assert_eq!(out.decorations.len(), 1);
        assert_eq!((out.decorations[0].pos, out.decorations[0].size), ([100.0, 50.0], [2.0, 20.0]));
        t.input(b"\x1b[4 q");
        let snap = t.snapshot();
        let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
        assert_eq!((out.decorations[0].pos, out.decorations[0].size), ([100.0, 68.0], [10.0, 2.0]));
    }

    #[test]
    fn hidden_cursor_draws_nothing() {
        let mut t = Term::new(5, 1, 0);
        t.input(b"\x1b[?25l");
        let snap = t.snapshot();
        let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
        assert!(out.backgrounds.is_empty() && out.decorations.is_empty());
    }

    #[test]
    fn underline_and_strikethrough_decorations() {
        let mut t = Term::new(5, 1, 0);
        t.input(b"\x1b[4ma\x1b[0m\x1b[9mb\x1b[?25l");
        let snap = t.snapshot();
        let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
        assert_eq!(out.decorations.len(), 2);
        assert_eq!((out.decorations[0].pos, out.decorations[0].size), ([100.0, 68.0], [10.0, 1.0]), "soulignement");
        assert_eq!((out.decorations[1].pos, out.decorations[1].size), ([110.0, 60.0], [10.0, 1.0]), "barré");
    }

    #[test]
    fn cells_outside_the_geometry_are_clipped() {
        let mut t = Term::new(40, 10, 0);
        t.input(b"\x1b[1;1Hy\x1b[10;40Hx");
        let snap = t.snapshot();
        let pane = PaneFrame { rect: PixelRect::new(0, 0, 50, 40), snapshot: &snap, focused: false };
        let out = pane_instances(&pane, metrics(), &palette(), 0);
        assert_eq!(out.glyphs.iter().map(|g| g.ch).collect::<Vec<_>>(), vec!['y'], "5 colonnes × 2 lignes visibles");
        assert!(out.decorations.is_empty(), "le curseur hors zone n'est pas dessiné");
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render`
Expected: modules `frame` et `grid` introuvables.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/src/frame.rs` :

```rust
//! Ce que le binaire donne au renderer pour une image : les panneaux avec
//! leur instantané, et le chrome (barre d'onglets, bordures, bandeaux)
//! déjà réduit à des rectangles et des textes.

use rustty_vt::Snapshot;

use crate::color::Rgba;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

pub struct PaneFrame<'a> {
    pub rect: PixelRect,
    pub snapshot: &'a Snapshot,
    pub focused: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChromeQuad {
    pub rect: PixelRect,
    pub color: Rgba,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChromeText {
    pub x: u32,
    pub y: u32,
    pub text: String,
    pub color: Rgba,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chrome {
    pub quads: Vec<ChromeQuad>,
    pub texts: Vec<ChromeText>,
}

pub struct Frame<'a> {
    pub viewport: (u32, u32),
    /// Fond de la fenêtre ; son alpha est l'opacité configurée.
    pub background: Rgba,
    pub panes: Vec<PaneFrame<'a>>,
    pub chrome: Chrome,
}
```

`crates/rustty-render/src/grid.rs` :

```rust
//! Du `Snapshot` aux instances : géométrie des cellules, fonds fusionnés,
//! glyphes à demander, décorations et curseur. Pur, sans GPU.

use rustty_vt::{Attrs, CursorShape, Snapshot};

use crate::color::{Palette, Rgba};
use crate::font::{CellMetrics, Variant};
use crate::frame::{PaneFrame, PixelRect};
use crate::pipeline::quad::QuadInstance;

pub const CURSOR_BAR_WIDTH: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridGeometry {
    pub origin_x: u32,
    pub origin_y: u32,
    pub cols: usize,
    pub rows: usize,
}

pub fn grid_geometry(rect: PixelRect, metrics: CellMetrics, padding: u32) -> GridGeometry {
    let inner_w = rect.width.saturating_sub(2 * padding);
    let inner_h = rect.height.saturating_sub(2 * padding);
    GridGeometry {
        origin_x: rect.x + padding,
        origin_y: rect.y + padding,
        cols: (inner_w / metrics.width.max(1)) as usize,
        rows: (inner_h / metrics.height.max(1)) as usize,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlyphRequest {
    pub x: f32,
    pub y: f32,
    pub ch: char,
    pub variant: Variant,
    pub color: Rgba,
    pub wide: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaneInstances {
    pub backgrounds: Vec<QuadInstance>,
    pub glyphs: Vec<GlyphRequest>,
    pub decorations: Vec<QuadInstance>,
}

pub fn pane_instances(pane: &PaneFrame, metrics: CellMetrics, palette: &Palette, padding: u32) -> PaneInstances {
    let geo = grid_geometry(pane.rect, metrics, padding);
    let snap: &Snapshot = pane.snapshot;
    let (cw, ch) = (metrics.width as f32, metrics.height as f32);
    let cols = geo.cols.min(snap.cols);
    let rows = geo.rows.min(snap.rows);
    let cursor = snap.cursor.filter(|c| c.col < cols && c.row < rows);
    let block_cursor = pane.focused && cursor.is_some() && snap.cursor_shape == CursorShape::Block;
    let mut out = PaneInstances::default();
    for (row, line) in snap.lines.iter().take(rows).enumerate() {
        let y = geo.origin_y as f32 + row as f32 * ch;
        let mut run: Option<(usize, usize, Rgba)> = None;
        for col in 0..cols {
            let cell = line.get(col);
            let (mut fg, bg) = palette.cell_colors(&cell.style);
            let under_cursor = block_cursor && cursor.is_some_and(|c| c.col == col && c.row == row);
            if under_cursor {
                fg = palette.background;
            }
            // Fonds : fusion des cellules contiguës de même couleur.
            match run {
                Some((start, len, color)) if color == bg => run = Some((start, len + 1, color)),
                Some((start, len, color)) => {
                    push_run(&mut out.backgrounds, palette, geo.origin_x as f32, y, cw, ch, start, len, color);
                    run = Some((col, 1, bg));
                }
                None => run = Some((col, 1, bg)),
            }
            let x = geo.origin_x as f32 + col as f32 * cw;
            if cell.c != ' ' && !cell.is_wide_continuation() {
                out.glyphs.push(GlyphRequest { x, y, ch: cell.c, variant: Variant::from_attrs(cell.style.attrs), color: fg, wide: cell.is_wide() });
            }
            if cell.style.attrs.contains(Attrs::UNDERLINE) {
                out.decorations.push(QuadInstance::new(x, y + metrics.underline_y as f32, cw, metrics.underline_thickness as f32, fg));
            }
            if cell.style.attrs.contains(Attrs::STRIKETHROUGH) {
                out.decorations.push(QuadInstance::new(x, y + metrics.strike_y as f32, cw, metrics.underline_thickness as f32, fg));
            }
        }
        if let Some((start, len, color)) = run {
            push_run(&mut out.backgrounds, palette, geo.origin_x as f32, y, cw, ch, start, len, color);
        }
    }
    if let Some(c) = cursor {
        let x = geo.origin_x as f32 + c.col as f32 * cw;
        let y = geo.origin_y as f32 + c.row as f32 * ch;
        let bar = CURSOR_BAR_WIDTH as f32;
        match (pane.focused, snap.cursor_shape) {
            (true, CursorShape::Block) => out.backgrounds.push(QuadInstance::new(x, y, cw, ch, palette.cursor)),
            (false, CursorShape::Block) => out.decorations.extend([
                QuadInstance::new(x, y, cw, 1.0, palette.cursor),
                QuadInstance::new(x, y + ch - 1.0, cw, 1.0, palette.cursor),
                QuadInstance::new(x, y, 1.0, ch, palette.cursor),
                QuadInstance::new(x + cw - 1.0, y, 1.0, ch, palette.cursor),
            ]),
            (_, CursorShape::Underline) => out.decorations.push(QuadInstance::new(x, y + ch - bar, cw, bar, palette.cursor)),
            (_, CursorShape::Beam) => out.decorations.push(QuadInstance::new(x, y, bar, ch, palette.cursor)),
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn push_run(out: &mut Vec<QuadInstance>, palette: &Palette, origin_x: f32, y: f32, cw: f32, ch: f32, start: usize, len: usize, color: Rgba) {
    if color == palette.background {
        return;
    }
    out.push(QuadInstance::new(origin_x + start as f32 * cw, y, len as f32 * cw, ch, color));
}
```

Si clippy refuse `too_many_arguments` même avec l'`allow`, regrouper `(origin_x, y, cw, ch)` dans une petite struct `RowGeometry`. Le test `focused_block_cursor_inverts_the_cell` compte sur l'ordre : le quad du curseur est poussé après les fonds de toutes les lignes.

`lib.rs` : `pub mod frame; pub mod grid;` et `pub use frame::{Chrome, ChromeQuad, ChromeText, Frame, PaneFrame, PixelRect}; pub use grid::{CURSOR_BAR_WIDTH, GlyphRequest, GridGeometry, PaneInstances, grid_geometry, pane_instances};`.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Vérification du test des fonds : cellules `a`,`b` bleues (run 0–1, x = 100, largeur 20), `c` fond par défaut (run non émis), `d` rouge (x = 130), puis les cellules vides par défaut (non émises) ; le curseur focalisé en colonne 4 ajoute un quad `palette.cursor` que le filtre du test écarte.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.47"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.47 — 2026-10-08 · « Instances de grille »

- `rustty-render` : `Frame`/`PaneFrame`/`Chrome` comme contrat avec le binaire ; géométrie des cellules avec marge, fonds fusionnés par séquences, glyphes demandés, soulignement, barré et les trois formes de curseur, le tout sans GPU.
```

```bash
git add Cargo.toml crates/rustty-render CHANGELOG.md
git commit -m "rustty-render : frame et instances de grille (0.1.0-alpha.47)"
```

---

### Task 13 : `Renderer` — cache de glyphes, passes de rendu, images de référence

**Files:**
- Create: `crates/rustty-render/src/renderer.rs`
- Create: `crates/rustty-render/tests/offscreen.rs`
- Create: `crates/rustty-render/tests/golden/` (PNG générés à la première exécution)
- Modify: `crates/rustty-render/src/lib.rs`
- Modify: `crates/rustty-render/Cargo.toml` (`unicode-width` ; `image` en dev-dependency)

**Interfaces:**
- Produces:
  - `pub struct Renderer` ; `Renderer::new(ctx: &GpuContext, format: wgpu::TextureFormat, fonts: FontSet, palette: Palette, padding: u32) -> Renderer` ; `metrics() -> CellMetrics` ; `fonts() -> &FontSet` ; `set_palette(&mut self, palette)` ; `render(&mut self, ctx: &GpuContext, view: &wgpu::TextureView, frame: &Frame)`.
  - Ordre de dessin : effacement avec `frame.background` (son alpha est l'opacité), fonds et curseur-bloc des panneaux, glyphes des panneaux, décorations des panneaux puis quads du chrome, glyphes des textes du chrome.
  - Cache de glyphes par `(char, Variant)` ; glyphe procédural d'abord, sinon police ; atlas plein ⇒ vidé et reconstruit ; glyphe introuvable ⇒ cellule vide.
  - Tests hors écran avec images de référence dans `tests/golden/*.png` : générées quand elles manquent ou si `UPDATE_GOLDEN=1`, comparées sinon (différence moyenne ≤ 1,0 par canal, ≤ 0,5 % de pixels différant de plus de 8) ; l'image obtenue est écrite dans `target/golden-actual/<nom>.png` en cas d'échec.

- [ ] **Step 1: Écrire les tests**

`crates/rustty-render/tests/offscreen.rs` :

```rust
//! Rendu complet hors écran, comparé à des images de référence produites
//! avec la police embarquée : identiques sur les trois OS à l'arrondi près.

mod common;

use std::path::PathBuf;

use rustty_config::Colors;
use rustty_render::{Frame, FontSet, GpuContext, OFFSCREEN_FORMAT, Offscreen, Palette, PaneFrame, PixelRect, Renderer, Rgba};
use rustty_vt::Term;

const FONT_PX: f32 = 16.0;
const PADDING: u32 = 4;

fn renderer(ctx: &GpuContext) -> Renderer {
    Renderer::new(ctx, OFFSCREEN_FORMAT, FontSet::embedded(FONT_PX), Palette::from_config(&Colors::default(), false), PADDING)
}

/// Rend `term` dans un viewport ajusté à sa grille et rend les pixels.
fn render_term(ctx: &GpuContext, r: &mut Renderer, term: &Term, focused: bool, opacity: f32) -> (Vec<u8>, u32, u32) {
    let m = r.metrics();
    let snap = term.snapshot();
    let width = snap.cols as u32 * m.width + 2 * PADDING;
    let height = snap.rows as u32 * m.height + 2 * PADDING;
    let target = Offscreen::new(ctx, width, height);
    let palette = Palette::from_config(&Colors::default(), false);
    let frame = Frame {
        viewport: (width, height),
        background: palette.background.with_alpha(opacity),
        panes: vec![PaneFrame { rect: PixelRect::new(0, 0, width, height), snapshot: &snap, focused }],
        chrome: Default::default(),
    };
    r.render(ctx, target.view(), &frame);
    (target.read_rgba(ctx).unwrap(), width, height)
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.png"))
}

/// Compare à l'image de référence, ou la crée si elle manque / si UPDATE_GOLDEN=1.
fn assert_matches_golden(name: &str, px: &[u8], width: u32, height: u32) {
    let path = golden_path(name);
    let actual = image::RgbaImage::from_raw(width, height, px.to_vec()).expect("dimensions cohérentes");
    if std::env::var_os("UPDATE_GOLDEN").is_some() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        actual.save(&path).unwrap();
        eprintln!("image de référence écrite : {}", path.display());
        return;
    }
    let expected = image::open(&path).unwrap().to_rgba8();
    assert_eq!(expected.dimensions(), (width, height), "dimensions de {name}");
    let mut total_diff = 0u64;
    let mut bad_pixels = 0u64;
    for (a, b) in expected.pixels().zip(actual.pixels()) {
        let d: [i32; 4] = std::array::from_fn(|i| (i32::from(a.0[i]) - i32::from(b.0[i])).abs());
        total_diff += d.iter().map(|&v| v as u64).sum::<u64>();
        if d.iter().any(|&v| v > 8) {
            bad_pixels += 1;
        }
    }
    let n = u64::from(width) * u64::from(height);
    let mean = total_diff as f64 / (n * 4) as f64;
    let bad_ratio = bad_pixels as f64 / n as f64;
    if mean > 1.0 || bad_ratio > 0.005 {
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/golden-actual").join(format!("{name}.png"));
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        actual.save(&out).unwrap();
        panic!("{name} diffère de la référence : moyenne {mean:.3}, {bad_ratio:.4} de pixels > 8 ; image obtenue : {}", out.display());
    }
}

fn pixel(px: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn hello_world_matches_golden() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let mut r = renderer(&ctx);
    let mut term = Term::new(14, 2, 0);
    term.input(b"Hello, \x1b[1;31mworld\x1b[0m!\r\n\x1b[4mrustty\x1b[0m \x1b[44m  \x1b[0m");
    let (px, w, h) = render_term(&ctx, &mut r, &term, true, 1.0);
    assert_matches_golden("hello_world", &px, w, h);
}

#[test]
fn box_drawing_and_powerline_match_golden() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let mut r = renderer(&ctx);
    let mut term = Term::new(8, 3, 0);
    term.input("┌──┐\u{E0B0}\u{E0B6}\u{E0B4}\r\n│漢│\r\n└──┘░▒▓█\x1b[?25l".as_bytes());
    let (px, w, h) = render_term(&ctx, &mut r, &term, true, 1.0);
    assert_matches_golden("box_drawing", &px, w, h);
}

#[test]
fn cursor_block_is_visible_and_inverts_the_glyph() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let mut r = renderer(&ctx);
    let mut term = Term::new(4, 1, 0);
    term.input(b"ab\x1b[D");
    let (px, w, _) = render_term(&ctx, &mut r, &term, true, 1.0);
    let m = r.metrics();
    let cursor_px = pixel(&px, w, PADDING + m.width + 1, PADDING + 1);
    let cursor = Palette::from_config(&Colors::default(), false).cursor.to_u8();
    assert_eq!(cursor_px, cursor, "coin du bloc curseur");
}

#[test]
fn unknown_characters_render_as_blank() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let mut r = renderer(&ctx);
    let mut term = Term::new(3, 1, 0);
    term.input("\u{E000}\u{10FFFF}\x1b[?25l".as_bytes());
    let (px, w, h) = render_term(&ctx, &mut r, &term, true, 1.0);
    let bg = Palette::from_config(&Colors::default(), false).background.to_u8();
    let m = r.metrics();
    for y in PADDING..PADDING + m.height {
        for x in PADDING..PADDING + 2 * m.width {
            assert_eq!(pixel(&px, w, x, y), bg, "({x},{y}) doit rester fond");
        }
    }
    let _ = h;
}

#[test]
fn background_opacity_is_written_to_alpha() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let mut r = renderer(&ctx);
    let mut term = Term::new(2, 1, 0);
    term.input(b"\x1b[?25l");
    let (px, w, _) = render_term(&ctx, &mut r, &term, true, 0.5);
    let a = pixel(&px, w, 0, 0)[3];
    assert!((127..=128).contains(&a), "alpha du fond : {a}");
}

#[test]
fn render_survives_tiny_viewport() {
    let Some(ctx) = common::gpu_or_skip() else { return };
    let mut r = renderer(&ctx);
    let mut term = Term::new(10, 3, 0);
    term.input(b"abc");
    let snap = term.snapshot();
    for (vw, vh) in [(0, 0), (1, 1), (5, 3)] {
        let target = Offscreen::new(&ctx, vw, vh);
        let frame = Frame {
            viewport: (vw, vh),
            background: Rgba::new(0.0, 0.0, 0.0, 1.0),
            panes: vec![PaneFrame { rect: PixelRect::new(0, 0, vw, vh), snapshot: &snap, focused: true }],
            chrome: Default::default(),
        };
        r.render(&ctx, target.view(), &frame);
        let px = target.read_rgba(&ctx).unwrap();
        assert_eq!(px.len() as u32, target.size().0 * target.size().1 * 4);
    }
}

```

Le débordement d'atlas se teste en unitaire dans `renderer.rs` avec un atlas de 64 px (ci-dessous), pas ici.

`crates/rustty-render/src/renderer.rs`, bas de fichier (test unitaire GPU du débordement d'atlas) :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustty_config::Colors;

    #[test]
    fn atlas_overflow_is_recovered() {
        let Ok(ctx) = GpuContext::headless() else {
            eprintln!("test GPU ignoré");
            return;
        };
        let mut r = Renderer::with_atlas_size(&ctx, OFFSCREEN_FORMAT, FontSet::embedded(16.0), Palette::from_config(&Colors::default(), false), 0, 64);
        let mut instances = Vec::new();
        for ch in "abcdefghijklmnopqrstuvwxyz0123456789".chars() {
            let req = GlyphRequest { x: 0.0, y: 0.0, ch, variant: Variant::Regular, color: Rgba::new(1.0, 1.0, 1.0, 1.0), wide: false };
            if let Some(i) = r.glyph_instance(&ctx, &req) {
                instances.push(i);
            }
        }
        assert_eq!(instances.len(), 36, "chaque glyphe a été placé, au prix de reconstructions");
        assert!(r.rebuilds >= 1, "un atlas de 64 px déborde forcément");
        let again = r.glyph_instance(&ctx, &GlyphRequest { x: 0.0, y: 0.0, ch: 'a', variant: Variant::Regular, color: Rgba::new(1.0, 1.0, 1.0, 1.0), wide: false });
        assert!(again.is_some());
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render --test offscreen`
Expected: `unresolved import rustty_render::Renderer`.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/Cargo.toml` : `unicode-width.workspace = true` dans `[dependencies]` ; `image.workspace = true` dans `[dev-dependencies]`.

`crates/rustty-render/src/renderer.rs` :

```rust
//! Orchestration : du `Frame` aux passes wgpu, avec le cache de glyphes et
//! l'atlas. Une image = un effacement, deux lots de quads, deux lots de glyphes.

use std::collections::HashMap;

use unicode_width::UnicodeWidthChar;

use crate::atlas::{AtlasPacker, AtlasRegion, DEFAULT_ATLAS_SIZE};
use crate::builtin::builtin_glyph;
use crate::color::{Palette, Rgba};
use crate::font::{CellMetrics, FontSet, Rasterizer, Variant};
use crate::frame::Frame;
use crate::gpu::{GpuContext, to_wgpu_color};
use crate::grid::{GlyphRequest, pane_instances};
use crate::pipeline::glyph::{AtlasTexture, GlyphInstance, GlyphPipeline};
use crate::pipeline::quad::{QuadInstance, QuadPipeline};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    ch: char,
    variant: Variant,
}

#[derive(Clone, Copy)]
struct CachedGlyph {
    region: AtlasRegion,
    left: i32,
    top: i32,
    is_color: bool,
}

pub struct Renderer {
    quads: QuadPipeline,
    glyphs: GlyphPipeline,
    atlas: AtlasTexture,
    packer: AtlasPacker,
    cache: HashMap<GlyphKey, Option<CachedGlyph>>,
    fonts: FontSet,
    rasterizer: Rasterizer,
    palette: Palette,
    padding: u32,
    /// Nombre de reconstructions de l'atlas (diagnostic et tests).
    pub(crate) rebuilds: u32,
}

impl Renderer {
    pub fn new(ctx: &GpuContext, format: wgpu::TextureFormat, fonts: FontSet, palette: Palette, padding: u32) -> Self {
        Self::with_atlas_size(ctx, format, fonts, palette, padding, DEFAULT_ATLAS_SIZE)
    }

    pub(crate) fn with_atlas_size(ctx: &GpuContext, format: wgpu::TextureFormat, fonts: FontSet, palette: Palette, padding: u32, atlas_size: u32) -> Self {
        Self {
            quads: QuadPipeline::new(&ctx.device, format),
            glyphs: GlyphPipeline::new(&ctx.device, format),
            atlas: AtlasTexture::new(&ctx.device, atlas_size),
            packer: AtlasPacker::new(atlas_size),
            cache: HashMap::new(),
            fonts,
            rasterizer: Rasterizer::new(),
            palette,
            padding,
            rebuilds: 0,
        }
    }

    pub fn metrics(&self) -> CellMetrics {
        self.fonts.metrics()
    }

    pub fn fonts(&self) -> &FontSet {
        &self.fonts
    }

    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    pub fn render(&mut self, ctx: &GpuContext, view: &wgpu::TextureView, frame: &Frame) {
        let metrics = self.metrics();
        let mut backgrounds = Vec::new();
        let mut requests = Vec::new();
        let mut overlay = Vec::new();
        for pane in &frame.panes {
            let inst = pane_instances(pane, metrics, &self.palette, self.padding);
            backgrounds.extend(inst.backgrounds);
            requests.extend(inst.glyphs);
            overlay.extend(inst.decorations);
        }
        overlay.extend(frame.chrome.quads.iter().map(|q| {
            QuadInstance::new(q.rect.x as f32, q.rect.y as f32, q.rect.width as f32, q.rect.height as f32, q.color)
        }));
        let mut chrome_requests = Vec::new();
        for text in &frame.chrome.texts {
            let mut col = 0u32;
            for ch in text.text.chars() {
                let x = (text.x + col * metrics.width) as f32;
                chrome_requests.push(GlyphRequest { x, y: text.y as f32, ch, variant: Variant::Regular, color: text.color, wide: false });
                col += ch.width().unwrap_or(1) as u32;
            }
        }
        let pane_glyphs: Vec<GlyphInstance> = requests.iter().filter_map(|r| self.glyph_instance(ctx, r)).collect();
        let chrome_glyphs: Vec<GlyphInstance> = chrome_requests.iter().filter_map(|r| self.glyph_instance(ctx, r)).collect();
        let viewport = frame.viewport;
        let bg_batch = self.quads.prepare(&ctx.device, &backgrounds, viewport);
        let overlay_batch = self.quads.prepare(&ctx.device, &overlay, viewport);
        let pane_batch = self.glyphs.prepare(&ctx.device, &self.atlas, &pane_glyphs, viewport);
        let chrome_batch = self.glyphs.prepare(&ctx.device, &self.atlas, &chrome_glyphs, viewport);
        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("rustty-frame") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rustty-frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(to_wgpu_color(frame.background)), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(b) = &bg_batch {
                self.quads.draw(&mut pass, b);
            }
            if let Some(b) = &pane_batch {
                self.glyphs.draw(&mut pass, b);
            }
            if let Some(b) = &overlay_batch {
                self.quads.draw(&mut pass, b);
            }
            if let Some(b) = &chrome_batch {
                self.glyphs.draw(&mut pass, b);
            }
        }
        ctx.queue.submit(Some(encoder.finish()));
    }

    /// L'instance à dessiner pour une demande, `None` si le caractère n'a
    /// aucun glyphe (il reste une cellule vide).
    pub(crate) fn glyph_instance(&mut self, ctx: &GpuContext, req: &GlyphRequest) -> Option<GlyphInstance> {
        let key = GlyphKey { ch: req.ch, variant: req.variant };
        let cached = match self.cache.get(&key) {
            Some(c) => *c,
            None => {
                let c = self.load_glyph(ctx, key);
                self.cache.insert(key, c);
                c
            }
        }?;
        let metrics = self.metrics();
        let x = req.x + cached.left as f32;
        let y = req.y + metrics.baseline as f32 - cached.top as f32;
        Some(GlyphInstance::new(
            x,
            y,
            cached.region.width as f32,
            cached.region.height as f32,
            cached.region.uv(self.packer.size()),
            req.color,
            cached.is_color,
        ))
    }

    fn load_glyph(&mut self, ctx: &GpuContext, key: GlyphKey) -> Option<CachedGlyph> {
        let metrics = self.metrics();
        let bitmap = match builtin_glyph(key.ch, metrics) {
            Some(b) => b,
            None => {
                let glyph = self.fonts.glyph(key.ch, key.variant)?;
                let face = self.fonts.face_by_slot(glyph.slot).clone();
                self.rasterizer.rasterize(&face, self.fonts.size_px(), glyph.glyph_id)?
            }
        };
        let region = match self.packer.insert(bitmap.width, bitmap.height) {
            Some(r) => r,
            None => {
                // Atlas plein : on repart de zéro. Les entrées du cache qui
                // pointaient dans l'ancien atlas sont invalidées.
                self.packer.clear();
                self.atlas.clear(&ctx.queue);
                self.cache.clear();
                self.rebuilds += 1;
                self.packer.insert(bitmap.width, bitmap.height)?
            }
        };
        self.atlas.upload(&ctx.queue, region, &bitmap.data);
        Some(CachedGlyph { region, left: bitmap.left, top: bitmap.top, is_color: bitmap.is_color })
    }
}
```

Note sur l'invalidation : `glyph_instance` insère dans le cache **après** `load_glyph`, donc un `cache.clear()` déclenché pendant ce chargement n'efface pas l'entrée en cours ; les instances déjà construites pour cette image avec l'ancien atlas sont fausses pendant une seule image après un débordement, ce qui est acceptable pour la v0.1 (l'atlas par défaut de 2048² ne déborde pas en usage courant).

`lib.rs` : `pub mod renderer;` et `pub use renderer::Renderer;`.

Générer les images de référence sur cette machine : `UPDATE_GOLDEN=1 cargo test -p rustty-render --test offscreen`, puis **ouvrir** `crates/rustty-render/tests/golden/hello_world.png` et `box_drawing.png` (avec l'outil `Read` ou un visualiseur) et vérifier à l'œil : texte lisible, « world » en rouge gras, « rustty » souligné, deux cellules bleues ; cadre fermé, triangle et demi-disques powerline, trames de densité croissante, bloc plein. Relancer `cargo test` sans la variable : les comparaisons passent.

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok`. Une image de référence absente est créée (premier passage) ; un second passage compare.

- [ ] **Step 5: Version, CHANGELOG, commit**

`version = "0.1.0-alpha.48"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.48 — 2026-10-08 · « Renderer »

- `rustty-render` : `Renderer` assemble fonds, glyphes (cache + atlas reconstruit au débordement), décorations et chrome en une passe ; opacité du fond par l'alpha ; images de référence comparées hors écran sur les trois OS.
```

```bash
git add Cargo.toml Cargo.lock crates/rustty-render/Cargo.toml crates/rustty-render/src crates/rustty-render/tests/offscreen.rs crates/rustty-render/tests/golden/hello_world.png crates/rustty-render/tests/golden/box_drawing.png CHANGELOG.md
git commit -m "rustty-render : renderer complet et images de référence (0.1.0-alpha.48)"
```

---

### Task 14 : Barre d'onglets — disposition, boutons de fermeture, test de clic, survol

**Files:**
- Create: `crates/rustty-render/src/chrome.rs`
- Modify: `crates/rustty-render/src/lib.rs`
- Modify: `crates/rustty-render/tests/offscreen.rs`
- Modify: `README.md`

**Interfaces:**
- Consumes: `CellMetrics`, `PixelRect`, `Chrome`, `ChromeQuad`, `ChromeText`, `Rgba`, `rustty_config::CloseButtonStyle`.
- Produces :
  - `pub struct TabBarStyle { pub background: Rgba, pub active_background: Rgba, pub inactive_background: Rgba, pub active_foreground: Rgba, pub inactive_foreground: Rgba, pub close_foreground: Rgba, pub close_background: Rgba, pub close_hover_foreground: Rgba, pub close_hover_background: Rgba, pub show_close_button: bool }` ; `TabBarStyle::from_config(palette: &Palette, close: &CloseButtonStyle, show_close_button: bool)` (fonds d'onglet : actif = `palette.background`, inactif = `ansi[0]`, barre = `ansi[8]` assombri à 0,5 ; textes : actif = `palette.foreground`, inactif = `ansi[7]`).
  - `pub struct TabSpec<'a> { pub title: &'a str, pub active: bool }`.
  - `pub enum HoverTarget { None, Tab(usize), CloseButton(usize), NewTabButton }`.
  - `pub struct TabRect { pub rect: PixelRect, pub close: Option<PixelRect> }` ; `pub struct TabBarLayout { pub bar: PixelRect, pub tabs: Vec<TabRect>, pub new_tab: PixelRect }` ; `TabBarLayout::hit_test(&self, x: u32, y: u32) -> HoverTarget` (bouton ✕ avant onglet, puis « + », sinon `None`).
  - `pub fn tab_bar_height(metrics: CellMetrics) -> u32` (= `metrics.height + 2 × TAB_BAR_PADDING`, `TAB_BAR_PADDING = 2`).
  - `pub fn layout_tab_bar(viewport_width: u32, y: u32, tabs: &[TabSpec], style: &TabBarStyle, metrics: CellMetrics) -> TabBarLayout` : chaque onglet occupe `(1 + colonnes du titre + 1 + [4 si bouton]) × largeur de cellule`, les onglets se suivent, sont tronqués à la largeur du viewport (un onglet qui ne tient pas est absent de `tabs`, sa place reste vide), et le bouton « + » (3 cellules) suit le dernier onglet s'il reste la place.
  - `pub fn tab_bar_chrome(layout: &TabBarLayout, tabs: &[TabSpec], style: &TabBarStyle, metrics: CellMetrics, hover: HoverTarget) -> Chrome` : fond de barre, fond par onglet, titre (texte, titre tronqué avec `…` à la largeur disponible), bouton ✕ = demi-disque gauche U+E0B6 et droit U+E0B4 (texte de la couleur du bouton sur le fond de l'onglet) autour d'une cellule de fond `close_background` avec le texte `✕` en `close_foreground` ; au survol du bouton, couleurs `close_hover_*` ; le bouton « + » en texte.

- [ ] **Step 1: Écrire les tests**

`crates/rustty-render/src/chrome.rs`, bas de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustty_config::{CloseButtonStyle, Colors};

    fn metrics() -> CellMetrics {
        CellMetrics { width: 10, height: 20, baseline: 16, underline_y: 18, underline_thickness: 1, strike_y: 10 }
    }

    fn style(show_close: bool) -> TabBarStyle {
        TabBarStyle::from_config(&Palette::from_config(&Colors::default(), false), &CloseButtonStyle::default(), show_close)
    }

    fn tabs() -> Vec<TabSpec<'static>> {
        vec![TabSpec { title: "1: sh", active: true }, TabSpec { title: "2: vim", active: false }]
    }

    #[test]
    fn height_is_a_cell_plus_padding() {
        assert_eq!(tab_bar_height(metrics()), 24);
    }

    #[test]
    fn tabs_are_laid_out_left_to_right_with_close_buttons() {
        let l = layout_tab_bar(400, 0, &tabs(), &style(true), metrics());
        assert_eq!(l.bar, PixelRect::new(0, 0, 400, 24));
        assert_eq!(l.tabs.len(), 2);
        // « 1: sh » = 5 colonnes + 1 + 1 + 4 (bouton) = 11 cellules = 110 px.
        assert_eq!(l.tabs[0].rect, PixelRect::new(0, 0, 110, 24));
        assert_eq!(l.tabs[0].close, Some(PixelRect::new(60, 2, 40, 20)), "espace, demi-disque, ✕, demi-disque");
        assert_eq!(l.tabs[1].rect.x, 110);
        assert_eq!(l.tabs[1].rect.width, 120);
        assert_eq!(l.new_tab, PixelRect::new(230, 0, 30, 24));
    }

    #[test]
    fn without_close_button_tabs_are_narrower() {
        let l = layout_tab_bar(400, 0, &tabs(), &style(false), metrics());
        assert_eq!(l.tabs[0].rect.width, 70);
        assert!(l.tabs[0].close.is_none());
    }

    #[test]
    fn tabs_that_do_not_fit_are_dropped() {
        let l = layout_tab_bar(150, 0, &tabs(), &style(true), metrics());
        assert_eq!(l.tabs.len(), 1, "le second onglet (120 px) ne tient pas après 110 px");
        assert_eq!(l.new_tab.width, 30);
        let tiny = layout_tab_bar(20, 0, &tabs(), &style(true), metrics());
        assert!(tiny.tabs.is_empty());
        assert_eq!(tiny.new_tab.width, 0, "pas de place pour « + »");
    }

    #[test]
    fn hit_test_prefers_close_button_then_tab_then_new_tab() {
        let l = layout_tab_bar(400, 0, &tabs(), &style(true), metrics());
        assert_eq!(l.hit_test(5, 5), HoverTarget::Tab(0));
        assert_eq!(l.hit_test(75, 10), HoverTarget::CloseButton(0));
        assert_eq!(l.hit_test(115, 10), HoverTarget::Tab(1));
        assert_eq!(l.hit_test(240, 10), HoverTarget::NewTabButton);
        assert_eq!(l.hit_test(300, 10), HoverTarget::None);
        assert_eq!(l.hit_test(5, 30), HoverTarget::None, "sous la barre");
    }

    #[test]
    fn chrome_colors_follow_active_state_and_hover() {
        let st = style(true);
        let l = layout_tab_bar(400, 0, &tabs(), &st, metrics());
        let normal = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::None);
        assert_eq!(normal.quads[0].color, st.background, "fond de barre d'abord");
        let active_bg = normal.quads.iter().find(|q| q.rect == l.tabs[0].rect).unwrap();
        assert_eq!(active_bg.color, st.active_background);
        let inactive_bg = normal.quads.iter().find(|q| q.rect == l.tabs[1].rect).unwrap();
        assert_eq!(inactive_bg.color, st.inactive_background);
        let close0 = normal.quads.iter().find(|q| q.rect.x == 80 && q.rect.width == 10).unwrap();
        assert_eq!(close0.color, st.close_background);
        let x_text = normal.texts.iter().find(|t| t.text == "✕" && t.x == 80).unwrap();
        assert_eq!(x_text.color, st.close_foreground);
        let discs: Vec<_> = normal.texts.iter().filter(|t| t.text == "\u{E0B6}" || t.text == "\u{E0B4}").collect();
        assert_eq!(discs.len(), 4, "deux demi-disques par onglet");
        assert!(discs.iter().all(|t| t.color == st.close_background));
        let title = normal.texts.iter().find(|t| t.text == "1: sh").unwrap();
        assert_eq!((title.x, title.y, title.color), (10, 2, st.active_foreground));
        let plus = normal.texts.iter().find(|t| t.text == "+").unwrap();
        assert_eq!(plus.x, 240);

        let hovered = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::CloseButton(0));
        let close0 = hovered.quads.iter().find(|q| q.rect.x == 80 && q.rect.width == 10).unwrap();
        assert_eq!(close0.color, st.close_hover_background);
        let x_text = hovered.texts.iter().find(|t| t.text == "✕" && t.x == 80).unwrap();
        assert_eq!(x_text.color, st.close_hover_foreground);
        let close1 = hovered.quads.iter().find(|q| q.rect.x == 200 && q.rect.width == 10).unwrap();
        assert_eq!(close1.color, st.close_background, "l'autre bouton garde ses couleurs");
    }

    #[test]
    fn long_titles_are_truncated_with_an_ellipsis() {
        let long = [TabSpec { title: "un titre vraiment beaucoup trop long pour la barre", active: true }];
        let l = layout_tab_bar(200, 0, &long, &style(false), metrics());
        assert_eq!(l.tabs.len(), 1);
        assert!(l.tabs[0].rect.width <= 200);
        let chrome = tab_bar_chrome(&l, &long, &style(false), metrics(), HoverTarget::None);
        let title = chrome.texts.iter().find(|t| t.text.ends_with('…')).expect("titre tronqué");
        assert!(title.text.chars().count() * 10 <= 180, "{}", title.text);
    }
}
```

Ajouter dans `crates/rustty-render/tests/offscreen.rs` :

```rust
#[test]
fn tab_bar_with_close_buttons_matches_golden() {
    use rustty_render::{HoverTarget, TabBarStyle, TabSpec, layout_tab_bar, tab_bar_chrome, tab_bar_height};
    let Some(ctx) = common::gpu_or_skip() else { return };
    let mut r = renderer(&ctx);
    let m = r.metrics();
    let palette = Palette::from_config(&Colors::default(), false);
    let style = TabBarStyle::from_config(&palette, &rustty_config::CloseButtonStyle::default(), true);
    let tabs = [TabSpec { title: "1: sh", active: true }, TabSpec { title: "2: vim", active: false }];
    let width = 30 * m.width;
    let bar_h = tab_bar_height(m);
    let layout = layout_tab_bar(width, 0, &tabs, &style, m);
    let chrome = tab_bar_chrome(&layout, &tabs, &style, m, HoverTarget::CloseButton(1));
    let mut term = Term::new(30, 1, 0);
    term.input(b"$ \x1b[?25l");
    let snap = term.snapshot();
    let height = bar_h + m.height + 2 * PADDING;
    let target = Offscreen::new(&ctx, width, height);
    let frame = Frame {
        viewport: (width, height),
        background: palette.background,
        panes: vec![PaneFrame { rect: PixelRect::new(0, bar_h, width, height - bar_h), snapshot: &snap, focused: true }],
        chrome,
    };
    r.render(&ctx, target.view(), &frame);
    let px = target.read_rgba(&ctx).unwrap();
    assert_matches_golden("tab_bar", &px, width, height);
    let hover_px = pixel(&px, width, layout.tabs[1].close.unwrap().x + 15, 10);
    assert_eq!(hover_px, style.close_hover_background.to_u8(), "le bouton survolé est dans sa couleur de survol");
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p rustty-render`
Expected: module `chrome` introuvable.

- [ ] **Step 3: Implémenter**

`crates/rustty-render/src/chrome.rs` :

```rust
//! La barre d'onglets : disposition en pixels, rectangles cliquables, et sa
//! traduction en quads et textes pour le renderer. Pur, sans GPU.

use rustty_config::CloseButtonStyle;
use unicode_width::UnicodeWidthStr;

use crate::color::{Palette, Rgba};
use crate::font::CellMetrics;
use crate::frame::{Chrome, ChromeQuad, ChromeText, PixelRect};

pub const TAB_BAR_PADDING: u32 = 2;
/// Cellules occupées par le bouton de fermeture : espace, ◖, ✕, ◗.
const CLOSE_CELLS: u32 = 4;
const NEW_TAB_CELLS: u32 = 3;

#[derive(Clone, Debug, PartialEq)]
pub struct TabBarStyle {
    pub background: Rgba,
    pub active_background: Rgba,
    pub inactive_background: Rgba,
    pub active_foreground: Rgba,
    pub inactive_foreground: Rgba,
    pub close_foreground: Rgba,
    pub close_background: Rgba,
    pub close_hover_foreground: Rgba,
    pub close_hover_background: Rgba,
    pub show_close_button: bool,
}

impl TabBarStyle {
    pub fn from_config(palette: &Palette, close: &CloseButtonStyle, show_close_button: bool) -> Self {
        Self {
            background: palette.ansi[8].dim(0.5),
            active_background: palette.background,
            inactive_background: palette.ansi[0],
            active_foreground: palette.foreground,
            inactive_foreground: palette.ansi[7],
            close_foreground: Rgba::from_rgb(close.foreground),
            close_background: Rgba::from_rgb(close.background),
            close_hover_foreground: Rgba::from_rgb(close.hover_foreground),
            close_hover_background: Rgba::from_rgb(close.hover_background),
            show_close_button,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabSpec<'a> {
    pub title: &'a str,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverTarget {
    None,
    Tab(usize),
    CloseButton(usize),
    NewTabButton,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabRect {
    pub rect: PixelRect,
    pub close: Option<PixelRect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabBarLayout {
    pub bar: PixelRect,
    pub tabs: Vec<TabRect>,
    pub new_tab: PixelRect,
}

impl TabBarLayout {
    pub fn hit_test(&self, x: u32, y: u32) -> HoverTarget {
        let inside = |r: PixelRect| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height;
        if !inside(self.bar) {
            return HoverTarget::None;
        }
        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.close.is_some_and(inside) {
                return HoverTarget::CloseButton(i);
            }
            if inside(tab.rect) {
                return HoverTarget::Tab(i);
            }
        }
        if self.new_tab.width > 0 && inside(self.new_tab) {
            return HoverTarget::NewTabButton;
        }
        HoverTarget::None
    }
}

pub fn tab_bar_height(metrics: CellMetrics) -> u32 {
    metrics.height + 2 * TAB_BAR_PADDING
}

fn title_cells(title: &str) -> u32 {
    title.width() as u32
}

pub fn layout_tab_bar(viewport_width: u32, y: u32, tabs: &[TabSpec], style: &TabBarStyle, metrics: CellMetrics) -> TabBarLayout {
    let (cw, bar_h) = (metrics.width, tab_bar_height(metrics));
    let close_cells = if style.show_close_button { CLOSE_CELLS } else { 0 };
    let mut x = 0u32;
    let mut out = Vec::new();
    for tab in tabs {
        let max_title = (viewport_width / cw).saturating_sub(2 + close_cells);
        let title = title_cells(tab.title).min(max_title);
        let cells = 1 + title + close_cells + 1;
        let width = cells * cw;
        if x + width > viewport_width {
            break;
        }
        let close = style.show_close_button.then(|| PixelRect::new(x + (1 + title) * cw, y + TAB_BAR_PADDING, close_cells * cw, metrics.height));
        out.push(TabRect { rect: PixelRect::new(x, y, width, bar_h), close });
        x += width;
    }
    let new_w = NEW_TAB_CELLS * cw;
    let new_tab = if x + new_w <= viewport_width { PixelRect::new(x, y, new_w, bar_h) } else { PixelRect::new(x, y, 0, bar_h) };
    TabBarLayout { bar: PixelRect::new(0, y, viewport_width, bar_h), tabs: out, new_tab }
}

/// Tronque `title` à `max_cells` colonnes, avec `…` si nécessaire.
fn fit_title(title: &str, max_cells: u32) -> String {
    if title_cells(title) <= max_cells {
        return title.to_string();
    }
    let mut out = String::new();
    for ch in title.chars() {
        if title_cells(&out) + 2 > max_cells {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

pub fn tab_bar_chrome(layout: &TabBarLayout, tabs: &[TabSpec], style: &TabBarStyle, metrics: CellMetrics, hover: HoverTarget) -> Chrome {
    let cw = metrics.width;
    let mut chrome = Chrome { quads: vec![ChromeQuad { rect: layout.bar, color: style.background }], texts: Vec::new() };
    let text_y = layout.bar.y + TAB_BAR_PADDING;
    for (i, (tab_rect, spec)) in layout.tabs.iter().zip(tabs).enumerate() {
        let (bg, fg) = if spec.active { (style.active_background, style.active_foreground) } else { (style.inactive_background, style.inactive_foreground) };
        chrome.quads.push(ChromeQuad { rect: tab_rect.rect, color: bg });
        let close_cells = if tab_rect.close.is_some() { CLOSE_CELLS } else { 0 };
        let max_title = (tab_rect.rect.width / cw).saturating_sub(2 + close_cells);
        chrome.texts.push(ChromeText { x: tab_rect.rect.x + cw, y: text_y, text: fit_title(spec.title, max_title), color: fg });
        if let Some(close) = tab_rect.close {
            let hovered = hover == HoverTarget::CloseButton(i);
            let (button_bg, button_fg) = if hovered { (style.close_hover_background, style.close_hover_foreground) } else { (style.close_background, style.close_foreground) };
            let x0 = close.x + cw;
            chrome.texts.push(ChromeText { x: x0, y: close.y, text: "\u{E0B6}".into(), color: button_bg });
            chrome.quads.push(ChromeQuad { rect: PixelRect::new(x0 + cw, close.y, cw, close.height), color: button_bg });
            chrome.texts.push(ChromeText { x: x0 + cw, y: close.y, text: "✕".into(), color: button_fg });
            chrome.texts.push(ChromeText { x: x0 + 2 * cw, y: close.y, text: "\u{E0B4}".into(), color: button_bg });
        }
    }
    if layout.new_tab.width > 0 {
        chrome.texts.push(ChromeText { x: layout.new_tab.x + cw, y: text_y, text: "+".into(), color: style.inactive_foreground });
    }
    chrome
}
```

Géométrie vérifiée à la main (`cw = 10`, barre 24 px) : onglet 0 « 1: sh » = 1 + 5 + 4 + 1 = 11 cellules = 110 px ; le bouton occupe les cellules `[espace, ◖, ✕, ◗]` à partir de `close.x = (1 + 5) × 10 = 60` : espace en 60, ◖ en 70, quad et ✕ en 80, ◗ en 90 ; `hit_test(75, 10)` tombe dans `[60, 100)` → `CloseButton(0)`. Onglet 1 « 2: vim » en `x = 110`, 12 cellules = 120 px, `close.x = 110 + 70 = 180`, ✕ en 200. Le « + » suit en 230 (texte en 240). Sans bouton : 7 cellules = 70 px. Dans le test hors écran, le pixel sondé `close.x + 15` (= 195 pour l'onglet 1) est dans le demi-disque gauche, plein de la couleur de survol.

`lib.rs` : `pub mod chrome;` et `pub use chrome::{HoverTarget, TAB_BAR_PADDING, TabBarLayout, TabBarStyle, TabRect, TabSpec, layout_tab_bar, tab_bar_chrome, tab_bar_height};`.

`README.md` : remplacer le bloc « Statut » par :

```markdown
> Statut : fondations. Cinq crates sont fonctionnelles et testées : `rustty-vt`
> (émulation), `rustty-layout` (onglets et divisions), `rustty-config` (TOML),
> `rustty-pty` (shell dans un pseudo-terminal) et `rustty-render` (rendu wgpu,
> testé hors écran sur les trois OS). Reste le binaire qui les assemble.
```

et ajouter à la fin de la section « Développement » :

```markdown
Les tests de rendu comparent des images de référence (`crates/rustty-render/tests/golden/`)
produites avec la police embarquée DejaVu Sans Mono. Pour les régénérer après un
changement voulu du rendu : `UPDATE_GOLDEN=1 cargo test -p rustty-render --test offscreen`,
puis vérifier les PNG à l'œil avant de les committer.
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tous `ok` ; l'image `tab_bar.png` est générée au premier passage, à inspecter (deux onglets, l'actif plus clair, boutons ✕ sur pastilles rouges, celui du second onglet en rouge clair de survol, « + » à droite), puis le second passage compare.

- [ ] **Step 5: Version, CHANGELOG, commit, push**

`version = "0.1.0-alpha.49"` ; CHANGELOG :

```markdown
## 0.1.0-alpha.49 — 2026-10-08 · « Barre d'onglets »

- `rustty-render` : disposition de la barre d'onglets, boutons de fermeture ✕ sur pastille arrondie avec couleurs de survol, bouton « + », test de clic ; image de référence.
- README : statut des cinq crates et régénération des images de référence.
```

```bash
git add Cargo.toml crates/rustty-render/src crates/rustty-render/tests/offscreen.rs crates/rustty-render/tests/golden/tab_bar.png README.md CHANGELOG.md
git commit -m "rustty-render : barre d'onglets avec boutons de fermeture et survol (0.1.0-alpha.49)"
git push
```

Vérifier le push (`git status -sb` sans `[ahead N]`), puis attendre la CI verte sur les trois OS. Si la CI Windows ou macOS échoue sur une image de référence avec une différence moyenne juste au-dessus de 1,0, lire la valeur rapportée et **ne pas** relever le seuil sans comprendre : une différence d'arrondi GPU reste sous 0,3 ; au-delà, c'est un vrai écart de rendu (police, position) à corriger.

---

## Suite

Plan suivant : `docs/superpowers/plans/2026-10-08-binaire.md` (crate `rustty` : winit, assemblage, entrées, chrome, opacité, rechargement de la config), à rédiger une fois ce plan exécuté et la CI verte.
