//! Pseudo-terminal : lance le shell, lit ce qu'il écrit, lui transmet les
//! frappes, le redimensionne. Le thread lecteur est fourni ; le `Term` qui
//! interprète les octets vit ailleurs (`rustty-vt`).

pub mod error;
pub mod shell;
pub mod size;

pub use error::PtyError;
pub use shell::{Shell, default_env};
pub use size::PtySize;
pub mod pty;

pub use pty::{ExitStatus, Pty};
pub mod reader;

pub use reader::{PtyEvent, READ_CHUNK, spawn_reader, spawn_reader_with};
pub mod writer;

pub use writer::{PtyWriter, spawn_writer};
