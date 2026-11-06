use std::path::{Path, PathBuf};

use super::*;

/// Dossier temporaire unique, supprimé à la fin du test.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("rustty-external-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn editor_falls_back_from_key_to_env_to_opener() {
    assert_eq!(
        editor_command("code --wait", Some("vim"), Some("nano")),
        ["code", "--wait"]
    );
    assert_eq!(editor_command("", Some("vim"), Some("nano")), ["vim"]);
    assert_eq!(editor_command("", None, Some("nano -w")), ["nano", "-w"]);
    assert_eq!(editor_command("", None, None), opener());
}

#[test]
fn blank_values_are_skipped() {
    assert_eq!(editor_command("  ", Some(" "), Some("")), opener());
    assert_eq!(file_manager_command(" \t"), opener());
    assert_eq!(
        file_manager_command("nautilus --new-window"),
        ["nautilus", "--new-window"]
    );
}

#[test]
fn opener_matches_the_platform() {
    if cfg!(target_os = "linux") {
        assert_eq!(opener(), ["xdg-open"]);
    }
    assert!(!opener().is_empty());
}

#[test]
fn config_file_is_created_once_then_left_alone() {
    let dir = TempDir::new("config");
    let path = dir.0.join("rustty.toml");
    ensure_config_file(&path).unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        rustty_config::DEFAULT_TOML
    );
    std::fs::write(&path, "[titlebar]\nheight = 20\n").unwrap();
    ensure_config_file(&path).unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "[titlebar]\nheight = 20\n"
    );
}

#[test]
fn folder_to_open_falls_back_to_home() {
    let home = PathBuf::from("/home/x");
    assert_eq!(
        folder_to_open(Some(PathBuf::from("/tmp")), Some(home.clone())),
        Some(PathBuf::from("/tmp"))
    );
    assert_eq!(folder_to_open(None, Some(home.clone())), Some(home));
    assert_eq!(folder_to_open(None, None), None);
}

#[test]
fn spawning_a_missing_program_is_an_error_not_a_panic() {
    assert!(spawn_detached(&["rustty-programme-inexistant".to_string()], Path::new("/")).is_err());
}

#[test]
fn an_empty_command_is_an_error() {
    assert!(spawn_detached(&[], Path::new("/")).is_err());
}

#[cfg(unix)]
#[test]
fn spawning_an_existing_program_returns_at_once() {
    let started = std::time::Instant::now();
    spawn_detached(&["sleep".to_string(), "2".to_string()], Path::new("/")).unwrap();
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "le lancement n'attend pas la fin du programme"
    );
}
