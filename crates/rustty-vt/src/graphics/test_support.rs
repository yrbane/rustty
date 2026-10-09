//! Aides de test partagées : PNG synthétiques et séquences APC graphiques.

use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ImageFormat, Rgba, RgbaImage};

/// PNG uni de `w` × `h` pixels, encodé en base64.
pub fn png_base64(w: u32, h: u32) -> String {
    let img = RgbaImage::from_pixel(w, h, Rgba([200, 30, 30, 255]));
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    STANDARD.encode(bytes)
}

/// Séquence APC complète `ESC _ G<keys>;<base64 PNG> ESC \`.
pub fn apc_png(keys: &str, w: u32, h: u32) -> String {
    format!("\x1b_G{keys};{}\x1b\\", png_base64(w, h))
}

/// Séquence APC sans charge utile.
pub fn apc(keys: &str) -> String {
    format!("\x1b_G{keys}\x1b\\")
}
