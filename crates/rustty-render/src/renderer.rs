//! Orchestration : du `Frame` aux passes wgpu, avec le cache de glyphes et
//! l'atlas. Une image = un effacement, deux lots de quads, les images des
//! panneaux, deux lots de glyphes.

use std::collections::{HashMap, HashSet};

use unicode_width::UnicodeWidthChar;

use crate::atlas::{AtlasPacker, atlas_size_for};
use crate::color::Palette;
use crate::font::{CellMetrics, FontSet, IconFace, Rasterizer, Variant};
use crate::frame::{Chrome, ChromeText, Frame};
use crate::gpu::{GpuContext, to_wgpu_color};
use crate::grid::{GlyphRequest, pane_instances};
use crate::images::ImageDraw;
use crate::images::pane_images;
use crate::pipeline::common::{GlobalsBinding, InstanceBuffer};
use crate::pipeline::glyph::{AtlasTexture, GlyphPipeline};
use crate::pipeline::image::{ImagePipeline, ImageTextures};
use crate::pipeline::quad::{QuadInstance, QuadPipeline};
use glyph_cache::{CachedGlyph, GlyphKey};

pub struct Renderer {
    quads: QuadPipeline,
    glyphs: GlyphPipeline,
    images: ImagePipeline,
    image_textures: ImageTextures,
    atlas: AtlasTexture,
    /// Bind group de l'atlas, refait seulement quand l'atlas l'est.
    atlas_group: wgpu::BindGroup,
    /// Uniforme du viewport, partagé par les trois pipelines.
    globals: GlobalsBinding,
    /// Tampons d'instances persistants, un par usage.
    buffers: Buffers,
    /// Objets GPU (tampons, bind groups) créés depuis la construction, hors
    /// textures d'image : en régime établi, une image n'en crée aucun.
    gpu_objects: u32,
    packer: AtlasPacker,
    cache: HashMap<GlyphKey, Option<CachedGlyph>>,
    fonts: FontSet,
    /// Face des icônes de la barre de titre.
    icons: IconFace,
    rasterizer: Rasterizer,
    palette: Palette,
    padding: u32,
    /// Nombre de reconstructions de l'atlas (diagnostic et tests).
    pub(crate) rebuilds: u32,
}

impl Renderer {
    pub fn new(
        ctx: &GpuContext,
        format: wgpu::TextureFormat,
        fonts: FontSet,
        palette: Palette,
        padding: u32,
    ) -> Self {
        let size = atlas_size_for(fonts.metrics());
        Self::with_atlas_size(ctx, format, fonts, palette, padding, size)
    }

    pub(crate) fn with_atlas_size(
        ctx: &GpuContext,
        format: wgpu::TextureFormat,
        fonts: FontSet,
        palette: Palette,
        padding: u32,
        atlas_size: u32,
    ) -> Self {
        let device = &ctx.device;
        let globals_layout = GlobalsBinding::layout(device);
        let glyphs = GlyphPipeline::new(device, format, &globals_layout);
        let atlas = AtlasTexture::new(device, atlas_size);
        let atlas_group = glyphs.atlas_group(device, &atlas);
        let buffers = Buffers::new(device);
        Self {
            quads: QuadPipeline::new(device, format, &globals_layout),
            glyphs,
            images: ImagePipeline::new(device, format, &globals_layout),
            image_textures: ImageTextures::default(),
            atlas,
            atlas_group,
            globals: GlobalsBinding::new(device, &globals_layout, (1, 1)),
            buffers,
            // Cinq tampons, l'uniforme et ses deux objets, le bind group de l'atlas.
            gpu_objects: 5 + 2 + 1,
            packer: AtlasPacker::new(atlas_size),
            cache: HashMap::new(),
            fonts,
            icons: IconFace::new(),
            rasterizer: Rasterizer::new(),
            palette,
            padding,
            rebuilds: 0,
        }
    }

    pub fn metrics(&self) -> CellMetrics {
        self.fonts.metrics()
    }

    pub fn fonts(&self) -> &FontSet {
        &self.fonts
    }

    /// Nombre de textures d'image en cache (tests).
    #[doc(hidden)]
    pub fn image_texture_count(&self) -> usize {
        self.image_textures.len()
    }

    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Objets GPU créés depuis la construction, hors textures d'image (tests).
    #[doc(hidden)]
    pub fn gpu_objects(&self) -> u32 {
        self.gpu_objects
    }

    /// Efface la cible avec `frame.background` puis dessine l'image.
    pub fn render(&mut self, ctx: &GpuContext, view: &wgpu::TextureView, frame: &Frame) {
        let load = wgpu::LoadOp::Clear(to_wgpu_color(frame.background));
        self.draw(ctx, view, frame, load);
    }

    /// Dessine par-dessus ce que la cible contient déjà (`frame.background`
    /// est ignoré) : une passe par taille de police dans la même image.
    pub fn render_onto(&mut self, ctx: &GpuContext, view: &wgpu::TextureView, frame: &Frame) {
        self.draw(ctx, view, frame, wgpu::LoadOp::Load);
    }

    fn draw(
        &mut self,
        ctx: &GpuContext,
        view: &wgpu::TextureView,
        frame: &Frame,
        load: wgpu::LoadOp<wgpu::Color>,
    ) {
        let collected = self.collect(frame);
        let image_ids = self.upload(ctx, frame, &collected);
        self.pass(ctx, view, load, &image_ids);
        // Seule une passe porteuse de panneaux dit quelles images sont encore
        // à l'écran : la passe du bandeau, sans panneau, ne vide pas le cache.
        if !frame.panes.is_empty() {
            let used: HashSet<u64> = collected.images.iter().map(|d| d.image.id).collect();
            self.image_textures.retain(&used);
        }
    }

