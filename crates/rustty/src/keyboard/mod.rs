//! Clavier : résolution des raccourcis puis encodage pour le terminal.

pub mod combo;
pub mod encode;

pub use combo::{key_combo, mods_from_winit};
pub use encode::{encode_key, paste_bytes};
