//! Réponses aux requêtes de l'application : identification et état.

use super::Term;

impl Term {
    /// DA1 : VT220 avec couleurs ANSI (22).
    pub(crate) fn device_attributes(&mut self) {
        self.outbox.respond(b"\x1b[?62;22c");
    }

    pub(crate) fn secondary_device_attributes(&mut self) {
        self.outbox.respond(b"\x1b[>1;10;0c");
    }

    pub(crate) fn device_status_report(&mut self, mode: u16, private: bool) {
        match mode {
            5 => self.outbox.respond(b"\x1b[0n"),
            6 => {
                let row = if self.modes.origin {
                    self.cursor.row.saturating_sub(self.region.top)
                } else {
                    self.cursor.row
                };
                let prefix = if private { "?" } else { "" };
                let reply = format!("\x1b[{prefix}{};{}R", row + 1, self.cursor.col + 1);
                self.outbox.respond(reply.as_bytes());
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::term::test_support::{feed, term};

    #[test]
    fn primary_and_secondary_device_attributes() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b[c");
        assert_eq!(t.drain_responses(), b"\x1b[?62;22c".to_vec());
        feed(&mut t, "\x1b[>c");
        assert_eq!(t.drain_responses(), b"\x1b[>1;10;0c".to_vec());
    }

    #[test]
    fn device_status_reports() {
        let mut t = term(10, 5);
        feed(&mut t, "\x1b[5n");
        assert_eq!(t.drain_responses(), b"\x1b[0n".to_vec());
        feed(&mut t, "\x1b[3;4H\x1b[6n");
        assert_eq!(t.drain_responses(), b"\x1b[3;4R".to_vec());
        feed(&mut t, "\x1b[2;5r\x1b[?6h\x1b[2;1H\x1b[6n");
        assert_eq!(
            t.drain_responses(),
            b"\x1b[2;1R".to_vec(),
            "relatif à la région en mode origine"
        );
        feed(&mut t, "\x1b[?6n");
        assert_eq!(t.drain_responses(), b"\x1b[?2;1R".to_vec());
    }
}
