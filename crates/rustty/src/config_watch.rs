//! Rechargement à chaud : surveillance du fichier et rechargement qui ne
//! casse jamais la session (une config invalide garde l'ancienne).

use std::path::{Path, PathBuf};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rustty_config::Config;

use crate::events::{UserEvent, Waker};

pub enum ReloadOutcome {
    Applied(Box<Config>),
    Rejected(String),
}

/// Recharge la configuration ; un fichier absent donne les défauts.
pub fn reload(path: Option<&Path>) -> ReloadOutcome {
    let result = match path {
        Some(p) if p.exists() => Config::load(p),
        Some(_) => Ok(Config::default()),
        None => Config::load_default(),
    };
    match result {
        Ok(c) => ReloadOutcome::Applied(Box::new(c)),
        Err(e) => ReloadOutcome::Rejected(e.to_string()),
    }
}

pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
}

impl ConfigWatcher {
    pub fn start(path: &Path, waker: Waker) -> Option<Self> {
        let dir = path.parent().filter(|d| d.is_dir())?.to_path_buf();
        let name = path.file_name()?.to_os_string();
        let watched: PathBuf = dir.join(&name);
        let mut watcher = notify::recommended_watcher(
            move |result: notify::Result<notify::Event>| match result {
                Ok(event)
                    if event
                        .paths
                        .iter()
                        .any(|p| p.file_name() == Some(name.as_os_str())) =>
                {
                    waker.wake(UserEvent::ConfigChanged)
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("surveillance de {} : {e}", watched.display()),
            },
        )
        .map_err(|e| tracing::warn!("surveillance de la configuration impossible : {e}"))
        .ok()?;
        watcher
            .watch(&dir, RecursiveMode::NonRecursive)
            .map_err(|e| tracing::warn!("surveillance de {} : {e}", dir.display()))
            .ok()?;
        Some(Self { _watcher: watcher })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::mpsc::channel;
    use std::time::Duration;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rustty-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn valid_file_is_applied() {
        let dir = temp_dir("valid");
        let path = dir.join("rustty.toml");
        std::fs::write(&path, "[window]\nopacity = 0.5\n").unwrap();
        match reload(Some(&path)) {
            ReloadOutcome::Applied(c) => assert_eq!(c.window.opacity, 0.5),
            ReloadOutcome::Rejected(e) => panic!("{e}"),
        }
    }

    #[test]
    fn invalid_reload_keeps_the_old_config_and_explains() {
        let dir = temp_dir("invalid");
        let path = dir.join("rustty.toml");
        std::fs::write(&path, "[window]\nopacity = 7\n").unwrap();
        match reload(Some(&path)) {
            ReloadOutcome::Rejected(message) => assert!(message.contains("opacity"), "{message}"),
            ReloadOutcome::Applied(_) => panic!("une opacité de 7 doit être refusée"),
        }
    }

    #[test]
    fn missing_file_means_defaults_without_banner() {
        let dir = temp_dir("missing");
        match reload(Some(&dir.join("absent.toml"))) {
            ReloadOutcome::Applied(c) => {
                assert_eq!(c.window.opacity, Config::default().window.opacity)
            }
            ReloadOutcome::Rejected(e) => panic!("{e}"),
        }
    }

    #[test]
    fn watcher_wakes_on_file_change() {
        let dir = temp_dir("watch");
        let path = dir.join("rustty.toml");
        std::fs::write(&path, "").unwrap();
        let (tx, rx) = channel();
        let _watcher = ConfigWatcher::start(&path, Arc::new(tx)).expect("répertoire existant");
        std::thread::sleep(Duration::from_millis(300));
        std::fs::write(&path, "[window]\nopacity = 0.8\n").unwrap();
        let got = rx.recv_timeout(Duration::from_secs(15));
        assert_eq!(got.ok(), Some(UserEvent::ConfigChanged));
    }

    #[test]
    fn watcher_on_a_missing_directory_is_none() {
        let path = std::env::temp_dir()
            .join("rustty-nope-dir-xyz")
            .join("rustty.toml");
        let (tx, _rx) = channel();
        assert!(ConfigWatcher::start(&path, Arc::new(tx)).is_none());
    }
}
