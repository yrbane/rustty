//! Ce que le terminal produit vers l'extérieur : événements pour l'interface
//! et octets de réponse à renvoyer à l'application.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermEvent {
    Title(String),
    Bell,
    /// OSC 52 : l'application demande à écrire dans le presse-papiers.
    SetClipboard(String),
}

#[derive(Debug, Default)]
pub struct Outbox {
    events: Vec<TermEvent>,
    responses: Vec<u8>,
}

impl Outbox {
    pub fn event(&mut self, ev: TermEvent) {
        self.events.push(ev);
    }

    pub fn respond(&mut self, bytes: &[u8]) {
        self.responses.extend_from_slice(bytes);
    }

    pub fn drain_events(&mut self) -> Vec<TermEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn drain_responses(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.responses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_and_responses_are_drained_in_order_and_once() {
        let mut o = Outbox::default();
        o.event(TermEvent::Bell);
        o.event(TermEvent::Title("t".into()));
        o.respond(b"\x1b[0n");
        o.respond(b"x");
        assert_eq!(
            o.drain_events(),
            vec![TermEvent::Bell, TermEvent::Title("t".into())]
        );
        assert!(o.drain_events().is_empty());
        assert_eq!(o.drain_responses(), b"\x1b[0nx".to_vec());
        assert!(o.drain_responses().is_empty());
    }
}
