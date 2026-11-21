use std::path::PathBuf;

use super::*;

/// Le premier éditeur essayé.
fn editor_command(configured: &str, visual: Option<&str>, editor: Option<&str>) -> Vec<String> {
    editor_candidates(configured, visual, editor).swap_remove(0)
}

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
    assert_eq!(editor_command("", Some("gedit"), Some("kate")), ["gedit"]);
    assert_eq!(editor_command("", None, Some("subl -w")), ["subl", "-w"]);
    assert_eq!(editor_command("", None, None), text_opener());
}

#[test]
fn quoted_programs_keep_their_spaces() {
    assert_eq!(
        editor_command("\"C:\\Program Files\\Code\\code.exe\" --wait", None, None),
        ["C:\\Program Files\\Code\\code.exe", "--wait"]
    );
    assert_eq!(
        editor_command("\"\"", None, None),
        text_opener(),
        "guillemets vides = blanc"
    );
}

#[test]
fn terminal_editors_yield_to_the_desktop_opener() {
    assert!(is_terminal_editor("/usr/bin/vim"));
    assert!(is_terminal_editor("nvim.exe"));
    assert!(is_terminal_editor("VIM.EXE"), "casse de Windows");
    assert!(!is_terminal_editor("code"));
    assert!(
        !is_terminal_editor("emacs"),
        "emacs ouvre une fenêtre par défaut"
    );
    assert_eq!(
        editor_command("", Some("vim"), Some("nano -w")),
        text_opener()
    );
    assert_eq!(editor_command("", Some("vim"), Some("gedit")), ["gedit"]);
}

#[test]
fn the_editor_key_is_always_respected() {
    // La clé est un choix explicite : un éditeur de terminal y est lancé tel
    // quel (à l'utilisateur de le vouloir), seul l'environnement est filtré.
    assert_eq!(editor_command("vim", Some("gedit"), None), ["vim"]);
    assert_eq!(editor_command("nano -w", None, None), ["nano", "-w"]);
}

#[test]
fn blank_values_are_skipped() {
    assert_eq!(editor_command("  ", Some(" "), Some("")), text_opener());
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
    if cfg!(windows) {
        // Pas de `cmd /C start` : ni console qui clignote, ni échappement.
        assert_eq!(opener(), ["rundll32", "url.dll,FileProtocolHandler"]);
    }
    assert!(!opener().is_empty());
}

#[test]
fn empty_quoted_arguments_are_kept_but_a_lone_one_is_blank() {
    assert_eq!(editor_command("code \"\"", None, None), ["code", ""]);
    assert_eq!(editor_command("\"\"", None, None), text_opener());
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
    assert!(
        spawn_detached(
            &["rustty-programme-inexistant".to_string()],
            std::ffi::OsStr::new("/")
        )
        .is_err()
    );
}

#[test]
fn an_empty_command_is_an_error() {
    assert!(spawn_detached(&[], std::ffi::OsStr::new("/")).is_err());
}

#[cfg(unix)]
#[test]
fn spawning_an_existing_program_returns_at_once() {
    let started = std::time::Instant::now();
    spawn_detached(
        &["sleep".to_string(), "2".to_string()],
        std::ffi::OsStr::new("/"),
    )
    .unwrap();
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "le lancement n'attend pas la fin du programme"
    );
}

#[test]
fn spawn_detached_takes_any_os_string_target() {
    // Un programme qui n'existe pas : l'erreur vient du lancement, pas du type de cible.
    let err = spawn_detached(
        &["rustty-programme-inexistant".to_string()],
        std::ffi::OsStr::new("https://x"),
    )
    .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn candidates_go_from_the_key_to_the_env_to_the_text_opener() {
    assert_eq!(
        editor_candidates("code", Some("vim"), Some("subl -w")),
        [
            vec!["code".to_string()],
            vec!["subl".to_string(), "-w".to_string()],
            text_opener(),
        ]
    );
    assert_eq!(editor_candidates("", None, None), [text_opener()]);
    assert_eq!(
        editor_candidates("code", Some("code"), None),
        [vec!["code".to_string()], text_opener()]
    );
}

#[test]
fn text_opener_asks_macos_for_the_text_editor() {
    if cfg!(target_os = "macos") {
        assert_eq!(text_opener(), ["open", "-t"]);
    } else {
        assert_eq!(text_opener(), opener());
    }
}

#[cfg(unix)]
#[test]
fn a_missing_editor_yields_to_the_next_one() {
    let missing = vec!["rustty-editeur-absent".to_string()];
    let present = vec!["true".to_string()];
    let target = std::ffi::OsStr::new("x");
    spawn_first(&[missing.clone(), present], target).unwrap();
    let err = spawn_first(&[missing], target).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert!(spawn_first(&[], target).is_err());
}
