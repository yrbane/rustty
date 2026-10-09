//! Arguments de la ligne de commande, sans dépendance : trois options suffisent.

use std::path::PathBuf;

pub const USAGE: &str = "\
Usage : rustty [OPTIONS]

Options :
  --config <chemin>   fichier de configuration TOML (défaut : ~/.config/rustty/rustty.toml)
  --hold              garder le panneau ouvert quand le shell se termine
  --install-desktop   installer le lanceur et l'icône (Linux, ~/.local/share)
  -V, --version       afficher la version
  -h, --help          afficher cette aide
";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Args {
    pub config: Option<PathBuf>,
    pub hold: bool,
    pub command: Option<Command>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Version,
    Help,
    InstallDesktop,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CliError {
    #[error("l'option {0} attend une valeur")]
    MissingValue(&'static str),
    #[error("option inconnue : {0}")]
    Unknown(String),
}

/// Analyse les arguments (sans le nom du programme).
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, CliError> {
    let mut out = Args::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--config" => {
                let value = it.next().ok_or(CliError::MissingValue("--config"))?;
                out.config = Some(PathBuf::from(value));
            }
            "--hold" => out.hold = true,
            "--version" | "-V" => out.command = Some(Command::Version),
            "--help" | "-h" => out.command = Some(Command::Help),
            "--install-desktop" => out.command = Some(Command::InstallDesktop),
            other => match other.strip_prefix("--config=") {
                Some(value) if !value.is_empty() => out.config = Some(PathBuf::from(value)),
                Some(_) => return Err(CliError::MissingValue("--config")),
                None => return Err(CliError::Unknown(other.to_string())),
            },
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_desktop_flag() {
        assert_eq!(
            parse_strs(&["--install-desktop"]).unwrap().command,
            Some(Command::InstallDesktop)
        );
        assert!(USAGE.contains("--install-desktop"));
    }

    fn parse_strs(args: &[&str]) -> Result<Args, CliError> {
        parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn no_arguments_means_defaults() {
        let a = parse_strs(&[]).unwrap();
        assert_eq!(a.config, None);
        assert!(!a.hold);
        assert_eq!(a.command, None);
    }

    #[test]
    fn config_takes_a_path_in_both_spellings() {
        assert_eq!(
            parse_strs(&["--config", "/tmp/r.toml"]).unwrap().config,
            Some(PathBuf::from("/tmp/r.toml"))
        );
        assert_eq!(
            parse_strs(&["--config=/tmp/r.toml"]).unwrap().config,
            Some(PathBuf::from("/tmp/r.toml"))
        );
    }

    #[test]
    fn config_without_value_is_an_error() {
        assert_eq!(
            parse_strs(&["--config"]).unwrap_err(),
            CliError::MissingValue("--config")
        );
    }

    #[test]
    fn hold_version_and_help() {
        assert!(parse_strs(&["--hold"]).unwrap().hold);
        assert_eq!(
            parse_strs(&["--version"]).unwrap().command,
            Some(Command::Version)
        );
        assert_eq!(parse_strs(&["-V"]).unwrap().command, Some(Command::Version));
        assert_eq!(
            parse_strs(&["--help"]).unwrap().command,
            Some(Command::Help)
        );
        assert_eq!(parse_strs(&["-h"]).unwrap().command, Some(Command::Help));
    }

    #[test]
    fn unknown_option_names_itself() {
        let e = parse_strs(&["--bogus"]).unwrap_err();
        assert_eq!(e, CliError::Unknown("--bogus".into()));
        assert!(e.to_string().contains("--bogus"));
    }

    #[test]
    fn usage_mentions_every_option() {
        for opt in ["--config", "--hold", "--version", "--help"] {
            assert!(USAGE.contains(opt), "{opt}");
        }
    }
}
