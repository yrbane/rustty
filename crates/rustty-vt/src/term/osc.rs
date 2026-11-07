//! Operating System Commands : titre de fenêtre et presse-papiers.

use base64::Engine as _;

use std::sync::Arc;

use super::Term;
use crate::outbox::TermEvent;

impl Term {
    /// OSC 0/2 : vte découpe sur `;`, on recolle tout ce qui suit le code.
    pub(crate) fn set_title_from_params(&mut self, params: &[&[u8]]) {
        let title = params
            .iter()
            .map(|p| String::from_utf8_lossy(p))
            .collect::<Vec<_>>()
            .join(";");
        self.set_title(title);
    }

    /// `OSC 8 ; params ; URI` : ouvre un lien pour les cellules à venir, ou
    /// le ferme (URI vide). Une URI trop longue (> 2083 octets), non UTF-8
    /// ou contenant un caractère de contrôle vaut fermeture : jamais de lien
    /// tronqué. Seul le paramètre `id=` est lu.
    pub(crate) fn set_hyperlink_from_params(&mut self, params: &[&[u8]]) {
        const MAX_URI: usize = 2083;
        let uri = params.get(2..).unwrap_or(&[]).join(&b';');
        let valid = !uri.is_empty()
            && uri.len() <= MAX_URI
            && !uri.iter().any(|b| *b < 0x20 || *b == 0x7f)
            && std::str::from_utf8(&uri).is_ok();
        if !valid {
            self.current_link = None;
            return;
        }
        let uri = String::from_utf8_lossy(&uri).into_owned();
        let id_param = params
            .get(1)
            .and_then(|p| {
                p.split(|b| *b == b':')
                    .find_map(|kv| kv.strip_prefix(b"id="))
            })
            .map(|v| String::from_utf8_lossy(v).into_owned());
        let id_param = id_param.as_deref();
        if let Some(id) = self.links.find(&uri, id_param) {
            // Connu : pas de copie de la table partagée avec un instantané.
            self.current_link = Some(id);
            return;
        }
        let mut id = Arc::make_mut(&mut self.links).insert(&uri, id_param);
        if id.is_none() && self.link_refusals % Self::LINK_RESCAN_EVERY == 0 {
            self.compact_links();
            id = Arc::make_mut(&mut self.links).insert(&uri, id_param);
        }
        if id.is_none() {
            self.link_refusals += 1;
        } else {
            self.link_refusals = 0;
        }
        self.current_link = id;
    }

    pub(crate) fn set_title(&mut self, title: String) {
        self.title.clone_from(&title);
        self.outbox.event(TermEvent::Title(title));
    }

    /// OSC 52 : `data` est le troisième paramètre. `?` (lecture) est refusé,
    /// un base64 invalide est ignoré.
    pub(crate) fn set_clipboard_from_base64(&mut self, data: &[u8]) {
        if data == b"?" {
            return;
        }
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) else {
            return;
        };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        self.outbox.event(TermEvent::SetClipboard(text));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::links::LinkTable;
    use crate::outbox::TermEvent;
    use crate::term::Term;
    use crate::term::test_support::{feed, term};

