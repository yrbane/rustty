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

use rustty_render::{GlobalsBinding, InstanceBuffer, QuadInstance, QuadPipeline};

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
    let layout = GlobalsBinding::layout(&ctx.device);
    let pipeline = QuadPipeline::new(&ctx.device, rustty_render::OFFSCREEN_FORMAT, &layout);
    let instances = [
        QuadInstance::new(2.0, 1.0, 4.0, 2.0, Rgba::new(1.0, 0.0, 0.0, 1.0)),
        QuadInstance::new(0.0, 0.0, 2.0, 1.0, Rgba::new(0.0, 0.0, 1.0, 0.5)),
    ];
    let globals = GlobalsBinding::new(&ctx.device, &layout, target.size());
    let mut buffer = InstanceBuffer::new(&ctx.device, "test-quads");
    buffer.write(&ctx.device, &ctx.queue, &instances);
    assert_eq!(buffer.count(), 2);
    let mut encoder = ctx.device.create_command_encoder(&Default::default());
    {
        let mut pass = load_pass(&mut encoder, target.view());
        pipeline.draw(&mut pass, &globals, &buffer);
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
fn an_instance_buffer_grows_by_doubling_and_never_shrinks() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut buffer = InstanceBuffer::new(&ctx.device, "test-growth");
    let one = [QuadInstance::new(
        0.0,
        0.0,
        1.0,
        1.0,
        Rgba::new(1.0, 1.0, 1.0, 1.0),
    )];
    assert!(
        !buffer.write(&ctx.device, &ctx.queue, &one),
        "tient d'emblée"
    );
    let many = vec![one[0]; 1000];
    assert!(buffer.write(&ctx.device, &ctx.queue, &many), "grandit");
    assert!(!buffer.write(&ctx.device, &ctx.queue, &many), "puis reste");
    assert!(
        !buffer.write(&ctx.device, &ctx.queue, &one),
        "ne rétrécit pas"
    );
    assert_eq!(buffer.count(), 1);
    assert!(!buffer.write::<QuadInstance>(&ctx.device, &ctx.queue, &[]));
    assert!(buffer.is_empty());
}

use rustty_render::{AtlasRegion, AtlasTexture, GlyphInstance, GlyphPipeline};

fn draw_glyphs(
    ctx: &rustty_render::GpuContext,
    target: &Offscreen,
    atlas: &AtlasTexture,
    instances: &[GlyphInstance],
) -> Vec<u8> {
    let layout = GlobalsBinding::layout(&ctx.device);
    let pipeline = GlyphPipeline::new(&ctx.device, rustty_render::OFFSCREEN_FORMAT, &layout);
    let globals = GlobalsBinding::new(&ctx.device, &layout, target.size());
    let atlas_group = pipeline.atlas_group(&ctx.device, atlas);
    let mut buffer = InstanceBuffer::new(&ctx.device, "test-glyphs");
    buffer.write(&ctx.device, &ctx.queue, instances);
    let mut encoder = ctx.device.create_command_encoder(&Default::default());
    {
        let mut pass = load_pass(&mut encoder, target.view());
        pipeline.draw(&mut pass, &globals, &atlas_group, &buffer);
    }
    ctx.queue.submit(Some(encoder.finish()));
    target.read_rgba(ctx).unwrap()
}

#[test]
fn monochrome_glyph_takes_the_instance_color() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let target = Offscreen::new(&ctx, 6, 6);
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 0.0, 1.0));
    let atlas = AtlasTexture::new(&ctx.device, 8);
    let region = AtlasRegion {
        x: 0,
        y: 0,
        width: 2,
        height: 2,
    };
    // Masque : plein en haut à gauche et en bas à droite, vide ailleurs.
    atlas.upload(
        &ctx.queue,
        region,
        &[
            255, 255, 255, 255, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255, 255, 255,
        ],
    );
    let px = draw_glyphs(
        &ctx,
        &target,
        &atlas,
        &[GlyphInstance::new(
            1.0,
            2.0,
            2.0,
            2.0,
            region.uv(8),
            Rgba::new(1.0, 0.0, 0.0, 1.0),
            false,
        )],
    );
    assert_eq!(
        pixel(&px, 6, 1, 2),
        [255, 0, 0, 255],
        "texel plein → couleur d'instance"
    );
    assert_eq!(
        pixel(&px, 6, 2, 2),
        [0, 0, 0, 255],
        "texel vide → fond inchangé"
    );
    assert_eq!(pixel(&px, 6, 2, 3), [255, 0, 0, 255]);
    assert_eq!(pixel(&px, 6, 0, 0), [0, 0, 0, 255]);
}

#[test]
fn color_glyph_keeps_its_own_colors() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let target = Offscreen::new(&ctx, 4, 4);
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 0.0, 1.0));
    let atlas = AtlasTexture::new(&ctx.device, 4);
    let region = AtlasRegion {
        x: 1,
        y: 1,
        width: 1,
        height: 1,
    };
    atlas.upload(&ctx.queue, region, &[0, 255, 0, 255]);
    let px = draw_glyphs(
        &ctx,
        &target,
        &atlas,
        &[GlyphInstance::new(
            0.0,
            0.0,
            1.0,
            1.0,
            region.uv(4),
            Rgba::new(1.0, 0.0, 0.0, 1.0),
            true,
        )],
    );
    assert_eq!(
        pixel(&px, 4, 0, 0),
        [0, 255, 0, 255],
        "la texture l'emporte sur la couleur d'instance"
    );
}

#[test]
fn atlas_clear_wipes_previous_glyphs() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let target = Offscreen::new(&ctx, 2, 2);
    clear(&ctx, target.view(), Rgba::new(0.0, 0.0, 0.0, 1.0));
    let atlas = AtlasTexture::new(&ctx.device, 2);
    let region = AtlasRegion {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    };
    atlas.upload(&ctx.queue, region, &[255, 255, 255, 255]);
    atlas.clear(&ctx.queue);
    let px = draw_glyphs(
        &ctx,
        &target,
        &atlas,
        &[GlyphInstance::new(
            0.0,
            0.0,
            1.0,
            1.0,
            region.uv(2),
            Rgba::new(1.0, 1.0, 1.0, 1.0),
            false,
        )],
    );
    assert_eq!(pixel(&px, 2, 0, 0), [0, 0, 0, 255]);
}
