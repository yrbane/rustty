//! Rendu GPU : transforme un `Snapshot` de `rustty-vt` et le chrome de
//! l'interface en passes wgpu. Testé hors écran, sans fenêtre.

pub mod color;

pub use color::{Palette, Rgba};
pub mod font;

pub use font::{
    CellMetrics, FaceData, FontError, FontSet, GlyphBitmap, GlyphRef, Rasterizer, Variant,
};
pub mod builtin;

pub use builtin::{builtin_glyph, is_builtin};
