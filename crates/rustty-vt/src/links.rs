//! Table des hyperliens OSC 8 d'un terminal : chaque cellule porte un
//! `LinkId`, la table donne l'URI et le paramètre `id`.

use std::collections::HashMap;
use std::num::NonZeroU16;

/// Identifiant d'un lien dans la table ; `NonZeroU16` pour que
/// `Option<LinkId>` tienne dans deux octets de la cellule.
pub type LinkId = NonZeroU16;

/// Un hyperlien tel que déclaré par `OSC 8 ; id=… ; URI`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub uri: String,
    /// Paramètre `id=` de la séquence, qui distingue deux liens de même URI.
    pub id_param: Option<String>,
}

/// Liens d'un terminal, dédupliqués par `(uri, id)`, bornés par `MAX_LINKS`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkTable {
    links: Vec<Link>,
    index: HashMap<(String, Option<String>), LinkId>,
}

impl LinkTable {
    /// Capacité de `LinkId` : au-delà, `insert` refuse les liens nouveaux.
    pub const MAX_LINKS: usize = u16::MAX as usize;

    /// L'identifiant du lien `(uri, id_param)`, créé s'il est nouveau ;
    /// `None` si la table est pleine et le lien inconnu.
    /// L'identifiant d'un lien déjà connu, sans rien modifier.
    pub fn find(&self, uri: &str, id_param: Option<&str>) -> Option<LinkId> {
        let key = (uri.to_string(), id_param.map(str::to_string));
        self.index.get(&key).copied()
    }

    pub fn insert(&mut self, uri: &str, id_param: Option<&str>) -> Option<LinkId> {
        if let Some(id) = self.find(uri, id_param) {
            return Some(id);
        }
        let key = (uri.to_string(), id_param.map(str::to_string));
        if self.links.len() >= Self::MAX_LINKS {
            return None;
        }
        self.links.push(Link {
            uri: key.0.clone(),
            id_param: key.1.clone(),
        });
        let id = LinkId::new(self.links.len() as u16)?;
        self.index.insert(key, id);
        Some(id)
    }

    pub fn get(&self, id: LinkId) -> Option<&Link> {
        self.links.get(usize::from(id.get()) - 1)
    }

    pub fn uri(&self, id: LinkId) -> Option<&str> {
        self.get(id).map(|l| l.uri.as_str())
    }

    pub fn len(&self) -> usize {
        self.links.len()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// Une table réduite aux liens `used`, renumérotés, et la correspondance
    /// ancien identifiant (indexée par `id.get()`, entrée 0 inutilisée) →
    /// nouvel identifiant.
    pub fn retain_remapped(
        &self,
        used: impl Fn(LinkId) -> bool,
    ) -> (LinkTable, Vec<Option<LinkId>>) {
        let mut kept = LinkTable::default();
        let mut map = vec![None; self.links.len() + 1];
        for (i, link) in self.links.iter().enumerate() {
            let Some(old) = LinkId::new(i as u16 + 1) else {
                continue;
            };
            if used(old) {
                map[usize::from(old.get())] = kept.insert(&link.uri, link.id_param.as_deref());
            }
        }
        (kept, map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_uri_and_id_share_one_link() {
        let mut t = LinkTable::default();
        let a = t.insert("https://a.example", None).unwrap();
        let b = t.insert("https://a.example", None).unwrap();
        let c = t.insert("https://a.example", Some("x")).unwrap();
        let d = t.insert("https://b.example", None).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_eq!(t.uri(c), Some("https://a.example"));
        assert_eq!(t.get(c).unwrap().id_param.as_deref(), Some("x"));
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn the_table_is_bounded() {
        let mut t = LinkTable::default();
        for i in 0..LinkTable::MAX_LINKS {
            assert!(t.insert(&format!("u{i}"), None).is_some());
        }
        assert_eq!(t.insert("one-more", None), None, "pleine");
        assert!(
            t.insert("u0", None).is_some(),
            "un lien connu reste utilisable"
        );
    }

    #[test]
    fn retain_remapped_keeps_only_used_links_and_renumbers() {
        let mut t = LinkTable::default();
        let a = t.insert("a", None).unwrap();
        let b = t.insert("b", None).unwrap();
        let c = t.insert("c", None).unwrap();
        let (mut kept, map) = t.retain_remapped(|id| id == b || id == c);
        assert_eq!(kept.len(), 2);
        assert_eq!(map[a.get() as usize], None);
        assert_eq!(kept.uri(map[b.get() as usize].unwrap()), Some("b"));
        assert_eq!(kept.uri(map[c.get() as usize].unwrap()), Some("c"));
        assert_eq!(
            kept.insert("b", None),
            map[b.get() as usize],
            "la déduplication survit"
        );
    }
}
