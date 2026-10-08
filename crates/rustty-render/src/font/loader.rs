//! Chargement des polices : la famille demandée ou un repli à chasse fixe,
//! en quatre variantes, plus la police embarquée comme dernier recours.

use std::collections::HashMap;
use std::sync::Arc;

use fontdb::{Database, Family, Query, Stretch, Style, Weight};

use super::{CellMetrics, FontError, Variant};

/// DejaVu Sans Mono, licence Bitstream Vera : rendu identique sur tous les OS
/// et dernier recours si le système n'a aucune police à chasse fixe.
pub const EMBEDDED_FONT: &[u8] = include_bytes!("../../assets/DejaVuSansMono.ttf");

/// Familles essayées quand celle de la configuration manque, du plus courant
/// au plus ancien, sur les trois OS.
const FALLBACK_FAMILIES: &[&str] = &[
    "DejaVu Sans Mono",
    "Liberation Mono",
    "Menlo",
    "Consolas",
    "Noto Sans Mono",
    "Courier New",
];

#[derive(Clone)]
pub struct FaceData {
    pub data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    pub index: u32,
}

impl FaceData {
    pub fn font_ref(&self) -> Option<swash::FontRef<'_>> {
        swash::FontRef::from_index((*self.data).as_ref(), self.index as usize)
    }
}

pub struct FontSet {
    db: Database,
    faces: [FaceData; 4],
    /// Faces de repli chargées à la demande (slots ≥ 4).
    fallbacks: Vec<FaceData>,
    /// Slot mémorisé par caractère hors des quatre variantes.
    coverage: HashMap<char, Option<usize>>,
    family_name: String,
    size_px: f32,
    metrics: CellMetrics,
}

impl FontSet {
    /// Polices système ; `family` d'abord, puis les replis.
    pub fn load(family: &str, size_px: f32) -> Result<Self, FontError> {
        let mut db = Database::new();
        db.load_system_fonts();
        let candidates = std::iter::once(family).chain(FALLBACK_FAMILIES.iter().copied());
        for name in candidates {
            if let Some(regular) = query(&db, name, Variant::Regular) {
                return Self::build(db, name, regular, size_px);
            }
        }
        let any_monospace = db.faces().find(|f| f.monospaced).map(|f| {
            (
                f.families
                    .first()
                    .map(|(n, _)| n.clone())
                    .unwrap_or_default(),
                f.id,
            )
        });
        if let Some((name, id)) = any_monospace {
            return Self::build(db, &name, id, size_px);
        }
        Ok(Self::embedded(size_px))
    }

    /// Les quatre variantes pointent sur la police embarquée.
    pub fn embedded(size_px: f32) -> Self {
        Self::from_bytes(EMBEDDED_FONT.to_vec(), size_px).expect("la police embarquée est valide")
    }

