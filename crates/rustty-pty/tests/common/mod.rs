//! Harnais des tests pty : la sortie est pompée en continu dans un thread
//! (jamais de tube plein, jamais de lecture bloquante dans le test) et chaque
//! attente est bornée, pour qu'un blocage devienne un échec qui s'explique.

use std::io::{ErrorKind, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rustty_pty::{ExitStatus, Pty};

const POLL: Duration = Duration::from_millis(10);

pub struct Output {
    buf: Arc<Mutex<Vec<u8>>>,
    closed: Arc<AtomicBool>,
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
    Output { buf, closed }
}

impl Output {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.buf.lock().unwrap()).into_owned()
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Attend que la sortie contienne `needle` ; échoue en montrant ce qui a été lu.
    pub fn expect(&self, needle: &str, timeout: Duration) -> String {
        let start = Instant::now();
        loop {
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
