//! Tests d'intégration : un vrai pseudo-terminal, un vrai shell du système.

use std::io::Read;
use std::time::{Duration, Instant};

use rustty_pty::{ExitStatus, Pty, PtySize, Shell, default_env};

/// Le shell de test et la commande qui affiche un argument puis sort avec un code.
fn echo_then_exit(text: &str, code: u32) -> Shell {
    if cfg!(windows) {
        Shell::new(
            "cmd.exe",
            vec!["/C".into(), format!("echo {text}& exit {code}")],
        )
    } else {
        Shell::new(
            "/bin/sh",
            vec!["-c".into(), format!("echo {text}; exit {code}")],
        )
    }
}

/// Lit jusqu'à voir `needle` ou jusqu'au délai ; rend tout ce qui a été lu.
fn read_until(reader: &mut dyn Read, needle: &str, timeout: Duration) -> String {
    let start = Instant::now();
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    while start.elapsed() < timeout {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                out.extend_from_slice(&buf[..n]);
                if String::from_utf8_lossy(&out).contains(needle) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn interactive_shell() -> Shell {
    if cfg!(windows) {
        Shell::new("cmd.exe", Vec::new())
    } else {
        Shell::new("/bin/sh", Vec::new())
    }
}

#[test]
fn echo_output_is_readable() {
    let mut pty = Pty::spawn(
        &echo_then_exit("bonjour", 0),
        PtySize::new(80, 24),
        &default_env(),
        None,
    )
    .unwrap();
    let mut reader = pty.reader().unwrap();
    let out = read_until(&mut *reader, "bonjour", Duration::from_secs(10));
    assert!(out.contains("bonjour"), "sortie lue : {out:?}");
    assert_eq!(pty.wait().unwrap(), ExitStatus::Exited(0));
}

#[test]
fn exit_code_is_reported() {
    let mut pty = Pty::spawn(
        &echo_then_exit("x", 3),
        PtySize::new(80, 24),
        &default_env(),
        None,
    )
    .unwrap();
    let mut reader = pty.reader().unwrap();
    let _ = read_until(&mut *reader, "x", Duration::from_secs(10));
    let status = pty.wait().unwrap();
    assert_eq!(status, ExitStatus::Exited(3));
    assert_eq!(
        pty.try_wait().unwrap(),
        Some(ExitStatus::Exited(3)),
        "try_wait après wait rend le même état"
    );
}

#[test]
fn unknown_program_is_an_error() {
    let shell = Shell::new("rustty-programme-qui-n-existe-pas", Vec::new());
    let result = Pty::spawn(&shell, PtySize::new(80, 24), &default_env(), None);
    match result {
        Err(e) => assert!(
            e.to_string().contains("rustty-programme-qui-n-existe-pas"),
            "{e}"
        ),
        Ok(mut pty) => {
            // Certains systèmes ne signalent l'échec qu'au premier wait : le
            // processus doit alors être terminé avec un code non nul.
            let status = pty.wait().unwrap();
            assert_ne!(
                status,
                ExitStatus::Exited(0),
                "un programme inexistant ne réussit pas"
            );
        }
    }
}

#[test]
fn written_input_is_echoed_back_by_an_interactive_shell() {
    let mut pty = Pty::spawn(
        &interactive_shell(),
        PtySize::new(80, 24),
        &default_env(),
        None,
    )
    .unwrap();
    let mut reader = pty.reader().unwrap();
    pty.write(b"echo marqueur-rustty\r\n").unwrap();
    let out = read_until(&mut *reader, "marqueur-rustty", Duration::from_secs(10));
    assert!(out.contains("marqueur-rustty"), "sortie lue : {out:?}");
    pty.write(b"exit\r\n").unwrap();
    let status = pty.wait().unwrap();
    assert_eq!(status, ExitStatus::Exited(0));
}

#[test]
fn resize_is_accepted_while_running() {
    let mut pty = Pty::spawn(
        &interactive_shell(),
        PtySize::new(80, 24),
        &default_env(),
        None,
    )
    .unwrap();
    pty.resize(PtySize::with_pixels(100, 40, 800, 600)).unwrap();
    pty.kill().unwrap();
    let _ = pty.wait();
}

#[test]
fn env_is_passed_to_the_child() {
    let shell = if cfg!(windows) {
        Shell::new("cmd.exe", vec!["/C".into(), "echo %RUSTTY_PROBE%".into()])
    } else {
        Shell::new("/bin/sh", vec!["-c".into(), "echo $RUSTTY_PROBE".into()])
    };
    let mut env = default_env();
    env.push(("RUSTTY_PROBE".into(), "valeur-sonde".into()));
    let mut pty = Pty::spawn(&shell, PtySize::new(80, 24), &env, None).unwrap();
    let mut reader = pty.reader().unwrap();
    let out = read_until(&mut *reader, "valeur-sonde", Duration::from_secs(10));
    assert!(out.contains("valeur-sonde"), "{out:?}");
    let _ = pty.wait();
}

#[test]
fn dropping_a_pty_kills_the_child() {
    let pty = Pty::spawn(
        &interactive_shell(),
        PtySize::new(80, 24),
        &default_env(),
        None,
    )
    .unwrap();
    let mut reader = pty.reader().unwrap();
    drop(pty);
    // Le lecteur cloné voit la fin du flux : Ok(0) ou une erreur, jamais un blocage infini.
    let _ = read_until(&mut *reader, "\u{0}jamais", Duration::from_secs(10));
}
