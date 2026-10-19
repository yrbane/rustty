//! Dossier courant d'un pseudo-terminal : celui du programme au premier plan,
//! à défaut celui du shell, `None` s'il n'existe plus. Windows ne l'expose
//! pas : voir le test unitaire de `cwd.rs`.

#![cfg(unix)]

// Seul `pump` sert ici ; le reste du harnais sert à `spawn.rs`.
#[allow(dead_code)]
mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rustty_pty::{Pty, PtySize, Shell, default_env};

const TIMEOUT: Duration = Duration::from_secs(30);

/// Un dossier temporaire propre au test, au chemin canonique (macOS place
/// `/tmp` derrière `/private`).
fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustty-cwd-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

fn spawn_in(shell: &Shell, cwd: Option<&Path>) -> Pty {
    Pty::spawn(shell, PtySize::new(80, 24), &default_env(), cwd).unwrap()
}

/// Sonde `pty.cwd()` jusqu'à obtenir `expected`, ou jusqu'au délai.
fn wait_for_cwd(pty: &Pty, expected: Option<&Path>) -> Option<PathBuf> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let cwd = pty.cwd();
        if cwd.as_deref() == expected || Instant::now() > deadline {
            return cwd;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn cwd_is_the_spawn_directory() {
    let dir = scratch_dir("spawn");
    let pty = spawn_in(&Shell::new("/bin/sh", Vec::new()), Some(&dir));
    let _out = common::pump(pty.reader().unwrap());
    assert_eq!(wait_for_cwd(&pty, Some(&dir)), Some(dir.clone()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cwd_follows_cd_in_the_shell() {
    let dir = scratch_dir("cd");
    let mut pty = spawn_in(&Shell::new("/bin/sh", Vec::new()), None);
    let _out = common::pump(pty.reader().unwrap());
    pty.write(format!("cd '{}'\n", dir.display()).as_bytes())
        .unwrap();
    assert_eq!(wait_for_cwd(&pty, Some(&dir)), Some(dir.clone()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cwd_is_the_foreground_program_one() {
    let dir = scratch_dir("foreground");
    let mut pty = spawn_in(&Shell::new("/bin/sh", Vec::new()), None);
    let _out = common::pump(pty.reader().unwrap());
    // Le sous-shell change de dossier puis devient `sleep` ; le shell, lui, reste.
    pty.write(format!("(cd '{}' && exec sleep 30)\n", dir.display()).as_bytes())
        .unwrap();
    assert_eq!(wait_for_cwd(&pty, Some(&dir)), Some(dir.clone()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deleted_directory_gives_none() {
    let dir = scratch_dir("deleted");
    let pty = spawn_in(&Shell::new("/bin/sh", Vec::new()), Some(&dir));
    let _out = common::pump(pty.reader().unwrap());
    assert_eq!(wait_for_cwd(&pty, Some(&dir)), Some(dir.clone()));
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(wait_for_cwd(&pty, None), None);
}
