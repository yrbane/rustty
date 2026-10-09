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

/// Recharge la configuration. Sans chemin explicite, un fichier absent donne
/// les défauts ; un chemin explicite absent est signalé (et les défauts
/// s'appliquent côté appelant).
pub fn reload(path: Option<&Path>) -> ReloadOutcome {
    let result = match path {
        Some(p) if p.exists() => Config::load(p),
        Some(p) => {
            return ReloadOutcome::Rejected(format!("{} introuvable", p.display()));
        }
        None => Config::load_default(),
    };
    match result {
        Ok(c) => ReloadOutcome::Applied(Box::new(c)),
        Err(e) => ReloadOutcome::Rejected(e.to_string()),
    }
}

/// Écrit la configuration d'exemple complète et commentée à `path`, jamais
/// par-dessus un fichier existant (`ErrorKind::AlreadyExists`). L'écriture
/// passe par un temporaire puis un lien dur (atomique, et qui échoue si la
/// destination existe) : jamais de fichier à moitié écrit.
pub fn init_config(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    publish(path, rustty_config::DEFAULT_TOML, |tmp, dest| {
        std::fs::hard_link(tmp, dest)
    })
}

/// Écrit `content` dans un temporaire voisin puis le publie avec `link`. Si
/// le système de fichiers refuse les liens durs (FAT, certains montages
/// réseau), repli sur une création exclusive : jamais d'écrasement.
fn publish(
    path: &Path,
    content: &str,
    link: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    use std::io::{ErrorKind, Write as _};
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let result = std::fs::write(&tmp, content).and_then(|()| match link(&tmp, path) {
        Err(e) if e.kind() != ErrorKind::AlreadyExists => std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .and_then(|mut f| f.write_all(content.as_bytes())),
        other => other,
    });
    let _ = std::fs::remove_file(&tmp);
    result
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

    #[test]
    fn init_config_writes_the_example_once() {
        let dir = temp_dir("init");
        let path = dir.join("sous/dossier/rustty.toml");
        init_config(&path).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            rustty_config::DEFAULT_TOML
        );
        assert!(
            matches!(reload(Some(&path)), ReloadOutcome::Applied(_)),
            "le fichier écrit est valide"
        );
    }

    #[test]
    fn init_config_never_overwrites() {
        let dir = temp_dir("init-existing");
        let path = dir.join("rustty.toml");
        std::fs::write(&path, "# à moi\n").unwrap();
        let err = init_config(&path).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# à moi\n");
    }
    #[test]
    fn init_config_leaves_no_temp_file() {
        let dir = temp_dir("init-tmp");
        init_config(&dir.join("rustty.toml")).unwrap();
        let existing = dir.join("rustty.toml");
        std::fs::write(&existing, "# à moi\n").unwrap();
        assert!(init_config(&existing).is_err());
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("rustty.toml")]);
    }

    #[cfg(unix)]
    #[test]
    fn unwritable_directory_is_an_error() {
        use std::os::unix::fs::PermissionsExt as _;
        // Root ignore les permissions : le test n'aurait aucun sens.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let dir = temp_dir("readonly");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = init_config(&dir.join("rustty.toml"));
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    }

    #[test]
    fn init_config_falls_back_when_hard_links_are_unsupported() {
        let dir = temp_dir("no-hardlink");
        let path = dir.join("rustty.toml");
        let no_links = |_: &Path, _: &Path| Err(std::io::Error::other("liens durs non gérés"));
        publish(&path, rustty_config::DEFAULT_TOML, no_links).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            rustty_config::DEFAULT_TOML
        );
        // Jamais d'écrasement, même en repli.
        let err = publish(&path, "autre", no_links).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            rustty_config::DEFAULT_TOML
        );
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[test]
    fn missing_explicit_config_is_reported() {
        let dir = temp_dir("explicit-missing");
        match reload(Some(&dir.join("absent.toml"))) {
            ReloadOutcome::Rejected(m) => {
                assert!(
                    m.contains("absent.toml") && m.contains("introuvable"),
                    "{m}"
                )
            }
            ReloadOutcome::Applied(_) => panic!("un chemin explicite absent doit être signalé"),
        }
    }

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
