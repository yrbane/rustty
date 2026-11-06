//! Programmes externes lancés par la barre de titre : l'éditeur du fichier de
//! configuration et l'explorateur du dossier courant. Résolution pure de la
//! commande, puis lancement détaché qui ne bloque jamais l'interface.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// L'ouvreur du bureau : il choisit l'application associée à la cible.
pub fn opener() -> Vec<String> {
    let parts: &[&str] = if cfg!(target_os = "macos") {
        &["open"]
    } else if cfg!(windows) {
        // ShellExecute sur la cible, sans console qui clignote ni
        // échappement de cmd (`&`, `^` dans un chemin).
        &["rundll32", "url.dll,FileProtocolHandler"]
    } else {
        &["xdg-open"]
    };
    parts.iter().map(|s| s.to_string()).collect()
}

/// La clé `editor` si elle n'est pas blanche, telle quelle (un éditeur de
/// terminal y est un choix explicite) ; sinon `$VISUAL` puis `$EDITOR`, en
/// sautant les éditeurs de terminal (`vim`, `nano`…) qui, lancés détachés
/// sans console, ne s'ouvriraient jamais ; sinon l'ouvreur du bureau.
/// Les valeurs sont découpées en mots, guillemets doubles compris.
pub fn editor_command(configured: &str, visual: Option<&str>, editor: Option<&str>) -> Vec<String> {
    split_command(configured)
        .or_else(|| {
            [visual, editor]
                .into_iter()
                .flatten()
                .filter_map(split_command)
                .find(|cmd| !is_terminal_editor(&cmd[0]))
        })
        .unwrap_or_else(opener)
}

/// Éditeurs qui vivent dans le terminal : sans console, ils ne s'ouvrent pas.
/// `emacs` n'y est pas : il ouvre une fenêtre par défaut.
const TERMINAL_EDITORS: [&str; 8] = ["vi", "vim", "nvim", "nano", "micro", "hx", "helix", "ed"];

/// Le programme (chemin, casse et `.exe` ignorés) est-il un éditeur de terminal ?
pub fn is_terminal_editor(program: &str) -> bool {
    let name = Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program)
        .to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    TERMINAL_EDITORS.contains(&name)
}

/// La clé `file_manager` découpée sur les blancs ; sinon l'ouvreur du bureau.
pub fn file_manager_command(configured: &str) -> Vec<String> {
    split_command(configured).unwrap_or_else(opener)
}

/// Découpe sur les blancs ; des guillemets doubles groupent un mot avec ses
/// espaces (`"C:\Program Files\Code\code.exe" --wait`) et sont retirés ;
/// `""` est un argument vide. `None` pour une valeur blanche ou un
/// programme vide.
fn split_command(value: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut had_quotes = false;
    for ch in value.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                had_quotes = true;
            }
            c if c.is_whitespace() && !quoted => {
                if !word.is_empty() || had_quotes {
                    parts.push(std::mem::take(&mut word));
                }
                had_quotes = false;
            }
            c => word.push(c),
        }
    }
    if !word.is_empty() || had_quotes {
        parts.push(word);
    }
    (!parts.first().is_none_or(String::is_empty)).then_some(parts)
}

/// Le fichier de configuration existe ensuite : créé depuis le modèle s'il
/// manquait, jamais réécrit s'il existe.
pub fn ensure_config_file(path: &Path) -> std::io::Result<()> {
    match crate::config_watch::init_config(path) {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        other => other,
    }
}

/// Le dossier courant du panneau s'il se lit, sinon le dossier personnel.
pub fn folder_to_open(cwd: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    cwd.or(home)
}

/// Lance `command[0]` avec `command[1..]` puis `target`, sans attendre et
/// sans partager les flux ; un thread moissonne l'enfant pour éviter un
/// zombie. L'échec de lancement est rendu, celui du programme est ignoré.
pub fn spawn_detached(command: &[String], target: &Path) -> std::io::Result<()> {
    let Some((program, args)) = command.split_first() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "commande vide",
        ));
    };
    let mut child = Command::new(program)
        .args(args)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
#[path = "external_tests.rs"]
mod tests;
