//! Thread lecteur : vide le pseudo-terminal par blocs et pousse des
//! événements vers l'interface, qui les donne au `Term`.

use std::io::Read;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

/// Taille d'un bloc de lecture. Assez grand pour absorber une sortie
/// massive, assez petit pour rester réactif.
pub const READ_CHUNK: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PtyEvent {
    Data(Vec<u8>),
    /// Fin du flux : le processus a fermé le terminal (ou lecture impossible).
    Eof,
}

/// Lit `reader` jusqu'à la fin et envoie chaque bloc dans `tx`, puis `Eof`.
pub fn spawn_reader(reader: Box<dyn Read + Send>, tx: Sender<PtyEvent>) -> JoinHandle<()> {
    spawn_reader_with(reader, move |ev| {
        // Un récepteur disparu est la seule raison d'échec : on l'ignore, la
        // lecture continue jusqu'à la fin du flux.
        let _ = tx.send(ev);
    })
}

/// Même boucle, avec un rappel : le binaire y branche son réveil de boucle
/// d'événements. Le rappel reçoit `Eof` en dernier.
pub fn spawn_reader_with<F>(mut reader: Box<dyn Read + Send>, mut sink: F) -> JoinHandle<()>
where
    F: FnMut(PtyEvent) + Send + 'static,
{
    std::thread::Builder::new()
        .name("rustty-pty-reader".into())
        .spawn(move || {
            let mut buf = vec![0u8; READ_CHUNK];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => sink(PtyEvent::Data(buf[..n].to_vec())),
                }
            }
            sink(PtyEvent::Eof);
        })
        .expect("création du thread lecteur")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::mpsc;

    #[test]
    fn delivers_data_then_eof_in_order() {
        let data: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let (tx, rx) = mpsc::channel();
        let handle = spawn_reader(Box::new(Cursor::new(data.clone())), tx);
        handle.join().unwrap();
        let mut received = Vec::new();
        let mut eof_seen = false;
        for ev in rx.iter() {
            match ev {
                PtyEvent::Data(chunk) => {
                    assert!(!eof_seen, "aucune donnée après Eof");
                    assert!(!chunk.is_empty() && chunk.len() <= READ_CHUNK);
                    received.extend_from_slice(&chunk);
                }
                PtyEvent::Eof => eof_seen = true,
            }
        }
        assert!(eof_seen);
        assert_eq!(received, data, "ordre et intégrité préservés");
    }

    #[test]
    fn empty_stream_yields_only_eof() {
        let (tx, rx) = mpsc::channel();
        spawn_reader(Box::new(Cursor::new(Vec::new())), tx)
            .join()
            .unwrap();
        assert_eq!(rx.iter().count(), 1);
    }

    #[test]
    fn stops_quietly_when_the_receiver_is_gone() {
        let data = vec![1u8; 300_000];
        let (tx, rx) = mpsc::channel();
        drop(rx);
        spawn_reader(Box::new(Cursor::new(data)), tx)
            .join()
            .unwrap();
    }

    #[test]
    fn callback_variant_sees_the_same_events() {
        let data = b"abc".to_vec();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink_seen = seen.clone();
        spawn_reader_with(Box::new(Cursor::new(data)), move |ev| {
            sink_seen.lock().unwrap().push(ev)
        })
        .join()
        .unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert!(matches!(&seen[0], PtyEvent::Data(d) if d == b"abc"));
        assert!(matches!(seen[1], PtyEvent::Eof));
    }
}
