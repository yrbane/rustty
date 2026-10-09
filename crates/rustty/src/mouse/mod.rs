//! Souris : rapports pour les applications, sélection pour l'utilisateur.

pub mod encode;
pub mod selection;

pub use encode::{MouseButton, MouseKind, encode_mouse};
pub use selection::{CellPos, Selection};
