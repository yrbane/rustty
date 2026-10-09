//! Clavier : résolution des raccourcis puis encodage pour le terminal.

pub mod combo;

pub use combo::{key_combo, mods_from_winit};
