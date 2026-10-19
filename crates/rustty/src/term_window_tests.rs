//! Tests de `TermWindow` : un vrai shell dans un vrai pseudo-terminal.

use super::*;
use base64::Engine;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(30);

fn here(shell: &Shell) -> Launch<'_> {
    Launch { shell, cwd: None }
}

fn echo_shell(text: &str) -> Shell {
    if cfg!(windows) {
        Shell::new("cmd.exe", vec!["/C".into(), format!("echo {text}& exit 7")])
    } else {
        Shell::new("/bin/sh", vec!["-c".into(), format!("echo {text}; exit 7")])
    }
}

fn wait_for_text(tw: &TermWindow, rx: &Receiver<UserEvent>, needle: &str) {
    let start = Instant::now();
    loop {
        if tw
            .snapshot()
            .lines
            .iter()
            .any(|l| l.text().contains(needle))
        {
            return;
        }
        assert!(
            start.elapsed() < TIMEOUT,
            "« {needle} » absent ; écran : {:?}",
            tw.term.lock().text()
        );
        let _ = rx.recv_timeout(Duration::from_millis(200));
    }
}

#[test]
fn shell_output_reaches_the_term_and_wakes_the_ui() {
    let (tx, rx) = channel();
    let tw = TermWindow::spawn(
        TermId(1),
        here(&echo_shell("bonjour-term")),
        40,
        5,
        (400, 100),
        100,
        Arc::new(tx),
    )
    .unwrap();
    wait_for_text(&tw, &rx, "bonjour-term");
    assert_eq!((tw.cols, tw.rows), (40, 5));
}

#[test]
fn exited_shell_is_reported_by_poll() {
    let (tx, rx) = channel();
    let mut tw = TermWindow::spawn(
        TermId(2),
        here(&echo_shell("fin")),
        40,
        5,
        (400, 100),
        100,
        Arc::new(tx),
    )
    .unwrap();
    wait_for_text(&tw, &rx, "fin");
    let start = Instant::now();
    loop {
        if let Some(status) = tw.poll_exit() {
            assert_eq!(status, ExitStatus::Exited(7));
            break;
        }
        assert!(start.elapsed() < TIMEOUT, "le shell ne se termine pas");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(tw.poll_exit(), Some(ExitStatus::Exited(7)), "mémorisé");
    assert_eq!(tw.exit, Some(ExitStatus::Exited(7)));
}

#[test]
fn written_bytes_and_resize_reach_the_shell() {
    let (tx, rx) = channel();
    let shell = if cfg!(windows) {
        Shell::new("cmd.exe", Vec::new())
    } else {
        Shell::new("/bin/sh", Vec::new())
    };
    let mut tw = TermWindow::spawn(
        TermId(3),
        here(&shell),
        40,
        5,
        (400, 100),
        100,
        Arc::new(tx),
    )
    .unwrap();
    assert!(tw.resize(60, 10, (600, 200), (10, 20)));
    assert_eq!((tw.cols, tw.rows), (60, 10));
    assert_eq!(tw.snapshot().cols, 60);
    tw.write(b"echo marqueur-tw\r\n".to_vec());
    wait_for_text(&tw, &rx, "marqueur-tw");
    tw.scroll(ScrollRequest::Lines(-1));
    tw.scroll(ScrollRequest::ToBottom);
    assert_eq!(tw.snapshot().display_offset, 0);
    drop(tw);
}

#[test]
fn pixel_only_change_reaches_the_term() {
    let (tx, _rx) = channel();
    let shell = if cfg!(windows) {
        Shell::new("cmd.exe", Vec::new())
    } else {
        Shell::new("/bin/sh", Vec::new())
    };
    let mut tw = TermWindow::spawn(
        TermId(4),
        here(&shell),
        40,
        5,
        (400, 100),
        100,
        Arc::new(tx),
    )
    .unwrap();
    assert!(!tw.resize(40, 5, (480, 120), (12, 24)), "grille inchangée");
    // 24×48 pixels : 2 colonnes à 12 px par cellule (3 avec les 10 px par défaut).
    let data = base64::engine::general_purpose::STANDARD.encode(vec![0u8; 24 * 48 * 3]);
    let apc = format!("\x1b_Ga=T,f=24,s=24,v=48;{data}\x1b\\");
    tw.term.lock().input(apc.as_bytes());
    assert_eq!(tw.term.lock().cursor().col, 2);
}

#[cfg(unix)]
#[test]
fn shell_starts_in_the_given_directory() {
    let (tx, _rx) = channel();
    let dir = std::env::temp_dir().canonicalize().unwrap();
    let shell = Shell::new("/bin/sh", Vec::new());
    let launch = Launch {
        shell: &shell,
        cwd: Some(&dir),
    };
    let tw = TermWindow::spawn(TermId(5), launch, 40, 5, (400, 100), 100, Arc::new(tx)).unwrap();
    let start = Instant::now();
    while tw.cwd().as_deref() != Some(dir.as_path()) {
        assert!(start.elapsed() < TIMEOUT, "dossier : {:?}", tw.cwd());
        std::thread::sleep(Duration::from_millis(20));
    }
}