    pub fn from_bytes(data: Vec<u8>, size_px: f32) -> Result<Self, FontError> {
        let face = FaceData {
            data: Arc::new(data),
            index: 0,
        };
        let font = face
            .font_ref()
            .ok_or_else(|| FontError::Invalid("format non reconnu".into()))?;
        let metrics = CellMetrics::from_font(&font, size_px);
        let family_name = font
            .localized_strings()
            .find_by_id(swash::StringId::Family, None)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "embedded".into());
        Ok(Self {
            db: Database::new(),
            faces: [face.clone(), face.clone(), face.clone(), face],
            fallbacks: Vec::new(),
            coverage: HashMap::new(),
            family_name,
            size_px,
            metrics,
        })
    }

    fn build(
        db: Database,
        family: &str,
        regular: fontdb::ID,
        size_px: f32,
    ) -> Result<Self, FontError> {
        let regular_face = load_face(&db, regular)?;
        let variant_face = |v: Variant| {
            query(&db, family, v)
                .and_then(|id| load_face(&db, id).ok())
                .unwrap_or_else(|| regular_face.clone())
        };
        let faces = [
            regular_face.clone(),
            variant_face(Variant::Bold),
            variant_face(Variant::Italic),
            variant_face(Variant::BoldItalic),
        ];
        let font = regular_face
            .font_ref()
            .ok_or_else(|| FontError::Invalid(family.into()))?;
        let metrics = CellMetrics::from_font(&font, size_px);
        Ok(Self {
            db,
            faces,
            fallbacks: Vec::new(),
            coverage: HashMap::new(),
            family_name: family.to_string(),
            size_px,
            metrics,
        })
    }

    pub fn size_px(&self) -> f32 {
        self.size_px
    }

    pub fn metrics(&self) -> CellMetrics {
        self.metrics
    }

    pub fn face(&self, variant: Variant) -> &FaceData {
        &self.faces[variant.index()]
    }

    pub fn family_name(&self) -> &str {
        &self.family_name
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphRef {
    /// 0–3 : variante demandée ou voisine ; ≥ 4 : face de repli.
    pub slot: usize,
    pub glyph_id: u16,
}

impl FontSet {
    pub fn face_by_slot(&self, slot: usize) -> &FaceData {
        if slot < 4 {
            &self.faces[slot]
        } else {
            &self.fallbacks[slot - 4]
        }
    }

    /// Le glyphe de `ch` : la variante demandée, les autres variantes, puis une
    /// police du système qui couvre `ch`, puis la police embarquée.
    pub fn glyph(&mut self, ch: char, variant: Variant) -> Option<GlyphRef> {
        let order = [variant.index(), 0, 1, 2, 3];
        for slot in order {
            if let Some(gid) = glyph_in(&self.faces[slot], ch) {
                return Some(GlyphRef {
                    slot,
                    glyph_id: gid,
                });
            }
        }
        if let Some(cached) = self.coverage.get(&ch) {
            return cached.map(|slot| GlyphRef {
                slot,
                glyph_id: glyph_in(self.face_by_slot(slot), ch).unwrap_or(0),
            });
        }
        let found = self.find_fallback(ch);
        self.coverage.insert(ch, found.map(|g| g.slot));
        found
    }

    fn find_fallback(&mut self, ch: char) -> Option<GlyphRef> {
        for (i, face) in self.fallbacks.iter().enumerate() {
            if let Some(gid) = glyph_in(face, ch) {
                return Some(GlyphRef {
                    slot: 4 + i,
                    glyph_id: gid,
                });
            }
        }
        let ids: Vec<fontdb::ID> = self.db.faces().map(|f| f.id).collect();
        for id in ids {
            let Ok(face) = load_face(&self.db, id) else {
                continue;
            };
            if let Some(gid) = glyph_in(&face, ch) {
                self.fallbacks.push(face);
                return Some(GlyphRef {
                    slot: 4 + self.fallbacks.len() - 1,
                    glyph_id: gid,
                });
            }
        }
        let embedded = FaceData {
            data: Arc::new(EMBEDDED_FONT.to_vec()),
            index: 0,
        };
        let gid = glyph_in(&embedded, ch)?;
        self.fallbacks.push(embedded);
        Some(GlyphRef {
            slot: 4 + self.fallbacks.len() - 1,
            glyph_id: gid,
        })
    }
}

/// Identifiant du glyphe de `ch` dans `face`, `None` si la face ne le couvre pas.
fn glyph_in(face: &FaceData, ch: char) -> Option<u16> {
    let font = face.font_ref()?;
    match font.charmap().map(ch) {
        0 => None,
        gid => Some(gid),
    }
}

fn query(db: &Database, family: &str, variant: Variant) -> Option<fontdb::ID> {
    let (weight, style) = match variant {
        Variant::Regular => (Weight::NORMAL, Style::Normal),
        Variant::Bold => (Weight::BOLD, Style::Normal),
        Variant::Italic => (Weight::NORMAL, Style::Italic),
        Variant::BoldItalic => (Weight::BOLD, Style::Italic),
    };
    let families = [Family::Name(family)];
    db.query(&Query {
        families: &families,
        weight,
        stretch: Stretch::Normal,
        style,
    })
}

pub(crate) fn load_face(db: &Database, id: fontdb::ID) -> Result<FaceData, FontError> {
    // Copie des octets : évite le mapping mémoire (unsafe) de fontdb ; une
    // police pèse quelques centaines de Kio, chargée une fois.
    let (data, index) = db
        .with_face_data(id, |bytes, index| (bytes.to_vec(), index))
        .ok_or_else(|| FontError::Invalid(format!("{id:?}")))?;
    Ok(FaceData {
        data: Arc::new(data),
        index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_font_loads_all_variants() {
        let set = FontSet::embedded(14.0);
        for v in Variant::ALL {
            assert!(set.face(v).font_ref().is_some(), "{v:?}");
        }
        assert_eq!(set.size_px(), 14.0);
        assert!(set.metrics().width > 0);
        assert_eq!(set.family_name(), "DejaVu Sans Mono");
    }

    #[test]
    fn from_bytes_rejects_garbage() {
        assert!(matches!(
            FontSet::from_bytes(vec![0u8; 100], 12.0),
            Err(FontError::Invalid(_))
        ));
    }

    #[test]
    fn unknown_family_falls_back() {
        let set = FontSet::load("Police-Inexistante-Rustty-42", 12.0).unwrap();
        assert!(set.metrics().width > 0 && set.metrics().height > 0);
        assert!(set.face(Variant::Bold).font_ref().is_some());
    }

    #[test]
    fn cell_metrics_are_positive_for_the_system_monospace() {
        let set = FontSet::load("monospace", 11.0).unwrap();
        let m = set.metrics();
        assert!(
            m.width > 0 && m.height > m.baseline && m.baseline > 0,
            "{m:?}"
        );
    }

    #[test]
    fn variant_from_attrs() {
        use rustty_vt::Attrs;
        assert_eq!(Variant::from_attrs(Attrs::empty()), Variant::Regular);
        assert_eq!(Variant::from_attrs(Attrs::BOLD), Variant::Bold);
        assert_eq!(Variant::from_attrs(Attrs::ITALIC), Variant::Italic);
        assert_eq!(
            Variant::from_attrs(Attrs::BOLD | Attrs::ITALIC | Attrs::UNDERLINE),
            Variant::BoldItalic
        );
        assert_eq!(Variant::BoldItalic.index(), 3);
    }
}
