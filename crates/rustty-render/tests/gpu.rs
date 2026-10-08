mod common;

use rustty_render::{Offscreen, Rgba, clear};

#[test]
fn clear_and_read_back() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let target = Offscreen::new(&ctx, 8, 4);
    assert_eq!(target.size(), (8, 4));
    clear(&ctx, target.view(), Rgba::new(1.0, 0.0, 0.0, 1.0));
    let px = target.read_rgba(&ctx).unwrap();
    assert_eq!(px.len(), 8 * 4 * 4);
    assert!(
        px.chunks(4).all(|p| p == [255, 0, 0, 255]),
        "{:?}",
        &px[..16]
    );
}

#[test]
fn readback_handles_row_padding() {
    // 3 px de large : 12 octets par ligne, bien en dessous de l'alignement de 256.
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let target = Offscreen::new(&ctx, 3, 2);
    clear(&ctx, target.view(), Rgba::new(0.0, 1.0, 0.0, 1.0));
    let px = target.read_rgba(&ctx).unwrap();
    assert_eq!(px.len(), 3 * 2 * 4);
    assert!(px.chunks(4).all(|p| p == [0, 255, 0, 255]));
}

#[test]
fn zero_size_is_clamped_to_one_pixel() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let target = Offscreen::new(&ctx, 0, 0);
    assert_eq!(target.size(), (1, 1));
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 1.0, 0.5));
    let px = target.read_rgba(&ctx).unwrap();
    assert_eq!(px, vec![0, 0, 255, 128]);
}
