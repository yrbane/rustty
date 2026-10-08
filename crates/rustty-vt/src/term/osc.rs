//! Operating System Commands : titre de fenêtre et presse-papiers.

use base64::Engine as _;

use super::Term;
use crate::outbox::TermEvent;

impl Term {
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
    use crate::outbox::TermEvent;
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
}
