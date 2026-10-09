//! Cache de glyphes du renderer : rastérisation, placement dans l'atlas et
//! reconstruction (agrandissement, puis vidage à la taille maximale).

use super::Renderer;
use crate::atlas::{AtlasPacker, AtlasRegion, DEFAULT_ATLAS_SIZE};
use crate::builtin::builtin_glyph;
use crate::font::Variant;
use crate::gpu::GpuContext;
use crate::grid::GlyphRequest;
use crate::pipeline::glyph::{AtlasTexture, GlyphInstance};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct GlyphKey {
    ch: char,
    variant: Variant,
}

#[derive(Clone, Copy)]
pub(super) struct CachedGlyph {
    region: AtlasRegion,
    left: i32,
    top: i32,
    is_color: bool,
}

/// Passes de construction des glyphes d'une image : deux agrandissements de
/// l'atlas, un vidage à la taille maximale, puis la passe finale.
const MAX_GLYPH_PASSES: usize = 4;

impl Renderer {
    /// Les instances d'une image. Si l'atlas est reconstruit en cours de route
    /// (agrandi, ou vidé à sa taille maximale), les instances déjà produites
    /// pointent dans l'ancien atlas : une nouvelle passe, cache vidé, les
    /// recrée toutes de façon cohérente. L'atlas double au plus deux fois
    /// (512 → 2048) ; si l'ensemble de travail dépasse même 2048, la dernière
    /// passe reconstruit encore et l'image reste partiellement fausse : on ne
    /// boucle pas.
    pub(super) fn build_glyphs(
        &mut self,
        ctx: &GpuContext,
        pane: &[GlyphRequest],
        chrome: &[GlyphRequest],
    ) -> (Vec<GlyphInstance>, Vec<GlyphInstance>) {
        let mut result = (Vec::new(), Vec::new());
        for _pass in 0..MAX_GLYPH_PASSES {
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

    /// Atlas plein : il double jusqu'à `DEFAULT_ATLAS_SIZE`, puis, à cette
    /// taille, repart de zéro. Les entrées du cache qui pointaient dans
    /// l'ancien atlas sont invalidées dans les deux cas.
    fn rebuild_atlas(&mut self, ctx: &GpuContext) {
        let size = self.packer.size();
        if size < DEFAULT_ATLAS_SIZE {
            let grown = (size * 2).min(DEFAULT_ATLAS_SIZE);
            self.atlas = AtlasTexture::new(&ctx.device, grown);
            self.packer = AtlasPacker::new(grown);
        } else {
            self.packer.clear();
            self.atlas.clear(&ctx.queue);
        }
        self.cache.clear();
        self.rebuilds += 1;
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
                self.rebuild_atlas(ctx);
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
