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
    let (a, _) = r.build_glyphs(&ctx, &first, &[], &[]);
    assert_eq!(a.len(), 26);
    let (b, _) = r.build_glyphs(&ctx, &second, &[], &[]);
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
fn an_icon_is_centered_on_its_point_whatever_its_ink_box() {
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
        256,
    );
    for ch in crate::font::glyphs::ALL {
        let icon = crate::frame::ChromeIcon {
            x: 100,
            y: 50,
            ch,
            size: 14,
            color: Rgba::new(1.0, 1.0, 1.0, 1.0),
        };
        let inst = r.icon_instance(&ctx, &icon).expect("glyphe présent");
        let (cx, cy) = (
            inst.pos[0] + inst.size[0] / 2.0,
            inst.pos[1] + inst.size[1] / 2.0,
        );
        assert!(
            (cx - 100.0).abs() <= 0.5 && (cy - 50.0).abs() <= 0.5,
            "{ch} : centre de l'encre ({cx}, {cy}) ≠ (100, 50)"
        );
    }
}

#[test]
fn icon_glyphs_scale_with_the_icon_size_not_the_text_size() {
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
        512,
    );
    let icon = |ch, size| crate::frame::ChromeIcon {
        x: 100,
        y: 50,
        ch,
        size,
        color: Rgba::new(1.0, 1.0, 1.0, 1.0),
    };
    let small = r
        .icon_instance(&ctx, &icon(crate::font::glyphs::CLOSE, 14))
        .unwrap();
    let big = r
        .icon_instance(&ctx, &icon(crate::font::glyphs::CLOSE, 28))
        .unwrap();
    assert!(
        big.size[1] >= small.size[1] * 1.6,
        "{} vs {}",
        big.size[1],
        small.size[1]
    );
    assert!(
        small.size[1] >= 8.0 && small.size[1] <= 14.0,
        "encre d'un ✕ Material à 14 px : {}",
        small.size[1]
    );
    assert_eq!(
        r.icon_instance(&ctx, &icon('a', 14)),
        None,
        "la face d'icônes ne couvre que les sept glyphes"
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

#[test]
fn full_atlas_doubles_and_keeps_every_glyph_of_the_frame() {
    let Ok(ctx) = GpuContext::headless() else {
        eprintln!("test GPU ignoré");
        return;
    };
    let mut r = Renderer::with_atlas_size(
        &ctx,
        OFFSCREEN_FORMAT,
        FontSet::embedded(64.0),
        Palette::from_config(&Colors::default(), false),
        0,
        512,
    );
    let page: Vec<_> = [Variant::Regular, Variant::Bold, Variant::Italic]
        .into_iter()
        .flat_map(|variant| {
            ('!'..='~').map(move |ch| GlyphRequest {
                variant,
                ..request(ch)
            })
        })
        .collect();
    let (instances, _) = r.build_glyphs(&ctx, &page, &[], &[]);
    assert_eq!(r.packer.size(), 1024, "l'atlas de 512 déborde : il double");
    assert_eq!(r.atlas.size(), 1024);
    let rebuilds = r.rebuilds;
    for (req, inst) in page.iter().zip(&instances) {
        assert_eq!(r.glyph_instance(&ctx, req), Some(*inst), "{:?}", req.ch);
    }
    assert_eq!(r.rebuilds, rebuilds, "aucune reconstruction de plus");
}

/// Contexte, renderer, cible et instantané d'un terminal `cols × rows`
/// rempli de texte ; `None` sans GPU.
fn fixture(
    cols: usize,
    rows: usize,
) -> Option<(
    GpuContext,
    Renderer,
    crate::gpu::Offscreen,
    rustty_vt::Snapshot,
)> {
    let Ok(ctx) = GpuContext::headless() else {
        eprintln!("test GPU ignoré");
        return None;
    };
    let r = Renderer::new(
        &ctx,
        OFFSCREEN_FORMAT,
        FontSet::embedded(16.0),
        Palette::from_config(&Colors::default(), false),
        0,
    );
    let m = r.metrics();
    let target = crate::gpu::Offscreen::new(&ctx, cols as u32 * m.width, rows as u32 * m.height);
    let mut t = rustty_vt::Term::new(cols, rows, 0);
    t.input(&vec![b'x'; cols * rows]);
    Some((ctx, r, target, t.snapshot()))
}

fn render_once(
    ctx: &GpuContext,
    r: &mut Renderer,
    target: &crate::gpu::Offscreen,
    snap: &rustty_vt::Snapshot,
) {
    let (w, h) = target.size();
    let frame = Frame {
        viewport: (w, h),
        background: Rgba::new(0.0, 0.0, 0.0, 1.0),
        panes: vec![crate::frame::PaneFrame {
            rect: crate::frame::PixelRect::new(0, 0, w, h),
            snapshot: snap,
            focused: true,
            link_hover: None,
        }],
        chrome: Default::default(),
    };
    r.render(ctx, target.view(), &frame);
}

#[test]
fn a_second_identical_frame_creates_no_gpu_objects() {
    let Some((ctx, mut r, target, snap)) = fixture(40, 10) else {
        return;
    };
    render_once(&ctx, &mut r, &target, &snap);
    render_once(&ctx, &mut r, &target, &snap);
    let after_two = r.gpu_objects();
    render_once(&ctx, &mut r, &target, &snap);
    assert_eq!(r.gpu_objects(), after_two);
}

#[test]
fn buffers_grow_once_for_a_larger_frame_then_stay() {
    let Some((ctx, mut r, target, small)) = fixture(10, 2) else {
        return;
    };
    render_once(&ctx, &mut r, &target, &small);
    render_once(&ctx, &mut r, &target, &small);
    let before = r.gpu_objects();
    let mut t = rustty_vt::Term::new(120, 40, 0);
    t.input(&vec![b'x'; 120 * 40]);
    let big = t.snapshot();
    let m = r.metrics();
    let big_target = crate::gpu::Offscreen::new(&ctx, 120 * m.width, 40 * m.height);
    render_once(&ctx, &mut r, &big_target, &big);
    let grown = r.gpu_objects();
    assert!(grown > before, "croissance");
    render_once(&ctx, &mut r, &big_target, &big);
    assert_eq!(r.gpu_objects(), grown, "stable ensuite");
}
