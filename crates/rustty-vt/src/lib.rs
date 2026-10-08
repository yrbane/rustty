//! Émulation de terminal pure, sans I/O : transforme un flux d'octets en état
//! de grille. Consommée par le renderer et le binaire `rustty`.

/// Version de la crate, héritée du workspace. Affichée dans l'interface.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod cell;
pub mod color;

pub use cell::{Attrs, Cell, Style};
pub use color::Color;
