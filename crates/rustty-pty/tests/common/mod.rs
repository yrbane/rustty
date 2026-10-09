//! Harnais des tests pty : la sortie est pompée en continu dans un thread
//! (jamais de tube plein, jamais de lecture bloquante dans le test) et chaque
//! attente est bornée, pour qu'un blocage devienne un échec qui s'explique.

use std::cell::Cell;
use std::io::{ErrorKind, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rustty_pty::{ExitStatus, Pty};

const POLL: Duration = Duration::from_millis(10);

/// Demande de position du curseur (DSR 6) : ConPTY l'envoie au démarrage et
/// n'émet rien tant qu'un terminal n'y a pas répondu.
const CURSOR_POSITION_REQUEST: &str = "\x1b[6n";
const CURSOR_POSITION_REPLY: &[u8] = b"\x1b[1;1R";

pub struct Output {
    buf: Arc<Mutex<Vec<u8>>>,
    closed: Arc<AtomicBool>,
    requests_answered: Cell<usize>,
}

/// Lit `reader` jusqu'à la fin du flux dans un thread dédié.
pub fn pump(mut reader: Box<dyn Read + Send>) -> Output {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let closed = Arc::new(AtomicBool::new(false));
    let (b, c) = (Arc::clone(&buf), Arc::clone(&closed));
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => b.lock().unwrap().extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        c.store(true, Ordering::SeqCst);
    });
    Output {
        buf,
        closed,
        requests_answered: Cell::new(0),
    }
}

impl Output {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.buf.lock().unwrap()).into_owned()
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Joue le rôle du terminal : répond à chaque demande de position du
    /// curseur apparue dans la sortie, comme le fera `rustty-vt`. `reply`
    /// écrit vers le PTY par le canal que le test a sous la main.
    pub fn answer_requests_with(&self, reply: &mut dyn FnMut(&[u8])) {
        let seen = self.text().matches(CURSOR_POSITION_REQUEST).count();
        while self.requests_answered.get() < seen {
            reply(CURSOR_POSITION_REPLY);
            self.requests_answered.set(self.requests_answered.get() + 1);
        }
    }

    /// Variante courante : répond par `Pty::write`. Un écrivain déjà cédé par
    /// `take_writer` rend l'écriture impossible, ce qui n'est pas une erreur
    /// du test (voir `writer_can_be_taken_once`).
    pub fn answer_requests(&self, pty: &mut Pty) {
        self.answer_requests_with(&mut |bytes| {
            let _ = pty.write(bytes);
        });
    }

    /// Attend que la sortie contienne `needle` en répondant aux requêtes du
    /// programme ; échoue en montrant ce qui a été lu.
    pub fn expect(&self, pty: &mut Pty, needle: &str, timeout: Duration) -> String {
        self.expect_with(
            &mut |bytes| {
                let _ = pty.write(bytes);
            },
            needle,
            timeout,
        )
    }

    /// Même attente, les réponses passant par `reply`.
    pub fn expect_with(
        &self,
        reply: &mut dyn FnMut(&[u8]),
        needle: &str,
        timeout: Duration,
    ) -> String {
        let start = Instant::now();
        loop {
            self.answer_requests_with(reply);
            let text = self.text();
            if text.contains(needle) {
                return text;
            }
            assert!(
                !self.is_closed(),
                "flux fermé sans « {needle} » ; sortie lue : {text:?}"
            );
            assert!(
                start.elapsed() < timeout,
                "« {needle} » absent après {timeout:?} ; sortie lue : {text:?}"
            );
            std::thread::sleep(POLL);
        }
    }

    /// Attend la fin du flux ; `true` si elle arrive dans le délai.
    pub fn wait_closed(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        while !self.is_closed() {
            if start.elapsed() >= timeout {
                return false;
            }
            std::thread::sleep(POLL);
        }
        true
    }
}

/// `wait` borné par sondage de `try_wait` : au-delà du délai, l'enfant est
/// tué et le test échoue en montrant la sortie.
pub fn wait_bounded(pty: &mut Pty, out: &Output, timeout: Duration) -> ExitStatus {
    let start = Instant::now();
    loop {
        out.answer_requests(pty);
        if let Some(status) = pty.try_wait().unwrap() {
            return status;
        }
        if start.elapsed() >= timeout {
            let _ = pty.kill();
            panic!(
                "le processus ne s'est pas terminé en {timeout:?} ; sortie lue : {:?}",
                out.text()
            );
        }
        std::thread::sleep(POLL);
    }
}
