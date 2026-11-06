//! Écriture ciblée dans le fichier de configuration : une seule clé change,
//! commentaires, ordre et formatage du reste sont conservés (`toml_edit`),
//! et le fichier est remplacé d'un bloc (temporaire puis renommage).

use std::fs;
use std::io;
use std::path::Path;

use rustty_config::Rgba8;
use rustty_render::Rgba;
use toml_edit::{DocumentMut, TomlError};

use crate::external;

/// « Par défaut » du nuancier : `[titlebar] background = <color>` dans le
/// fichier `path`, créé depuis le modèle s'il manquait, le reste intact.
pub fn save_background(path: &Path, color: Rgba8) -> io::Result<()> {
    external::ensure_config_file(path)?;
    let text = fs::read_to_string(path)?;
    let updated = with_titlebar_background(&text, color).map_err(io::Error::other)?;
    write_atomically(path, &updated)
}

/// Le texte TOML avec `[titlebar] background = "<couleur>"`, tout le reste
/// intact. La section est créée si elle manque ; une table en ligne reste en
/// ligne. Un texte invalide est rendu tel quel en erreur, rien n'est deviné.
pub fn with_titlebar_background(toml: &str, color: Rgba8) -> Result<String, TomlError> {
    let mut doc: DocumentMut = toml.parse()?;
    if doc.get("titlebar").is_none() {
        doc["titlebar"] = toml_edit::table();
    }
    doc["titlebar"]["background"] = toml_edit::value(color.to_string());
    if let Some(inline) = doc["titlebar"].as_inline_table_mut() {
        // La clé ajoutée hérite d'espaces mal placés : on remet en forme la
        // seule table en ligne visée.
        inline.fmt();
    }
    Ok(doc.to_string())
}

/// Remplace `path` d'un bloc : le contenu va dans un fichier temporaire du
/// même dossier, renommé par-dessus ensuite ; un échec ne laisse ni fichier
/// à moitié écrit ni temporaire.
pub fn write_atomically(path: &Path, contents: &str) -> io::Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("chemin sans nom de fichier"))?;
    let mut tmp_name = name.to_os_string();
    tmp_name.push(format!(".tmp-{}", std::process::id()));
    let tmp = path.with_file_name(tmp_name);
    let written = fs::write(&tmp, contents).and_then(|()| fs::rename(&tmp, path));
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

/// La couleur de rendu en couleur de configuration (`#rrggbb[aa]`).
pub fn rgba8_of(color: Rgba) -> Rgba8 {
    let [r, g, b, a] = color.to_u8();
    Rgba8::new(r, g, b, a)
}

#[cfg(test)]
#[path = "config_write_tests.rs"]
mod tests;
