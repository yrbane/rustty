//! Rendu GPU : transforme un `Snapshot` de `rustty-vt` et le chrome de
//! l'interface en passes wgpu. Testé hors écran, sans fenêtre.

pub mod color;

pub use color::{Palette, Rgba};
