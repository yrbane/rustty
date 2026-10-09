//! Tests du renderer qui touchent à son état interne (atlas, cache).

use super::*;
use crate::color::Rgba;
use crate::gpu::OFFSCREEN_FORMAT;
use rustty_config::Colors;

fn request(ch: char) -> GlyphRequest {
    GlyphRequest {
        x: 0.0,
        y: 0.0,
        ch,
        variant: Variant::Regular,
        color: Rgba::new(1.0, 1.0, 1.0, 1.0),
        wide: false,
    }
}

#[test]
fn instances_of_one_frame_stay_consistent_after_an_atlas_rebuild() {
    let Ok(ctx) = GpuContext::headless() else {
        eprintln!("test GPU ignoré");
        return;
    };
    let palette = Palette::from_config(&Colors::default(), false);
    let mut r = Renderer::with_atlas_size(
        &ctx,
        OFFSCREEN_FORMAT,
        FontSet::embedded(16.0),
        palette,
        0,
        80,
    );
    let first: Vec<_> = "abcdefghijklmnopqrstuvwxyz".chars().map(request).collect();
    let second: Vec<_> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().map(request).collect();
    let (a, _) = r.build_glyphs(&ctx, &first, &[]);
    assert_eq!(a.len(), 26);
    let (b, _) = r.build_glyphs(&ctx, &second, &[]);
    assert!(r.rebuilds >= 1, "un atlas de 80 px ne tient pas 52 glyphes");
    let rebuilds = r.rebuilds;
    for (req, inst) in second.iter().zip(&b) {
        assert_eq!(r.glyph_instance(&ctx, req), Some(*inst), "{:?}", req.ch);
    }
    assert_eq!(
        r.rebuilds, rebuilds,
        "toutes les instances de l'image pointaient dans l'atlas final"
    );
}

#[test]
fn atlas_overflow_is_recovered() {
    let Ok(ctx) = GpuContext::headless() else {
        eprintln!("test GPU ignoré");
        return;
    };
    let mut r = Renderer::with_atlas_size(
        &ctx,
        OFFSCREEN_FORMAT,
        FontSet::embedded(16.0),
        Palette::from_config(&Colors::default(), false),
        0,
        64,
    );
    let mut instances = Vec::new();
    for ch in "abcdefghijklmnopqrstuvwxyz0123456789".chars() {
        let req = GlyphRequest {
            x: 0.0,
            y: 0.0,
            ch,
            variant: Variant::Regular,
            color: Rgba::new(1.0, 1.0, 1.0, 1.0),
            wide: false,
        };
        if let Some(i) = r.glyph_instance(&ctx, &req) {
            instances.push(i);
        }
    }
    assert_eq!(
        instances.len(),
        36,
        "chaque glyphe a été placé, au prix de reconstructions"
    );
    assert!(r.rebuilds >= 1, "un atlas de 64 px déborde forcément");
    let again = r.glyph_instance(
        &ctx,
        &GlyphRequest {
            x: 0.0,
            y: 0.0,
            ch: 'a',
            variant: Variant::Regular,
            color: Rgba::new(1.0, 1.0, 1.0, 1.0),
            wide: false,
        },
    );
    assert!(again.is_some());
}
