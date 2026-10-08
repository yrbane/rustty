//! Polices : découverte (`fontdb`), variantes gras/italique, repli, métriques
//! de cellule, rastérisation (`swash`).

pub mod loader;
pub mod metrics;
pub mod raster;

use rustty_vt::Attrs;

pub use loader::{EMBEDDED_FONT, FaceData, FontSet, GlyphRef};
pub use metrics::CellMetrics;
pub use raster::{GlyphBitmap, Rasterizer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl Variant {
    pub const ALL: [Variant; 4] = [
        Variant::Regular,
        Variant::Bold,
        Variant::Italic,
        Variant::BoldItalic,
    ];

    pub fn from_attrs(attrs: Attrs) -> Self {
        match (attrs.contains(Attrs::BOLD), attrs.contains(Attrs::ITALIC)) {
            (false, false) => Self::Regular,
            (true, false) => Self::Bold,
            (false, true) => Self::Italic,
            (true, true) => Self::BoldItalic,
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Regular => 0,
            Self::Bold => 1,
            Self::Italic => 2,
            Self::BoldItalic => 3,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("aucune police à chasse fixe trouvée")]
    NoMonospace,
    #[error("police illisible : {0}")]
    Invalid(String),
}
