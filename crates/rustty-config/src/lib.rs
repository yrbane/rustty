//! Configuration de rustty : lecture TOML, valeurs par défaut complètes,
//! validation avec erreurs positionnées, table des raccourcis.

pub mod action;
pub mod color;

pub use color::{ColorParseError, Rgb};

pub mod config;
pub mod error;
pub mod example;
pub mod keymap;
pub mod keys;
pub mod sections;

pub use action::{Action, FocusDirection, ResizeDir, SplitAxis};
pub use config::{Config, MAX_SCROLLBACK_LINES};
pub use error::ConfigError;
pub use example::DEFAULT_TOML;
pub use keymap::KeyMap;
pub use keys::{Key, KeyCombo, KeyParseError, Mods, NamedKey};
pub use sections::{CloseButtonStyle, Colors, Font, TabBarPosition, Tabs, Window};
