//! Images transmises avec un `i=`, conservées pour des `a=p` ultérieurs.

use std::sync::Arc;

use super::ImageData;

/// Quota de pixels RGBA mémorisés (256 Mio).
pub const QUOTA: usize = 256 * 1024 * 1024;

/// Magasin d'images borné : la plus ancienne insertion est oubliée d'abord.
pub struct ImageStore {
    quota: usize,
    entries: Vec<(u32, Arc<ImageData>)>,
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::with_quota(QUOTA)
    }
}

impl ImageStore {
    pub fn with_quota(quota: usize) -> Self {
        Self {
            quota,
            entries: Vec::new(),
        }
    }

    /// Mémorise l'image ; un `id` déjà connu est remplacé.
    pub fn insert(&mut self, id: u32, image: Arc<ImageData>) {
        self.remove(id);
        self.entries.push((id, image));
        let mut total = self.total();
        // La dernière insertion est toujours conservée, même seule au-dessus du quota.
        while total > self.quota && self.entries.len() > 1 {
            let (_, oldest) = self.entries.remove(0);
            total -= oldest.rgba.len();
        }
    }

    pub fn get(&self, id: u32) -> Option<Arc<ImageData>> {
        self.entries
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, img)| Arc::clone(img))
    }

    pub fn remove(&mut self, id: u32) {
        self.entries.retain(|(i, _)| *i != id);
    }

    fn total(&self) -> usize {
        self.entries.iter().map(|(_, i)| i.rgba.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(bytes_side: u32) -> Arc<ImageData> {
        let n = bytes_side as usize;
        Arc::new(ImageData::new(bytes_side, 1, vec![0; n * 4]))
    }

    #[test]
    fn store_returns_what_was_inserted() {
        let mut s = ImageStore::default();
        s.insert(1, img(2));
        assert_eq!(s.get(1).unwrap().width, 2);
        assert!(s.get(2).is_none());
        s.remove(1);
        assert!(s.get(1).is_none());
    }

    #[test]
    fn reinserting_an_id_replaces_it() {
        let mut s = ImageStore::default();
        s.insert(1, img(2));
        s.insert(1, img(3));
        assert_eq!(s.get(1).unwrap().width, 3);
        assert_eq!(s.total(), 12);
    }

    #[test]
    fn store_forgets_oldest_over_quota() {
        let mut s = ImageStore::with_quota(40);
        s.insert(1, img(4)); // 16 octets
        s.insert(2, img(4));
        s.insert(3, img(4)); // 48 > 40 : l'image 1 part
        assert!(s.get(1).is_none());
        assert!(s.get(2).is_some() && s.get(3).is_some());
    }
}
