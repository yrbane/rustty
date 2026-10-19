//! Un terminal vivant : le `Term` sous verrou, le PTY, le thread lecteur qui
//! l'alimente et réveille l'interface, le thread écrivain.

use std::path::{Path, PathBuf};
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

/// Ce qu'un nouveau terminal lance, et où.
#[derive(Clone, Copy)]
pub struct Launch<'a> {
    pub shell: &'a Shell,
    /// `None` : le dossier hérité du processus rustty.
    pub cwd: Option<&'a Path>,
}

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
        launch: Launch<'_>,
        cols: usize,
        rows: usize,
        pixels: (u32, u32),
        scrollback_lines: usize,
        waker: Waker,
    ) -> Result<Self, PtyError> {
        let size = PtySize::with_pixels(cols as u16, rows as u16, pixels.0 as u16, pixels.1 as u16);
        let mut pty = Pty::spawn(launch.shell, size, &default_env(), launch.cwd)?;
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

    /// Dossier courant du programme au premier plan, sinon du shell.
    pub fn cwd(&self) -> Option<PathBuf> {
        self.pty.cwd()
    }

    pub fn has_running_children(&self) -> bool {
        self.pty.has_running_children()
    }
}

#[cfg(test)]
#[path = "term_window_tests.rs"]
mod tests;
