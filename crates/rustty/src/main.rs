//! Point d'entrée : arguments, journalisation, configuration, puis la fenêtre.

mod cli;
// Consommé par app.rs (tâche 14).
#[allow(unused)]
mod keyboard;
// Consommé par app.rs (tâche 14).
#[allow(unused)]
mod mouse;
// Consommé par app.rs (tâche 14).
#[allow(unused)]
mod geometry;
// Consommés par app.rs (tâche 14).
#[allow(unused)]
mod banner;
#[allow(unused)]
mod events;
#[allow(unused)]
mod model;
#[allow(unused)]
mod tab;
#[allow(unused)]
mod term_window;
#[allow(unused)]
mod title;
#[allow(unused)]
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
        None => {}
    }
    init_tracing();
    let (config, error) = load_config(args.config.as_deref());
    if let Some(e) = &error {
        tracing::warn!("configuration : {e} — valeurs par défaut utilisées");
    }
    println!(
        "rustty {VERSION} — police {} {}px, opacité {}",
        config.font.family, config.font.size, config.window.opacity
    );
    ExitCode::SUCCESS
}

/// Niveau via `RUSTTY_LOG` (syntaxe `tracing-subscriber`), `warn` par défaut.
fn init_tracing() {
    let filter = EnvFilter::try_from_env("RUSTTY_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

/// La configuration demandée, ou les défauts avec le message d'erreur à afficher.
fn load_config(path: Option<&std::path::Path>) -> (Config, Option<String>) {
    let result = match path {
        Some(p) => Config::load(p),
        None => Config::load_default(),
    };
    match result {
        Ok(c) => (c, None),
        Err(e) => (Config::default(), Some(e.to_string())),
    }
}
