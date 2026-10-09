//! Point d'entrée : arguments, journalisation, configuration, puis la fenêtre.

mod accent;
mod app;
mod appearance;
mod banner;
mod cli;
mod config_watch;
mod desktop;
mod effects;
mod events;
mod font_zoom;
mod geometry;
mod gpu_surface;
mod input;
mod keyboard;
mod model;
mod mouse;
mod pane_fonts;
mod relayout;
mod rename;
mod render;
mod render_frame;
mod renderers;
mod tab;
mod term_window;
mod title;
mod window_state;
mod workspace;

use std::process::ExitCode;

use rustty_config::Config;
use tracing_subscriber::EnvFilter;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args = match cli::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("rustty : {e}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    match args.command {
        Some(cli::Command::Version) => {
            println!("rustty {VERSION}");
            return ExitCode::SUCCESS;
        }
        Some(cli::Command::Help) => {
            print!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        Some(cli::Command::InstallDesktop) => return install_desktop(),
        Some(cli::Command::InitConfig) => return init_config(args.config.as_deref()),
        None => {}
    }
    init_tracing();
    let (config, error) = match config_watch::reload(args.config.as_deref()) {
        config_watch::ReloadOutcome::Applied(c) => (*c, None),
        config_watch::ReloadOutcome::Rejected(e) => (Config::default(), Some(e)),
    };
    match app::run(config, error, args.config, args.hold) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rustty : {e:#}");
            ExitCode::from(1)
        }
    }
}

/// Les dépendances ne parlent qu'en cas d'erreur (arboard prévient par exemple
/// à chaque démarrage sous GNOME qu'il se replie sur le presse-papiers X11).
const DEFAULT_LOG_FILTER: &str = "error,rustty=warn";

/// Écrit la configuration d'exemple au chemin demandé ou par défaut.
fn init_config(path: Option<&std::path::Path>) -> ExitCode {
    let Some(path) = path
        .map(std::path::Path::to_path_buf)
        .or_else(Config::default_path)
    else {
        eprintln!("rustty : impossible de déterminer le dossier de configuration");
        return ExitCode::from(1);
    };
    match config_watch::init_config(&path) {
        Ok(()) => {
            println!("{}", path.display());
            println!("Configuration écrite : les raccourcis sont dans la section [keys].");
            ExitCode::SUCCESS
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            eprintln!(
                "rustty : {} existe déjà, rien n'a été écrit",
                path.display()
            );
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("rustty : impossible d'écrire {} : {e}", path.display());
            ExitCode::from(1)
        }
    }
}

/// Installe le lanceur et les icônes pour l'utilisateur courant.
fn install_desktop() -> ExitCode {
    if !cfg!(target_os = "linux") {
        println!("Rien à installer : l'icône est déjà embarquée dans l'exécutable sur ce système.");
        return ExitCode::SUCCESS;
    }
    let (Some(home), Ok(exec)) = (desktop::data_home(), std::env::current_exe()) else {
        eprintln!("rustty : impossible de déterminer ~/.local/share ou le chemin de l'exécutable");
        return ExitCode::from(1);
    };
    match desktop::install(&home, &exec) {
        Ok(files) => {
            for f in files {
                println!("{}", f.display());
            }
            println!("Lanceur installé : rustty apparaît dans les menus avec son icône.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rustty : installation impossible : {e}");
            ExitCode::from(1)
        }
    }
}

/// Niveau via `RUSTTY_LOG` (syntaxe `tracing-subscriber`), `warn` par défaut.
fn init_tracing() {
    let filter = EnvFilter::try_from_env("RUSTTY_LOG")
        .unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_log_filter_is_valid_and_quiet_for_dependencies() {
        assert!(EnvFilter::try_new(DEFAULT_LOG_FILTER).is_ok());
        assert!(
            DEFAULT_LOG_FILTER.starts_with("error,"),
            "les dépendances ne parlent qu'en cas d'erreur"
        );
        assert!(DEFAULT_LOG_FILTER.contains("rustty=warn"));
    }
}
