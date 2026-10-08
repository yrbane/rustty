//! Le programme à lancer dans le pseudo-terminal et l'environnement qu'on lui donne.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shell {
    pub program: String,
    pub args: Vec<String>,
}

impl Shell {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
        }
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
    vec![
        ("TERM".into(), "xterm-256color".into()),
        ("COLORTERM".into(), "truecolor".into()),
    ]
}

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