    #[test]
    fn title_with_bel_and_st_terminators() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]0;Hello\x07");
        assert_eq!(t.title(), "Hello");
        feed(&mut t, "\x1b]2;World\x1b\\");
        assert_eq!(t.title(), "World");
        assert_eq!(
            t.drain_events(),
            vec![
                TermEvent::Title("Hello".into()),
                TermEvent::Title("World".into())
            ]
        );
    }

    #[test]
    fn osc52_sets_clipboard_from_base64() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]52;c;aGVsbG8=\x07");
        assert_eq!(
            t.drain_events(),
            vec![TermEvent::SetClipboard("hello".into())]
        );
    }

    #[test]
    fn osc52_invalid_base64_is_ignored() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]52;c;!!!not-base64!!!\x07ok");
        assert!(t.drain_events().is_empty());
        assert_eq!(t.text(), vec!["ok"], "le flux continue d'être interprété");
    }

    #[test]
    fn osc52_query_is_not_answered() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]52;c;?\x07");
        assert!(t.drain_events().is_empty());
        assert!(
            t.drain_responses().is_empty(),
            "ne jamais divulguer le presse-papiers"
        );
    }

    #[test]
    fn title_keeps_its_semicolons() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b]2;vim; a;b\x07");
        assert_eq!(t.title(), "vim; a;b");
    }

    #[test]
    fn osc8_links_printed_cells_until_closed() {
        let mut t = term(20, 4);
        t.input(b"\x1b]8;;https://exemple.org\x1b\\ab\x1b]8;;\x1b\\c");
        let id = t.grid().cell(0, 0).link.expect("lien sur a");
        assert_eq!(t.grid().cell(1, 0).link, Some(id));
        assert_eq!(t.grid().cell(2, 0).link, None);
        assert_eq!(t.links().uri(id), Some("https://exemple.org"));
        assert_eq!(t.snapshot().link_uri(id), Some("https://exemple.org"));
    }

    #[test]
    fn sgr_reset_does_not_close_the_link_but_the_id_param_distinguishes() {
        let mut t = term(20, 4);
        t.input(b"\x1b]8;id=1;https://x\x1b\\a\x1b[0mb\x1b]8;id=2;https://x\x1b\\c");
        let a = t.grid().cell(0, 0).link.unwrap();
        assert_eq!(t.grid().cell(1, 0).link, Some(a));
        assert_ne!(t.grid().cell(2, 0).link, Some(a));
        assert!(t.grid().cell(2, 0).link.is_some());
    }

    #[test]
    fn a_uri_with_semicolons_is_kept_whole() {
        let mut t = term(20, 4);
        t.input(b"\x1b]8;;https://x/?a=1;b=2\x1b\\a");
        let id = t.grid().cell(0, 0).link.unwrap();
        assert_eq!(t.links().uri(id), Some("https://x/?a=1;b=2"));
    }

    #[test]
    fn an_oversized_uri_closes_the_link() {
        let mut t = term(20, 4);
        let long = format!("\x1b]8;;https://{}\x1b\\", "a".repeat(2100));
        t.input(b"\x1b]8;;https://ok\x1b\\a");
        t.input(long.as_bytes());
        t.input(b"b");
        assert!(t.grid().cell(0, 0).link.is_some());
        assert_eq!(t.grid().cell(1, 0).link, None);
        assert_eq!(t.links().len(), 1);
    }

    #[test]
    fn invalid_utf8_closes_the_link() {
        let mut t = term(20, 4);
        t.input(b"\x1b]8;;https://ok\x1b\\a");
        t.input(b"\x1b]8;;https://\xff\x1b\\b");
        assert_eq!(t.grid().cell(1, 0).link, None);
        assert_eq!(t.links().len(), 1);
    }

    #[test]
    fn control_bytes_close_the_link() {
        // Le parseur vte retire déjà les C0 d'une OSC ; la garde reste pour
        // tout autre chemin, exercée directement.
        for bad in [&b"https://x\x01y"[..], b"https://x\x7fy"] {
            let mut t = term(20, 4);
            t.input(b"\x1b]8;;https://ok\x1b\\a");
            t.set_hyperlink_from_params(&[b"8", b"", bad]);
            t.input(b"b");
            assert_eq!(t.grid().cell(1, 0).link, None);
            assert_eq!(t.links().len(), 1);
        }
    }

    #[test]
    fn ris_clears_the_table_and_the_current_link() {
        let mut t = term(20, 4);
        t.input(b"\x1b]8;;https://x\x1b\\a\x1bcb");
        assert!(t.links().is_empty());
        assert_eq!(t.current_link(), None);
        assert_eq!(t.grid().cell(0, 0).link, None);
    }

    #[test]
    fn ris_keeps_the_links_of_the_history() {
        // L'historique survit à RIS : ses liens doivent encore désigner la
        // bonne URI après qu'un nouveau lien a pris l'identifiant 1.
        let mut t = Term::new(4, 1, 10);
        t.input(b"\x1b]8;;https://old\x1b\\abcd\x1b]8;;\x1b\\\r\n");
        t.input(b"\x1bc\x1b]8;;https://new\x1b\\x");
        let snap = t.snapshot();
        let old = t.scrollback().get(0).unwrap().cells()[0].link.unwrap();
        assert_eq!(snap.link_uri(old), Some("https://old"));
        assert_eq!(t.links().len(), 2);
    }

    #[test]
    fn a_wide_character_carries_its_link_on_both_cells() {
        let mut t = term(20, 4);
        t.input("\x1b]8;;https://x\x1b\\\u{1f517}a".as_bytes());
        let id = t.grid().cell(0, 0).link;
        assert!(id.is_some());
        assert_eq!(t.grid().cell(1, 0).link, id, "moitié droite");
        assert_eq!(t.grid().cell(2, 0).link, id);
    }

    #[test]
    fn a_known_link_does_not_copy_the_table_shared_with_a_snapshot() {
        let mut t = term(20, 4);
        t.input(b"\x1b]8;;https://x\x1b\\a\x1b]8;;\x1b\\");
        let snap = t.snapshot();
        t.input(b"\x1b]8;;https://x\x1b\\b");
        assert!(
            Arc::ptr_eq(&snap.links, t.links()),
            "même lien : pas de copie"
        );
        t.input(b"\x1b]8;;https://y\x1b\\c");
        assert!(!Arc::ptr_eq(&snap.links, t.links()), "nouveau lien : copie");
    }

    #[test]
    fn a_saturated_table_is_not_rescanned_for_every_refused_link() {
        // Tous les liens vivants : le compactage ne libère rien, un nouveau
        // lien est refusé, et les refus suivants ne rebalaient pas tout.
        let mut t = Term::new(4, 2, 70_000);
        for i in 0..LinkTable::MAX_LINKS {
            t.input(format!("\x1b]8;;https://{i}\x1b\\a\r\n").as_bytes());
        }
        assert_eq!(t.links().len(), LinkTable::MAX_LINKS);
        t.input(b"\x1b]8;;https://full\x1b\\x");
        assert_eq!(t.current_link(), None);
        let before = t.link_compactions();
        for i in 0..100 {
            t.input(format!("\x1b]8;;https://more{i}\x1b\\x").as_bytes());
        }
        assert_eq!(t.link_compactions(), before, "pas de balayage par refus");
    }

    #[test]
    fn erased_cells_carry_no_link() {
        let mut t = term(20, 4);
        t.input(b"\x1b]8;;https://x\x1b\\abc\x1b[2D\x1b[K");
        assert_eq!(t.grid().cell(1, 0).link, None);
        assert!(t.grid().cell(0, 0).link.is_some());
    }

    #[test]
    fn alt_screen_shares_the_table_and_compaction_keeps_visible_links() {
        let mut t = Term::new(20, 2, 2);
        t.input(b"\x1b]8;;https://main\x1b\\m\x1b]8;;\x1b\\");
        t.input(b"\x1b[?1049h\x1b]8;;https://alt\x1b\\x\x1b]8;;\x1b\\\x1b[?1049l");
        assert_eq!(t.links().len(), 2, "une table pour les deux écrans");
        t.compact_links();
        let id = t.grid().cell(0, 0).link.unwrap();
        assert_eq!(t.links().uri(id), Some("https://main"));
        assert_eq!(t.links().len(), 1);
    }

    #[test]
    fn a_full_table_is_compacted_before_refusing() {
        let mut t = Term::new(4, 2, 0);
        for i in 0..LinkTable::MAX_LINKS {
            t.input(format!("\x1b]8;;u{i}\x1b\\\r").as_bytes());
        }
        t.input(b"\x1b]8;;https://new\x1b\\z");
        let id = t
            .grid()
            .cell(0, 0)
            .link
            .expect("après compactage, place pour le nouveau lien");
        assert_eq!(t.links().uri(id), Some("https://new"));
        assert_eq!(t.links().len(), 1);
    }

    #[test]
    fn ed3_compacts_links_of_the_cleared_history() {
        let mut t = Term::new(4, 1, 10);
        t.input(b"\x1b]8;;https://old\x1b\\o\x1b]8;;\x1b\\\r\nn");
        assert_eq!(t.links().len(), 1);
        t.input(b"\x1b[3J");
        assert!(t.links().is_empty());
    }
}
