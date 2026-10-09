//! Protocole graphique kitty : analyse des commandes APC `G…` et
//! réassemblage des charges utiles fragmentées en morceaux.
//!
//! Module pur (aucune E/S) : les étapes suivantes y branchent le décodage
//! d'image et le stockage.

mod chunks;
mod command;
mod image;

pub use self::image::{ImageData, ImageStrip, MAX_SIDE, Placement, decode};
pub use chunks::{ChunkResult, Chunks, GraphicsError, MAX_PAYLOAD};
pub use command::{Action, Format, GraphicsCommand, parse};