    /// Les instances CPU d'une image : fonds et glyphes des panneaux, bandes
    /// d'image, décor (soulignements, chrome), textes du chrome.
    fn collect(&self, frame: &Frame) -> Collected {
        let metrics = self.metrics();
        let mut c = Collected::default();
        for pane in &frame.panes {
            c.images.extend(pane_images(pane, metrics, self.padding));
            let inst = pane_instances(pane, metrics, &self.palette, self.padding);
            c.backgrounds.extend(inst.backgrounds);
            c.requests.extend(inst.glyphs);
            c.overlay.extend(inst.decorations);
        }
        c.overlay.extend(chrome_quads(&frame.chrome));
        c.chrome_requests = chrome_glyph_requests(&frame.chrome.texts, metrics);
        c
    }

    /// Glyphes rasterisés dans l'atlas, textures d'image envoyées, tampons
    /// et uniforme écrits. Rend les identifiants d'image, dans l'ordre des
    /// instances.
    fn upload(&mut self, ctx: &GpuContext, frame: &Frame, c: &Collected) -> Vec<u64> {
        let (pane_glyphs, chrome_glyphs) =
            self.build_glyphs(ctx, &c.requests, &c.chrome_requests, &frame.chrome.icons);
        let staged =
            self.images
                .stage(&ctx.device, &ctx.queue, &mut self.image_textures, &c.images);
        self.globals.ensure(&ctx.queue, frame.viewport);
        let (d, q) = (&ctx.device, &ctx.queue);
        let b = &mut self.buffers;
        let grown = [
            b.backgrounds.write(d, q, &c.backgrounds),
            b.overlay.write(d, q, &c.overlay),
            b.images.write(d, q, &staged.instances),
            b.pane_glyphs.write(d, q, &pane_glyphs),
            b.chrome_glyphs.write(d, q, &chrome_glyphs),
        ];
        self.gpu_objects += grown.iter().filter(|&&g| g).count() as u32;
        staged.ids
    }

    /// La passe de rendu, dans l'ordre : fonds, images, glyphes des
    /// panneaux, décor, glyphes du chrome.
    fn pass(
        &self,
        ctx: &GpuContext,
        view: &wgpu::TextureView,
        load: wgpu::LoadOp<wgpu::Color>,
        image_ids: &[u64],
    ) {
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rustty-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rustty-frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            let (b, g, atlas) = (&self.buffers, &self.globals, &self.atlas_group);
            self.quads.draw(&mut pass, g, &b.backgrounds);
            self.images
                .draw(&mut pass, g, &self.image_textures, &b.images, image_ids);
            self.glyphs.draw(&mut pass, g, atlas, &b.pane_glyphs);
            self.quads.draw(&mut pass, g, &b.overlay);
            self.glyphs.draw(&mut pass, g, atlas, &b.chrome_glyphs);
        }
        ctx.queue.submit(Some(encoder.finish()));
    }
}

/// Les instances CPU d'une image, avant tout envoi au GPU.
#[derive(Default)]
struct Collected {
    backgrounds: Vec<QuadInstance>,
    requests: Vec<GlyphRequest>,
    overlay: Vec<QuadInstance>,
    images: Vec<ImageDraw>,
    chrome_requests: Vec<GlyphRequest>,
}

/// Les rectangles du chrome, plats puis stylés.
fn chrome_quads(chrome: &Chrome) -> impl Iterator<Item = QuadInstance> + '_ {
    let flat = chrome.quads.iter().map(|q| {
        QuadInstance::new(
            q.rect.x as f32,
            q.rect.y as f32,
            q.rect.width as f32,
            q.rect.height as f32,
            q.color,
        )
    });
    let styled = chrome.shapes.iter().map(|s| {
        QuadInstance::styled(
            s.rect.x as f32,
            s.rect.y as f32,
            s.rect.width as f32,
            s.rect.height as f32,
            s.style,
        )
    });
    flat.chain(styled)
}

/// Les textes du chrome, un glyphe régulier par caractère, avancés de la
/// largeur du caractère.
fn chrome_glyph_requests(texts: &[ChromeText], metrics: CellMetrics) -> Vec<GlyphRequest> {
    let mut out = Vec::new();
    for text in texts {
        let mut col = 0u32;
        for ch in text.text.chars() {
            out.push(GlyphRequest {
                x: (text.x + col * metrics.width) as f32,
                y: text.y as f32,
                ch,
                variant: Variant::Regular,
                color: text.color,
                wide: false,
            });
            col += ch.width().unwrap_or(1) as u32;
        }
    }
    out
}

/// Les cinq tampons d'instances d'une image, dans l'ordre de dessin.
struct Buffers {
    backgrounds: InstanceBuffer,
    images: InstanceBuffer,
    pane_glyphs: InstanceBuffer,
    overlay: InstanceBuffer,
    chrome_glyphs: InstanceBuffer,
}

impl Buffers {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            backgrounds: InstanceBuffer::new(device, "rustty-backgrounds"),
            images: InstanceBuffer::new(device, "rustty-images"),
            pane_glyphs: InstanceBuffer::new(device, "rustty-pane-glyphs"),
            overlay: InstanceBuffer::new(device, "rustty-overlay"),
            chrome_glyphs: InstanceBuffer::new(device, "rustty-chrome-glyphs"),
        }
    }
}

mod glyph_cache;
#[cfg(test)]
mod tests;
