//! Tests d'intégration : un vrai pseudo-terminal, un vrai shell du système.
//! Toute lecture passe par le harnais `common` : sortie pompée en continu,
//! attentes bornées.

mod common;

use std::io::Write;
use std::time::Duration;

use common::{pump, wait_bounded};
use rustty_pty::{ExitStatus, Pty, PtySize, Shell, default_env};

/// Marge large : PowerShell et les runners virtualisés sont lents à démarrer.
const TIMEOUT: Duration = Duration::from_secs(30);

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

fn interactive_shell() -> Shell {
    if cfg!(windows) {
        Shell::new("cmd.exe", Vec::new())
    } else {
        Shell::new("/bin/sh", Vec::new())
    }
}

fn spawn(shell: &Shell) -> Pty {
    Pty::spawn(shell, PtySize::new(80, 24), &default_env(), None).unwrap()
}

#[test]
fn echo_output_is_readable() {
    let mut pty = spawn(&echo_then_exit("bonjour", 0));
    let out = pump(pty.reader().unwrap());
    out.expect(&mut pty, "bonjour", TIMEOUT);
    assert_eq!(wait_bounded(&mut pty, &out, TIMEOUT), ExitStatus::Exited(0));
}

#[test]
fn exit_code_is_reported() {
    let mut pty = spawn(&echo_then_exit("x", 3));
    let out = pump(pty.reader().unwrap());
    out.expect(&mut pty, "x", TIMEOUT);
    assert_eq!(wait_bounded(&mut pty, &out, TIMEOUT), ExitStatus::Exited(3));
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
            let out = pump(pty.reader().unwrap());
            let status = wait_bounded(&mut pty, &out, TIMEOUT);
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
    let mut pty = spawn(&interactive_shell());
    let out = pump(pty.reader().unwrap());
    pty.write(b"echo marqueur-rustty\r\n").unwrap();
    out.expect(&mut pty, "marqueur-rustty", TIMEOUT);
    pty.write(b"exit\r\n").unwrap();
    assert_eq!(wait_bounded(&mut pty, &out, TIMEOUT), ExitStatus::Exited(0));
}

#[test]
fn resize_is_accepted_while_running() {
    let mut pty = spawn(&interactive_shell());
    let out = pump(pty.reader().unwrap());
    pty.resize(PtySize::with_pixels(100, 40, 800, 600)).unwrap();
    pty.kill().unwrap();
    assert_ne!(
        wait_bounded(&mut pty, &out, TIMEOUT),
        ExitStatus::Exited(0),
        "un shell tué ne réussit pas"
    );
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
    let out = pump(pty.reader().unwrap());
    out.expect(&mut pty, "valeur-sonde", TIMEOUT);
    wait_bounded(&mut pty, &out, TIMEOUT);
}

#[test]
fn dropping_a_pty_kills_the_child() {
    let pty = spawn(&interactive_shell());
    let out = pump(pty.reader().unwrap());
    drop(pty);
    // Le lecteur cloné voit la fin du flux (Unix : esclave fermé ; Windows :
    // pseudo-console fermée), jamais un blocage infini.
    assert!(
        out.wait_closed(TIMEOUT),
        "le flux reste ouvert après la libération du Pty ; sortie : {:?}",
        out.text()
    );
}

#[test]
fn large_output_is_delivered_in_order() {
    // 2 000 lignes numérotées : bien plus qu'un bloc de lecture.
    let shell = if cfg!(windows) {
        Shell::new(
            "cmd.exe",
            vec![
                "/C".into(),
                "for /L %i in (1,1,2000) do @echo ligne-%i".into(),
            ],
        )
    } else {
        Shell::new(
            "/bin/sh",
            vec![
                "-c".into(),
                "i=1; while [ $i -le 2000 ]; do echo ligne-$i; i=$((i+1)); done".into(),
            ],
        )
    };
    let mut pty = Pty::spawn(&shell, PtySize::new(200, 50), &default_env(), None).unwrap();
    let out = pump(pty.reader().unwrap());
    wait_bounded(&mut pty, &out, TIMEOUT);
    let text = out.expect(&mut pty, "ligne-2000", TIMEOUT);
    // ConPTY entoure le texte de séquences de contrôle (jusque sur la même
    // ligne) : on ne retient que les numéros, dans l'ordre d'apparition.
    let numbers: Vec<u32> = text
        .split("ligne-")
        .skip(1)
        .filter_map(|rest| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect();
    assert_eq!(
        numbers.len(),
        2000,
        "les 2000 lignes sont arrivées, sans doublon ni perte : {numbers:?}"
    );
    assert!(
        numbers.iter().copied().eq(1..=2000),
        "ordre des lignes : {numbers:?}"
    );
}

#[cfg(unix)]
#[test]
fn killed_shell_reports_signaled() {
    let shell = Shell {
        program: "sh".into(),
        args: vec!["-c".into(), "kill -9 $$".into()],
    };
    let mut pty = spawn(&shell);
    let out = pump(pty.reader().unwrap());
    assert_eq!(wait_bounded(&mut pty, &out, TIMEOUT), ExitStatus::Signaled);
}

#[test]
fn writer_can_be_taken_once() {
    let mut pty = spawn(&interactive_shell());
    let out = pump(pty.reader().unwrap());
    let mut taken = pty.take_writer().expect("premier appel");
    assert!(pty.take_writer().is_none(), "second appel");
    assert!(
        pty.write(b"x").is_err(),
        "l'écrivain n'est plus dans le Pty"
    );
    taken.write_all(b"echo via-writer\r\n").unwrap();
    taken.flush().unwrap();
    // Les réponses du « terminal » (DSR de ConPTY) passent par l'écrivain cédé.
    out.expect_with(
        &mut |bytes| {
            taken.write_all(bytes).unwrap();
            taken.flush().unwrap();
        },
        "via-writer",
        TIMEOUT,
    );
    pty.kill().unwrap();
    wait_bounded(&mut pty, &out, TIMEOUT);
}

#[cfg(unix)]
#[test]
fn running_children_are_detected_from_the_foreground_process_group() {
    let mut pty = spawn(&interactive_shell());
    let out = pump(pty.reader().unwrap());
    out.expect(&mut pty, "$", TIMEOUT);
    assert!(
        !pty.has_running_children(),
        "un shell au repos n'a pas d'enfant au premier plan"
    );
    pty.write(b"sleep 3\r\n").unwrap();
    let start = std::time::Instant::now();
    while !pty.has_running_children() {
        assert!(
            start.elapsed() < TIMEOUT,
            "sleep n'est jamais passé au premier plan ; sortie : {:?}",
            out.text()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    pty.kill().unwrap();
    wait_bounded(&mut pty, &out, TIMEOUT);
}
