//! Façade sur `portable-pty` : un processus dans un pseudo-terminal, son
//! flux de lecture clonable, son flux d'écriture, sa taille et son état.

use std::io::{Read, Write};
use std::path::Path;

use portable_pty::{Child, CommandBuilder, MasterPty, native_pty_system};

use crate::error::PtyError;
use crate::shell::Shell;
use crate::size::PtySize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitStatus {
    Exited(u32),
    /// Terminé par un signal (Unix) ou tué.
    Signaled,
}

pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    writer: Option<Box<dyn Write + Send>>,
    child: Box<dyn Child + Send + Sync>,
    program: String,
}

fn to_native(size: PtySize) -> portable_pty::PtySize {
    portable_pty::PtySize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: size.pixel_width,
        pixel_height: size.pixel_height,
    }
}

impl Pty {
    /// Ouvre un pseudo-terminal de `size` et y lance `shell` avec `env` en plus
    /// de l'environnement hérité, dans `cwd` s'il est donné.
    pub fn spawn(
        shell: &Shell,
        size: PtySize,
        env: &[(String, String)],
        cwd: Option<&Path>,
    ) -> Result<Self, PtyError> {
        let system = native_pty_system();
        let pair = system
            .openpty(to_native(size))
            .map_err(|e| PtyError::Open(e.to_string()))?;
        let mut cmd = CommandBuilder::new(&shell.program);
        cmd.args(&shell.args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }
        let child = pair.slave.spawn_command(cmd).map_err(|e| PtyError::Spawn {
            program: shell.program.clone(),
            reason: e.to_string(),
        })?;
        // Le côté esclave appartient à l'enfant : on le ferme ici pour que la
        // fin du processus se traduise par une fin de flux côté lecteur.
        drop(pair.slave);
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Open(e.to_string()))?;
        Ok(Self {
            master: pair.master,
            writer: Some(writer),
            child,
            program: shell.program.clone(),
        })
    }

    /// Un lecteur bloquant indépendant ; `read` rend `Ok(0)` ou une erreur
    /// quand le terminal est fermé.
    ///
    /// Sur Unix, cela arrive dès que le dernier processus tenant l'esclave se
    /// termine. Sur Windows (ConPTY), le tube n'est fermé qu'à la libération du
    /// `Pty` : la fin du shell se détecte donc par `try_wait`, pas par la fin
    /// du flux, et l'appelant doit sonder `try_wait` (ou réagir à `wait`) au
    /// lieu d'attendre `Ok(0)`.
    pub fn reader(&self) -> Result<Box<dyn Read + Send>, PtyError> {
        self.master
            .try_clone_reader()
            .map_err(|e| PtyError::Open(e.to_string()))
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            PtyError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "écrivain déjà pris par take_writer",
            ))
        })?;
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }

    /// Cède l'écrivain (une seule fois) à un thread dédié ; `write` échoue ensuite.
    pub fn take_writer(&mut self) -> Option<Box<dyn Write + Send>> {
        self.writer.take()
    }

    /// Vrai si un programme autre que le shell est au premier plan du terminal.
    #[cfg(unix)]
    pub fn has_running_children(&self) -> bool {
        match (self.master.process_group_leader(), self.child.process_id()) {
            (Some(leader), Some(pid)) => leader != pid as libc::pid_t,
            _ => false,
        }
    }

    /// Windows n'expose pas le groupe de premier plan de ConPTY.
    #[cfg(windows)]
    pub fn has_running_children(&self) -> bool {
        false
    }

    pub fn resize(&self, size: PtySize) -> Result<(), PtyError> {
        self.master
            .resize(to_native(size))
            .map_err(|e| PtyError::Open(e.to_string()))
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// `None` tant que le processus tourne.
    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, PtyError> {
        Ok(self.child.try_wait()?.map(convert_status))
    }

    pub fn wait(&mut self) -> Result<ExitStatus, PtyError> {
        Ok(convert_status(self.child.wait()?))
    }

    pub fn kill(&mut self) -> Result<(), PtyError> {
        self.child.kill()?;
        Ok(())
    }

    pub fn program(&self) -> &str {
        &self.program
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn convert_status(status: portable_pty::ExitStatus) -> ExitStatus {
    // portable-pty rapporte une mort par signal avec `signal()` renseigné et un
    // code de sortie conventionnel (1) : seul `signal()` fait foi.
    if status.signal().is_some() {
        ExitStatus::Signaled
    } else {
        ExitStatus::Exited(status.exit_code())
    }
}
