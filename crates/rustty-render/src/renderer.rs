//! Orchestration : du `Frame` aux passes wgpu, avec le cache de glyphes et
//! l'atlas. Une image = un effacement, deux lots de quads, les images des
//! panneaux, deux lots de glyphes.

use std::collections::{HashMap, HashSet};

use unicode_width::UnicodeWidthChar;

use crate::atlas::{AtlasPacker, AtlasRegion, atlas_size_for};
use crate::builtin::builtin_glyph;
use crate::color::Palette;
use crate::font::{CellMetrics, FontSet, Rasterizer, Variant};
use crate::frame::Frame;
use crate::gpu::{GpuContext, to_wgpu_color};
use crate::grid::{GlyphRequest, pane_instances};
use crate::images::pane_images;
use crate::pipeline::glyph::{AtlasTexture, GlyphInstance, GlyphPipeline};
use crate::pipeline::image::{ImagePipeline, ImageTextures};
use crate::pipeline::quad::{QuadInstance, QuadPipeline};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    ch: char,
    variant: Variant,
}

#[derive(Clone, Copy)]
struct CachedGlyph {
    region: AtlasRegion,
    left: i32,
    top: i32,
    is_color: bool,
}

pub struct Renderer {
    quads: QuadPipeline,
    glyphs: GlyphPipeline,
    images: ImagePipeline,
    image_textures: ImageTextures,
    atlas: AtlasTexture,
    packer: AtlasPacker,
    cache: HashMap<GlyphKey, Option<CachedGlyph>>,
    fonts: FontSet,
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
        Self {
            quads: QuadPipeline::new(&ctx.device, format),
            glyphs: GlyphPipeline::new(&ctx.device, format),
            images: ImagePipeline::new(&ctx.device, format),
            image_textures: ImageTextures::default(),
            atlas: AtlasTexture::new(&ctx.device, atlas_size),
            packer: AtlasPacker::new(atlas_size),
            cache: HashMap::new(),
            fonts,
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
        let metrics = self.metrics();
        let mut backgrounds = Vec::new();
        let mut requests = Vec::new();
        let mut overlay = Vec::new();
        let mut images = Vec::new();
        for pane in &frame.panes {
            images.extend(pane_images(pane, metrics, self.padding));
            let inst = pane_instances(pane, metrics, &self.palette, self.padding);
            backgrounds.extend(inst.backgrounds);
            requests.extend(inst.glyphs);
            overlay.extend(inst.decorations);
        }
        overlay.extend(frame.chrome.quads.iter().map(|q| {
            QuadInstance::new(
                q.rect.x as f32,
                q.rect.y as f32,
                q.rect.width as f32,
                q.rect.height as f32,
                q.color,
            )
        }));
        let mut chrome_requests = Vec::new();
        for text in &frame.chrome.texts {
            let mut col = 0u32;
            for ch in text.text.chars() {
                let x = (text.x + col * metrics.width) as f32;
                chrome_requests.push(GlyphRequest {
                    x,
                    y: text.y as f32,
                    ch,
                    variant: Variant::Regular,
                    color: text.color,
                    wide: false,
                });
                col += ch.width().unwrap_or(1) as u32;
            }
        }
        let (pane_glyphs, chrome_glyphs) = self.build_glyphs(ctx, &requests, &chrome_requests);
        let viewport = frame.viewport;
        let bg_batch = self.quads.prepare(&ctx.device, &backgrounds, viewport);
        let overlay_batch = self.quads.prepare(&ctx.device, &overlay, viewport);
        let image_batch = self.images.prepare(
            &ctx.device,
            &ctx.queue,
            &mut self.image_textures,
            &images,
            viewport,
        );
        let pane_batch = self
            .glyphs
            .prepare(&ctx.device, &self.atlas, &pane_glyphs, viewport);
        let chrome_batch = self
            .glyphs
            .prepare(&ctx.device, &self.atlas, &chrome_glyphs, viewport);
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
            if let Some(b) = &bg_batch {
                self.quads.draw(&mut pass, b);
            }
            if let Some(b) = &image_batch {
                self.images.draw(&mut pass, b, &self.image_textures);
            }
            if let Some(b) = &pane_batch {
                self.glyphs.draw(&mut pass, b);
            }
            if let Some(b) = &overlay_batch {
                self.quads.draw(&mut pass, b);
            }
            if let Some(b) = &chrome_batch {
                self.glyphs.draw(&mut pass, b);
            }
        }
        ctx.queue.submit(Some(encoder.finish()));
        // Seule une passe porteuse de panneaux dit quelles images sont encore
        // à l'écran : la passe du bandeau, sans panneau, ne vide pas le cache.
        if !frame.panes.is_empty() {
            let used: HashSet<u64> = images.iter().map(|d| d.image.id).collect();
            self.image_textures.retain(&used);
        }
    }

    /// Les instances d'une image. Si l'atlas est reconstruit en cours de route,
    /// les instances déjà produites pointent dans l'ancien atlas : une seconde
    /// passe, cache vidé, les recrée toutes de façon cohérente. Si l'ensemble
    /// de travail dépasse l'atlas, la seconde passe reconstruit encore et
    /// l'image reste partiellement fausse : on ne boucle pas.
    fn build_glyphs(
        &mut self,
        ctx: &GpuContext,
        pane: &[GlyphRequest],
        chrome: &[GlyphRequest],
    ) -> (Vec<GlyphInstance>, Vec<GlyphInstance>) {
        let mut result = (Vec::new(), Vec::new());
        for _pass in 0..2 {
            let before = self.rebuilds;
            result.0 = pane
                .iter()
                .filter_map(|r| self.glyph_instance(ctx, r))
                .collect();
            result.1 = chrome
                .iter()
                .filter_map(|r| self.glyph_instance(ctx, r))
                .collect();
            if self.rebuilds == before {
                break;
            }
        }
        result
    }

    /// L'instance à dessiner pour une demande, `None` si le caractère n'a
    /// aucun glyphe (il reste une cellule vide).
    pub(crate) fn glyph_instance(
        &mut self,
        ctx: &GpuContext,
        req: &GlyphRequest,
    ) -> Option<GlyphInstance> {
        let key = GlyphKey {
            ch: req.ch,
            variant: req.variant,
        };
        let cached = match self.cache.get(&key) {
            Some(c) => *c,
            None => {
                let c = self.load_glyph(ctx, key);
                self.cache.insert(key, c);
                c
            }
        }?;
        let metrics = self.metrics();
        let x = req.x + cached.left as f32;
        let y = req.y + metrics.baseline as f32 - cached.top as f32;
        Some(GlyphInstance::new(
            x,
            y,
            cached.region.width as f32,
            cached.region.height as f32,
            cached.region.uv(self.packer.size()),
            req.color,
            cached.is_color,
        ))
    }

    fn load_glyph(&mut self, ctx: &GpuContext, key: GlyphKey) -> Option<CachedGlyph> {
        let metrics = self.metrics();
        let bitmap = match builtin_glyph(key.ch, metrics) {
            Some(b) => b,
            None => {
                let glyph = self.fonts.glyph(key.ch, key.variant)?;
                let face = self.fonts.face_by_slot(glyph.slot).clone();
                self.rasterizer
                    .rasterize(&face, self.fonts.size_px(), glyph.glyph_id)?
            }
        };
        let region = match self.packer.insert(bitmap.width, bitmap.height) {
            Some(r) => r,
            None => {
                // Atlas plein : on repart de zéro. Les entrées du cache qui
                // pointaient dans l'ancien atlas sont invalidées.
                self.packer.clear();
                self.atlas.clear(&ctx.queue);
                self.cache.clear();
                self.rebuilds += 1;
                self.packer.insert(bitmap.width, bitmap.height)?
            }
        };
        self.atlas.upload(&ctx.queue, region, &bitmap.data);
        Some(CachedGlyph {
            region,
            left: bitmap.left,
            top: bitmap.top,
            is_color: bitmap.is_color,
        })
    }
}

#[cfg(test)]
mod tests;
