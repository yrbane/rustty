//! Placement des bitmaps de glyphes dans une texture carrée, par étagères :
//! simple, rapide, et suffisant pour des glyphes de hauteurs proches.

use crate::font::CellMetrics;

pub const DEFAULT_ATLAS_SIZE: u32 = 2048;
const MIN_ATLAS_SIZE: u32 = 512;

/// Côté de l'atlas pour une taille de cellule : de quoi loger ~16 glyphes de
/// large (deux fois la plus grande dimension, pour les glyphes larges),
/// en puissance de deux, borné. Évite 16 Mio de VRAM par taille de police.
pub fn atlas_size_for(metrics: CellMetrics) -> u32 {
    let wanted = 16 * metrics.width.max(metrics.height) * 2;
    wanted
        .next_power_of_two()
        .clamp(MIN_ATLAS_SIZE, DEFAULT_ATLAS_SIZE)
}

/// Marge entre deux régions, pour que l'échantillonnage ne bave pas.
const PADDING: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl AtlasRegion {
    /// Coordonnées de texture `[u0, v0, u1, v1]`.
    pub fn uv(&self, atlas_size: u32) -> [f32; 4] {
        let s = atlas_size as f32;
        [
            self.x as f32 / s,
            self.y as f32 / s,
            (self.x + self.width) as f32 / s,
            (self.y + self.height) as f32 / s,
        ]
    }
}

#[derive(Clone, Debug)]
struct Shelf {
    y: u32,
    height: u32,
    next_x: u32,
}

#[derive(Clone, Debug)]
pub struct AtlasPacker {
    size: u32,
    shelves: Vec<Shelf>,
    next_y: u32,
    count: usize,
}

impl AtlasPacker {
    pub fn new(size: u32) -> Self {
        Self {
            size,
            shelves: Vec::new(),
            next_y: 0,
            count: 0,
        }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn clear(&mut self) {
        self.shelves.clear();
        self.next_y = 0;
        self.count = 0;
    }

    /// Place une région de `width × height`. Une étagère existante est
    /// réutilisée si le glyphe y tient sans gaspiller plus de la moitié de sa
    /// hauteur ; sinon une nouvelle étagère est ouverte.
    pub fn insert(&mut self, width: u32, height: u32) -> Option<AtlasRegion> {
        if width == 0 || height == 0 || width > self.size || height > self.size {
            return None;
        }
        let padded_w = width + PADDING;
        if let Some(shelf) = self.shelves.iter_mut().find(|s| {
            height <= s.height
                && height * 2 >= s.height
                && s.next_x + padded_w <= self.size + PADDING
        }) {
            let region = AtlasRegion {
                x: shelf.next_x,
                y: shelf.y,
                width,
                height,
            };
            shelf.next_x += padded_w;
            self.count += 1;
            return Some(region);
        }
        let shelf_height = height + PADDING;
        if self.next_y + height > self.size {
            return None;
        }
        let region = AtlasRegion {
            x: 0,
            y: self.next_y,
            width,
            height,
        };
        self.shelves.push(Shelf {
            y: self.next_y,
            height,
            next_x: padded_w,
        });
        self.next_y += shelf_height;
        self.count += 1;
        Some(region)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(width: u32, height: u32) -> CellMetrics {
        CellMetrics {
            width,
            height,
            baseline: 0,
            underline_y: 0,
            underline_thickness: 1,
            strike_y: 0,
        }
    }

    #[test]
    fn atlas_is_small_for_small_fonts() {
        assert_eq!(atlas_size_for(cell(8, 16)), 512);
    }

    #[test]
    fn atlas_is_capped() {
        assert_eq!(atlas_size_for(cell(80, 160)), 2048);
    }

    #[test]
    fn regions_do_not_overlap_and_stay_inside() {
        let mut p = AtlasPacker::new(64);
        let mut regions = Vec::new();
        for (w, h) in [(10, 12), (20, 12), (30, 8), (5, 30), (40, 10), (10, 10)] {
            let r = p.insert(w, h).unwrap();
            assert_eq!((r.width, r.height), (w, h));
            assert!(r.x + r.width <= 64 && r.y + r.height <= 64, "{r:?}");
            regions.push(r);
        }
        for (i, a) in regions.iter().enumerate() {
            for b in &regions[i + 1..] {
                let overlap = a.x < b.x + b.width
                    && b.x < a.x + a.width
                    && a.y < b.y + b.height
                    && b.y < a.y + a.height;
                assert!(!overlap, "{a:?} chevauche {b:?}");
            }
        }
        assert_eq!(p.len(), 6);
    }

    #[test]
    fn same_height_glyphs_share_a_shelf() {
        let mut p = AtlasPacker::new(100);
        let a = p.insert(10, 10).unwrap();
        let b = p.insert(10, 10).unwrap();
        assert_eq!(a.y, b.y, "même étagère");
        assert_eq!(b.x, a.x + a.width + 1, "une marge d'un pixel");
    }

    #[test]
    fn full_atlas_returns_none_until_cleared() {
        let mut p = AtlasPacker::new(16);
        assert!(p.insert(15, 15).is_some());
        assert!(p.insert(2, 2).is_none(), "plus de place");
        p.clear();
        assert_eq!(p.len(), 0);
        assert!(p.insert(15, 15).is_some());
    }

    #[test]
    fn oversized_or_empty_requests_are_rejected() {
        let mut p = AtlasPacker::new(32);
        assert!(p.insert(40, 4).is_none());
        assert!(p.insert(4, 40).is_none());
        assert!(p.insert(0, 4).is_none());
        assert!(p.insert(4, 0).is_none());
        assert_eq!(p.len(), 0);
    }

    #[test]
    fn uv_maps_pixels_to_unit_square() {
        let r = AtlasRegion {
            x: 16,
            y: 32,
            width: 16,
            height: 32,
        };
        assert_eq!(r.uv(64), [0.25, 0.5, 0.5, 1.0]);
    }
}
