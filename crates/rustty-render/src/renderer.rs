//! Orchestration : du `Frame` aux passes wgpu, avec le cache de glyphes et
//! l'atlas. Une image = un effacement, deux lots de quads, deux lots de glyphes.

use std::collections::HashMap;

use unicode_width::UnicodeWidthChar;

use crate::atlas::{AtlasPacker, AtlasRegion, DEFAULT_ATLAS_SIZE};
use crate::builtin::builtin_glyph;
use crate::color::Palette;
use crate::font::{CellMetrics, FontSet, Rasterizer, Variant};
use crate::frame::Frame;
use crate::gpu::{GpuContext, to_wgpu_color};
use crate::grid::{GlyphRequest, pane_instances};
use crate::pipeline::glyph::{AtlasTexture, GlyphInstance, GlyphPipeline};
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
        Self::with_atlas_size(ctx, format, fonts, palette, padding, DEFAULT_ATLAS_SIZE)
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
        for pane in &frame.panes {
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
mod tests {
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
}
