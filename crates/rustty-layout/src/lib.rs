//! Géométrie des onglets et des fenêtres : un arbre de divisions par onglet,
//! qui ne sait rien du rendu et produit des rectangles en pixels.

pub mod geometry;
mod node;
pub mod tab_layout;

pub use geometry::{Axis, Direction, Rect, SplitId, WindowId};
pub use node::{MAX_RATIO, MIN_RATIO};
pub use tab_layout::TabLayout;
