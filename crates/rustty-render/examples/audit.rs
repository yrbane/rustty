//! Micro-benchmarks d'audit du rendu (lancer en release).
use std::time::Instant;

use rustty_config::Colors;
use rustty_render::{
    CellMetrics, FontSet, Frame, GpuContext, OFFSCREEN_FORMAT, Offscreen, Palette, PaneFrame,
    PixelRect, Renderer, pane_instances,
};
use rustty_vt::Term;

/// Meilleur temps par itération sur `REPS` répétitions (le bruit ne fait que rallonger).
const REPS: u32 = 5;
fn timed<R>(label: &str, iters: u32, mut f: impl FnMut() -> R) {
    let mut best = f64::MAX;
    for _ in 0..REPS {
        let t0 = Instant::now();
        for _ in 0..iters {
            std::hint::black_box(f());
        }
        best = best.min(t0.elapsed().as_secs_f64() / f64::from(iters));
    }
    println!("{label:<48} {:>10.1} µs", best * 1e6);
}

fn main() {
    let mut t = Term::new(200, 60, 0);
    let fill: Vec<u8> = (0..60)
        .flat_map(|i| {
            format!("\x1b[3{}m\x1b[4m{i:06} {}\r\n", i % 8, "mot ".repeat(48)).into_bytes()
        })
        .collect();
    t.input(&fill);
    let snap = t.snapshot();
    let palette = Palette::from_config(&Colors::default(), false);
    let m = CellMetrics {
        width: 10,
        height: 20,
        baseline: 16,
        underline_y: 18,
        underline_thickness: 1,
        strike_y: 10,
    };
    let pane = PaneFrame {
        rect: PixelRect::new(0, 0, 2008, 1208),
        snapshot: &snap,
        focused: true,
        link_hover: None,
    };
    timed("pane_instances 200x60 stylé", 500, || {
        pane_instances(&pane, m, &palette, 4)
    });
    let ctx = GpuContext::headless().expect("gpu");
    let mut r = Renderer::new(
        &ctx,
        OFFSCREEN_FORMAT,
        FontSet::embedded(16.0),
        palette.clone(),
        4,
    );
    let mm = r.metrics();
    let (w, h) = (200 * mm.width + 8, 60 * mm.height + 8);
    let target = Offscreen::new(&ctx, w, h);
    let frame = Frame {
        viewport: (w, h),
        background: palette.background,
        panes: vec![PaneFrame {
            rect: PixelRect::new(0, 0, w, h),
            snapshot: &snap,
            focused: true,
            link_hover: None,
        }],
        chrome: Default::default(),
    };
    r.render(&ctx, target.view(), &frame);
    timed("Renderer::render 200x60 hors écran (lavapipe)", 60, || {
        r.render(&ctx, target.view(), &frame)
    });
}
