//! Point d'entrée : arguments, journalisation, configuration, puis la fenêtre.

mod app;
mod banner;
mod cli;
mod config_watch;
mod effects;
mod events;
mod geometry;
mod gpu_surface;
mod input;
mod keyboard;
mod model;
mod mouse;
mod render_frame;
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

/// Niveau via `RUSTTY_LOG` (syntaxe `tracing-subscriber`), `warn` par défaut.
fn init_tracing() {
    let filter = EnvFilter::try_from_env("RUSTTY_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
