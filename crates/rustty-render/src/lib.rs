//! Rendu GPU : transforme un `Snapshot` de `rustty-vt` et le chrome de
//! l'interface en passes wgpu. Testé hors écran, sans fenêtre.

pub mod color;

pub use color::{Palette, Rgba, readable_on};
pub mod font;

pub use font::{
    CellMetrics, FaceData, FontError, FontSet, GlyphBitmap, GlyphRef, Rasterizer, Variant,
};
pub mod builtin;

pub use builtin::{builtin_glyph, is_builtin};
pub mod atlas;

pub use atlas::{AtlasPacker, AtlasRegion, DEFAULT_ATLAS_SIZE, atlas_size_for};
pub mod gpu;

pub use gpu::{GpuContext, GpuError, OFFSCREEN_FORMAT, Offscreen, clear};
pub mod pipeline;

pub use pipeline::glyph::{AtlasTexture, GlyphBatch, GlyphInstance, GlyphPipeline};
pub use pipeline::image::{ImageBatch, ImageInstance, ImagePipeline, ImageTextures};
pub use pipeline::quad::{QuadBatch, QuadInstance, QuadPipeline};
pub mod frame;
pub mod grid;
pub mod images;

pub use frame::{Chrome, ChromeQuad, ChromeText, Frame, PaneFrame, PixelRect};
pub use grid::{
    CURSOR_BAR_WIDTH, GlyphRequest, GridGeometry, PaneInstances, grid_geometry, pane_instances,
};
pub use images::{ImageDraw, pane_images};
pub mod renderer;

pub use renderer::Renderer;
pub mod chrome;

pub use chrome::{
    HoverTarget, INACTIVE_ACCENT_DIM, TAB_BAR_PADDING, TabBarLayout, TabBarStyle, TabRect, TabSpec,
    layout_tab_bar, tab_bar_chrome, tab_bar_height,
};
