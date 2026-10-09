//! Dessin des images du protocole graphique kitty, hors écran.

mod common;
mod support;

use std::io::Cursor;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use rustty_render::{GpuContext, Renderer};
use rustty_vt::Term;
use support::{PADDING, assert_matches_golden, pixel, render_term, renderer};

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

/// PNG 4×2 en quatre quadrants de 2×1 pixels : rouge, vert / bleu, blanc.
fn quadrants_png() -> String {
    let img = image::RgbaImage::from_fn(4, 2, |x, y| {
        image::Rgba(match (x < 2, y == 0) {
            (true, true) => RED,
            (false, true) => GREEN,
            (true, false) => BLUE,
            (false, false) => WHITE,
        })
    });
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    STANDARD.encode(bytes)
}

/// Un `Term` 8×3 dont les cellules ont la taille de celles du renderer, avec
/// l'image étirée sur 4×2 cellules à partir de la colonne 1.
fn term_with_image(r: &Renderer) -> Term {
    let m = r.metrics();
    let mut term = Term::new(8, 3, 0);
    term.set_cell_pixels(m.width, m.height);
    term.input(b"A");
    term.input(format!("\x1b_Ga=T,f=100,c=4,r=2;{}\x1b\\", quadrants_png()).as_bytes());
    term.input(b"\r\nok\x1b[?25l");
    term
}

fn gpu() -> Option<GpuContext> {
    common::gpu_or_skip()
}

#[test]
fn image_strip() {
    let Some(ctx) = gpu() else {
        return;
    };
    let mut r = renderer(&ctx);
    let term = term_with_image(&r);
    let (px, w, h) = render_term(&ctx, &mut r, &term, true, 1.0);
    let m = r.metrics();
    // Un point bien à l'intérieur de chaque quadrant (hors du flou bilinéaire).
    let at = |col: f32, row: f32| {
        let x = PADDING + (col * m.width as f32) as u32;
        let y = PADDING + (row * m.height as f32) as u32;
        pixel(&px, w, x, y)
    };
    assert_eq!(at(1.6, 0.2), RED, "quadrant haut gauche");
    assert_eq!(at(4.4, 0.2), GREEN, "quadrant haut droit");
    assert_eq!(at(1.6, 1.8), BLUE, "quadrant bas gauche");
    assert_eq!(at(4.4, 1.8), WHITE, "quadrant bas droit");
    assert_ne!(at(6.5, 0.5), RED, "rien à droite de l'image");
    assert_matches_golden("image_strip", &px, w, h);
}

#[test]
fn renderer_drops_textures_of_images_no_longer_drawn() {
    let Some(ctx) = gpu() else {
        return;
    };
    let mut r = renderer(&ctx);
    assert_eq!(r.image_texture_count(), 0);
    let with_image = term_with_image(&r);
    render_term(&ctx, &mut r, &with_image, true, 1.0);
    assert_eq!(r.image_texture_count(), 1);
    render_term(&ctx, &mut r, &with_image, true, 1.0);
    assert_eq!(r.image_texture_count(), 1, "texture réutilisée");
    let other = term_with_image(&r);
    render_term(&ctx, &mut r, &other, true, 1.0);
    assert_eq!(r.image_texture_count(), 1, "l'ancienne image est oubliée");
    render_term(&ctx, &mut r, &Term::new(8, 3, 0), true, 1.0);
    assert_eq!(r.image_texture_count(), 0);
}
