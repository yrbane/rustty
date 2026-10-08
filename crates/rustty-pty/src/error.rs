//! Erreurs du pseudo-terminal, en français, avec le programme concerné.

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("impossible d'ouvrir un pseudo-terminal : {0}")]
    Open(String),
    #[error("impossible de lancer « {program} » : {reason}")]
    Spawn { program: String, reason: String },
    #[error("erreur d'entrée-sortie sur le pseudo-terminal : {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_error_names_the_program() {
        let e = PtyError::Spawn {
            program: "nope".into(),
            reason: "introuvable".into(),
        };
        assert!(e.to_string().contains("nope"));
    }
}
