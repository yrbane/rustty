//! Configuration de rustty : lecture TOML, valeurs par défaut complètes,
//! validation avec erreurs positionnées, table des raccourcis.

pub mod color;

pub use color::{ColorParseError, Rgb};
