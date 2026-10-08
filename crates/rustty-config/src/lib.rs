//! Configuration de rustty : lecture TOML, valeurs par défaut complètes,
//! validation avec erreurs positionnées, table des raccourcis.

pub mod color;

pub use color::{ColorParseError, Rgb};

pub mod config;
pub mod error;
pub mod sections;

pub use config::{Config, MAX_SCROLLBACK_LINES};
pub use error::ConfigError;
pub use sections::{CloseButtonStyle, Colors, Font, TabBarPosition, Tabs, Window};
