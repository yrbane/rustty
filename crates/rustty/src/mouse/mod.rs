//! Souris : rapports pour les applications, sélection pour l'utilisateur.

pub mod encode;
pub mod selection;
pub mod wheel;

pub use encode::{MouseButton, MouseKind, encode_mouse};
pub use selection::{CellPos, Selection};
pub use wheel::WheelAccumulator;
