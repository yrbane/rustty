//! Un renderer par taille de police : celui de la configuration (barre
//! d'onglets, bandeaux, panneaux non zoomés) et un par taille zoomée en
//! usage, créé à la demande et libéré quand plus aucun panneau ne s'en sert.

use std::collections::{BTreeSet, HashMap};

use rustty_render::{CellMetrics, FontSet, GpuContext, Palette, Renderer};

use crate::pane_fonts::SizeKey;

/// Taille de police de la config en points typographiques → pixels à 96 dpi.
const PT_TO_PX: f32 = 96.0 / 72.0;

/// Ce qu'il faut pour créer un renderer.
pub struct RendererSpec<'a> {
    pub ctx: &'a GpuContext,
    pub format: wgpu::TextureFormat,
    pub family: &'a str,
    pub scale_factor: f64,
    pub palette: &'a Palette,
    pub padding: u32,
}

impl RendererSpec<'_> {
    fn build(&self, key: SizeKey) -> Renderer {
        Renderer::new(
            self.ctx,
            self.format,
            load_fonts(self.family, key.size(), self.scale_factor),
            self.palette.clone(),
            self.padding,
        )
    }
}

pub struct Renderers {
    base: SizeKey,
    map: HashMap<SizeKey, Renderer>,
}

impl Renderers {
    pub fn new(spec: &RendererSpec, base_size: f32) -> Self {
        let base = SizeKey::of(base_size);
        Self {
            base,
            map: HashMap::from([(base, spec.build(base))]),
        }
    }

    pub fn base_key(&self) -> SizeKey {
        self.base
    }

    /// Les métriques de cellule d'une taille ; celles de la base si elle
    /// n'est pas (encore) chargée.
    pub fn metrics(&self, key: SizeKey) -> CellMetrics {
        self.map
            .get(&key)
            .or_else(|| self.map.get(&self.base))
            .map(Renderer::metrics)
            .expect("le renderer de base existe toujours")
    }

    pub fn get_mut(&mut self, key: SizeKey) -> Option<&mut Renderer> {
        self.map.get_mut(&key)
    }

    /// Crée les renderers des tailles `keys` qui manquent et libère ceux
    /// qui ne servent plus (jamais celui de la base).
    pub fn sync(&mut self, spec: &RendererSpec, keys: &BTreeSet<SizeKey>) {
        let base = self.base;
        self.map.retain(|k, _| *k == base || keys.contains(k));
        for key in keys {
            self.map.entry(*key).or_insert_with(|| spec.build(*key));
        }
    }

    pub fn set_palette(&mut self, palette: &Palette) {
        for r in self.map.values_mut() {
            r.set_palette(palette.clone());
        }
    }
}

fn load_fonts(family: &str, size_pt: f32, scale_factor: f64) -> FontSet {
    let px = size_pt * PT_TO_PX * scale_factor as f32;
    FontSet::load(family, px).unwrap_or_else(|e| {
        tracing::warn!("police « {family} » : {e} — police embarquée");
        FontSet::embedded(px)
    })
}
