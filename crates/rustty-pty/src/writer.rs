//! Écriture vers le PTY depuis un thread dédié : l'interface envoie des
//! messages dans un canal et ne bloque jamais, même si le shell ne lit pas.

use std::io::Write;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;

#[derive(Clone, Debug)]
pub struct PtyWriter {
    tx: Sender<Vec<u8>>,
}

impl PtyWriter {
    /// Met `bytes` en file ; un thread écrivain disparu rend l'envoi silencieux.
    pub fn send(&self, bytes: impl Into<Vec<u8>>) {
        let _ = self.tx.send(bytes.into());
    }
}

/// Démarre le thread qui écrit chaque message dans `sink`, dans l'ordre.
pub fn spawn_writer(mut sink: Box<dyn Write + Send>) -> (PtyWriter, JoinHandle<()>) {
    let (tx, rx) = channel::<Vec<u8>>();
    let handle = std::thread::Builder::new()
        .name("rustty-pty-writer".into())
        .spawn(move || {
            for message in rx {
                if sink
                    .write_all(&message)
                    .and_then(|()| sink.flush())
                    .is_err()
                {
                    break;
                }
            }
        })
        .expect("création du thread écrivain");
    (PtyWriter { tx }, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn messages_are_written_in_order_then_the_thread_stops() {
        let sink = Sink::default();
        let (writer, handle) = spawn_writer(Box::new(sink.clone()));
        writer.send(b"ab".to_vec());
        writer.send("cd");
        let clone = writer.clone();
        clone.send(vec![b'e']);
        drop(writer);
        drop(clone);
        handle.join().unwrap();
        assert_eq!(sink.0.lock().unwrap().as_slice(), b"abcde");
    }

    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("fermé"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_write_error_stops_the_thread_and_later_sends_are_silent() {
        let (writer, handle) = spawn_writer(Box::new(Broken));
        writer.send("x");
        handle.join().unwrap();
        writer.send("y");
    }
}
