//! Un terminal vivant : le `Term` sous verrou, le PTY, le thread lecteur qui
//! l'alimente et réveille l'interface, le thread écrivain.

use std::sync::Arc;

use parking_lot::Mutex;
use rustty_pty::{
    ExitStatus, Pty, PtyError, PtyEvent, PtySize, PtyWriter, Shell, default_env, spawn_reader_with,
    spawn_writer,
};
use rustty_vt::{Modes, Snapshot, Term, TermEvent};

use crate::events::{UserEvent, Waker};
use crate::model::ScrollRequest;
use crate::tab::TermId;

pub struct TermWindow {
    pub term: Arc<Mutex<Term>>,
    pub writer: PtyWriter,
    pub cols: usize,
    pub rows: usize,
    pub exit: Option<ExitStatus>,
    /// Dernière taille de cellule transmise au `Term`, en pixels.
    cell: (u32, u32),
    pty: Pty,
}

impl TermWindow {
    pub fn spawn(
        id: TermId,
        shell: &Shell,
        cols: usize,
        rows: usize,
        pixels: (u32, u32),
        scrollback_lines: usize,
        waker: Waker,
    ) -> Result<Self, PtyError> {
        let size = PtySize::with_pixels(cols as u16, rows as u16, pixels.0 as u16, pixels.1 as u16);
        let mut pty = Pty::spawn(shell, size, &default_env(), None)?;
        let reader = pty.reader()?;
        let sink = pty
            .take_writer()
            .ok_or_else(|| PtyError::Open("écrivain indisponible".into()))?;
        let (writer, _writer_thread) = spawn_writer(sink);
        let term = Arc::new(Mutex::new(Term::new(cols, rows, scrollback_lines)));
        let (term_for_reader, writer_for_reader) = (Arc::clone(&term), writer.clone());
        let _reader_thread = spawn_reader_with(reader, move |event| match event {
            PtyEvent::Data(bytes) => {
                let (responses, events) = {
                    let mut t = term_for_reader.lock();
                    t.input(&bytes);
                    (t.drain_responses(), t.drain_events())
                };
                if !responses.is_empty() {
                    writer_for_reader.send(responses);
                }
                for e in events {
                    waker.wake(match e {
                        TermEvent::Title(title) => UserEvent::TermTitle(id, title),
                        TermEvent::Bell => UserEvent::TermBell(id),
                        TermEvent::ModeChanged(_) => UserEvent::TermModes(id),
                        TermEvent::SetClipboard(text) => UserEvent::SetClipboard(text),
                    });
                }
                waker.wake(UserEvent::TermUpdated(id));
            }
            PtyEvent::Failed(message) => waker.wake(UserEvent::TermFailed(id, message)),
            PtyEvent::Eof => waker.wake(UserEvent::PtyEof(id)),
        });
        Ok(Self {
            term,
            writer,
            cols,
            rows,
            exit: None,
            cell: (0, 0),
            pty,
        })
    }

    pub fn snapshot(&self) -> Snapshot {
        self.term.lock().snapshot()
    }

    pub fn modes(&self) -> Modes {
        *self.term.lock().modes()
    }

    pub fn write(&self, bytes: Vec<u8>) {
        self.writer.send(bytes);
    }

    /// Redimensionne la grille et transmet la taille d'une cellule au `Term`
    /// (elle sert à compter les cellules couvertes par une image). Vrai si la
    /// grille a changé ; un changement de pixels seul ne touche pas le PTY.
    pub fn resize(
        &mut self,
        cols: usize,
        rows: usize,
        pixels: (u32, u32),
        cell: (u32, u32),
    ) -> bool {
        if cell != self.cell {
            self.cell = cell;
            self.term.lock().set_cell_pixels(cell.0, cell.1);
        }
        if (cols, rows) == (self.cols, self.rows) {
            return false;
        }
        self.cols = cols;
        self.rows = rows;
        self.term.lock().resize(cols, rows);
        let size = PtySize::with_pixels(cols as u16, rows as u16, pixels.0 as u16, pixels.1 as u16);
        if let Err(e) = self.pty.resize(size) {
            tracing::warn!("resize du pty : {e}");
        }
        true
    }

    pub fn scroll(&self, request: ScrollRequest) {
        let mut t = self.term.lock();
        match request {
            ScrollRequest::Lines(n) => t.scroll_display(n as isize),
            ScrollRequest::Pages(n) => t.scroll_display(n as isize * self.rows as isize),
            ScrollRequest::ToBottom => t.scroll_display_to_bottom(),
        }
    }

    /// Le statut de sortie du shell, mémorisé dès qu'il est connu.
    pub fn poll_exit(&mut self) -> Option<ExitStatus> {
        if self.exit.is_none() {
            self.exit = self.pty.try_wait().ok().flatten();
        }
        self.exit
    }

    pub fn has_running_children(&self) -> bool {
        self.pty.has_running_children()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::sync::mpsc::{Receiver, channel};
    use std::time::{Duration, Instant};

    const TIMEOUT: Duration = Duration::from_secs(30);

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
            &echo_shell("bonjour-term"),
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
            &echo_shell("fin"),
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
        let mut tw =
            TermWindow::spawn(TermId(3), &shell, 40, 5, (400, 100), 100, Arc::new(tx)).unwrap();
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
        let mut tw =
            TermWindow::spawn(TermId(4), &shell, 40, 5, (400, 100), 100, Arc::new(tx)).unwrap();
        assert!(!tw.resize(40, 5, (480, 120), (12, 24)), "grille inchangée");
        // 24×48 pixels : 2 colonnes à 12 px par cellule (3 avec les 10 px par défaut).
        let data = base64::engine::general_purpose::STANDARD.encode(vec![0u8; 24 * 48 * 3]);
        let apc = format!("\x1b_Ga=T,f=24,s=24,v=48;{data}\x1b\\");
        tw.term.lock().input(apc.as_bytes());
        assert_eq!(tw.term.lock().cursor().col, 2);
    }
}
