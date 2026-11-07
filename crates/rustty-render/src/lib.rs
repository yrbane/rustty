//! Rendu GPU : transforme un `Snapshot` de `rustty-vt` et le chrome de
//! l'interface en passes wgpu. Testé hors écran, sans fenêtre.

pub mod color;

pub use color::{Palette, Rgba, readable_on};
pub mod font;

pub use font::{
    CellMetrics, FaceData, FontError, FontSet, GlyphBitmap, GlyphRef, IconFace, Rasterizer,
    Variant, glyphs as icons,
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
pub use pipeline::quad::{QuadBatch, QuadInstance, QuadPipeline, QuadStyle};
pub mod frame;
pub mod grid;
pub mod images;

pub use frame::{
    CellRange, Chrome, ChromeIcon, ChromeQuad, ChromeShape, ChromeText, Frame, PaneFrame, PixelRect,
};
pub use grid::{
    CURSOR_BAR_WIDTH, GlyphRequest, GridGeometry, PaneInstances, grid_geometry, pane_instances,
};
pub use images::{ImageDraw, pane_images};
pub mod renderer;

pub use renderer::Renderer;
pub mod chrome;

pub use chrome::{
    ButtonLook, HoverTarget, INACTIVE_ACCENT_DIM, PaneHeaderSpec, TAB_BAR_PADDING, TabBarLayout,
    TabBarStyle, TabRect, TabSpec, TitleBarLayout, TitleBarState, TitleBarStyle, TitleTarget,
    layout_tab_bar, layout_title_bar, pane_header_chrome, pane_header_close_rect, tab_bar_chrome,
    tab_bar_height, title_bar_chrome, title_bar_height,
};
