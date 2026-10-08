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

use rustty_render::{QuadInstance, QuadPipeline};

fn pixel(px: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

fn load_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    })
}

#[test]
fn quads_are_drawn_in_pixel_coordinates_with_alpha_blending() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let target = Offscreen::new(&ctx, 8, 4);
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 0.0, 1.0));
    let pipeline = QuadPipeline::new(&ctx.device, rustty_render::OFFSCREEN_FORMAT);
    let instances = [
        QuadInstance::new(2.0, 1.0, 4.0, 2.0, Rgba::new(1.0, 0.0, 0.0, 1.0)),
        QuadInstance::new(0.0, 0.0, 2.0, 1.0, Rgba::new(0.0, 0.0, 1.0, 0.5)),
    ];
    let batch = pipeline
        .prepare(&ctx.device, &instances, target.size())
        .unwrap();
    assert_eq!(batch.len(), 2);
    let mut encoder = ctx.device.create_command_encoder(&Default::default());
    {
        let mut pass = load_pass(&mut encoder, target.view());
        pipeline.draw(&mut pass, &batch);
    }
    ctx.queue.submit(Some(encoder.finish()));
    let px = target.read_rgba(&ctx).unwrap();
    assert_eq!(
        pixel(&px, 8, 2, 1),
        [255, 0, 0, 255],
        "coin haut-gauche du quad rouge"
    );
    assert_eq!(
        pixel(&px, 8, 5, 2),
        [255, 0, 0, 255],
        "coin bas-droit inclus"
    );
    assert_eq!(pixel(&px, 8, 6, 1), [0, 0, 0, 255], "hors du quad");
    assert_eq!(pixel(&px, 8, 1, 3), [0, 0, 0, 255]);
    let blended = pixel(&px, 8, 0, 0);
    assert!(
        blended[2] >= 126 && blended[2] <= 129 && blended[0] == 0,
        "bleu à 50 % sur noir : {blended:?}"
    );
}

#[test]
fn empty_batch_is_none() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let pipeline = QuadPipeline::new(&ctx.device, rustty_render::OFFSCREEN_FORMAT);
    assert!(pipeline.prepare(&ctx.device, &[], (8, 8)).is_none());
}
