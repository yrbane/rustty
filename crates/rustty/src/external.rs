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
        &["cmd", "/C", "start", ""]
    } else {
        &["xdg-open"]
    };
    parts.iter().map(|s| s.to_string()).collect()
}

/// Première valeur non blanche parmi la clé `editor`, `$VISUAL` et
/// `$EDITOR`, découpée sur les blancs ; sinon l'ouvreur du bureau.
pub fn editor_command(configured: &str, visual: Option<&str>, editor: Option<&str>) -> Vec<String> {
    [Some(configured), visual, editor]
        .into_iter()
        .flatten()
        .find_map(split_command)
        .unwrap_or_else(opener)
}

/// La clé `file_manager` découpée sur les blancs ; sinon l'ouvreur du bureau.
pub fn file_manager_command(configured: &str) -> Vec<String> {
    split_command(configured).unwrap_or_else(opener)
}

/// `None` pour une valeur blanche.
fn split_command(value: &str) -> Option<Vec<String>> {
    let parts: Vec<String> = value.split_whitespace().map(str::to_string).collect();
    (!parts.is_empty()).then_some(parts)
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
