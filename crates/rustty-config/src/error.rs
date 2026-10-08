//! Erreurs de configuration, toujours positionnées : ligne et colonne pour
//! une erreur de syntaxe ou de clé, nom du champ pour une valeur hors bornes.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("ligne {line}, colonne {column} : {message}")]
    Parse {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("valeur invalide pour {field} : {reason}")]
    Invalid { field: &'static str, reason: String },
    #[error("impossible de lire {path} : {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl ConfigError {
    /// Convertit une erreur `toml` en position (ligne, colonne) 1-indexée.
    pub(crate) fn from_toml(source: &str, err: &toml::de::Error) -> Self {
        let offset = err.span().map_or(0, |s| s.start).min(source.len());
        let before = &source[..offset];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, str::len) + 1;
        Self::Parse {
            line,
            column,
            message: err.message().to_string(),
        }
    }
}
