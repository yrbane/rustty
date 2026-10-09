//! Aide partagée des tests hors écran : un renderer à la police embarquée,
//! le rendu d'un `Term` ajusté à sa grille et la comparaison aux images de
//! référence.

use std::path::PathBuf;

use rustty_config::Colors;
use rustty_render::{
    FontSet, Frame, GpuContext, OFFSCREEN_FORMAT, Offscreen, Palette, PaneFrame, PixelRect,
    Renderer,
};
use rustty_vt::Term;

const FONT_PX: f32 = 16.0;
pub const PADDING: u32 = 4;

pub fn renderer(ctx: &GpuContext) -> Renderer {
    Renderer::new(
        ctx,
        OFFSCREEN_FORMAT,
        FontSet::embedded(FONT_PX),
        Palette::from_config(&Colors::default(), false),
        PADDING,
    )
}

/// Rend `term` dans un viewport ajusté à sa grille et rend les pixels.
pub fn render_term(
    ctx: &GpuContext,
    r: &mut Renderer,
    term: &Term,
    focused: bool,
    opacity: f32,
) -> (Vec<u8>, u32, u32) {
    let m = r.metrics();
    let snap = term.snapshot();
    let width = snap.cols as u32 * m.width + 2 * PADDING;
    let height = snap.rows as u32 * m.height + 2 * PADDING;
    let target = Offscreen::new(ctx, width, height);
    let palette = Palette::from_config(&Colors::default(), false);
    let frame = Frame {
        viewport: (width, height),
        background: palette.background.with_alpha(opacity),
        panes: vec![PaneFrame {
            rect: PixelRect::new(0, 0, width, height),
            snapshot: &snap,
            focused,
        }],
        chrome: Default::default(),
    };
    r.render(ctx, target.view(), &frame);
    (target.read_rgba(ctx).unwrap(), width, height)
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.png"))
}

/// Compare à l'image de référence, ou la crée si elle manque / si UPDATE_GOLDEN=1.
pub fn assert_matches_golden(name: &str, px: &[u8], width: u32, height: u32) {
    let path = golden_path(name);
    let actual =
        image::RgbaImage::from_raw(width, height, px.to_vec()).expect("dimensions cohérentes");
    if std::env::var_os("UPDATE_GOLDEN").is_some() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        actual.save(&path).unwrap();
        eprintln!("image de référence écrite : {}", path.display());
        return;
    }
    let expected = image::open(&path).unwrap().to_rgba8();
    assert_eq!(
        expected.dimensions(),
        (width, height),
        "dimensions de {name}"
    );
    let mut total_diff = 0u64;
    let mut bad_pixels = 0u64;
    for (a, b) in expected.pixels().zip(actual.pixels()) {
        let d: [i32; 4] = std::array::from_fn(|i| (i32::from(a.0[i]) - i32::from(b.0[i])).abs());
        total_diff += d.iter().map(|&v| v as u64).sum::<u64>();
        if d.iter().any(|&v| v > 8) {
            bad_pixels += 1;
        }
    }
    let n = u64::from(width) * u64::from(height);
    let mean = total_diff as f64 / (n * 4) as f64;
    let bad_ratio = bad_pixels as f64 / n as f64;
    if mean > 1.0 || bad_ratio > 0.005 {
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/golden-actual")
            .join(format!("{name}.png"));
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        actual.save(&out).unwrap();
        panic!(
            "{name} diffère de la référence : moyenne {mean:.3}, {bad_ratio:.4} de pixels > 8 ; image obtenue : {}",
            out.display()
        );
    }
}

pub fn pixel(px: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}
