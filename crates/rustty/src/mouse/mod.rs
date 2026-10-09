//! Souris : rapports pour les applications, sélection pour l'utilisateur.

pub mod click;
pub mod encode;
pub mod motion;
pub mod selection;
pub mod wheel;

pub use click::DoubleClick;
pub use encode::{MouseButton, MouseKind, encode_mouse};
pub use motion::MotionFilter;
pub use selection::{CellPos, Selection};
pub use wheel::WheelAccumulator;
