//! Dispatch pur : chaque méthode de `vte::Perform` traduit la séquence reçue en
//! appel vers le module propriétaire. Aucune logique ici.

use vte::{Params, Perform};

use super::Term;
use crate::outbox::TermEvent;

impl Perform for Term {
    fn print(&mut self, c: char) {
        self.print_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x07 => self.outbox.event(TermEvent::Bell),
            0x08 => self.backspace(),
            0x09 => self.horizontal_tab(),
            0x0A..=0x0C => self.linefeed(),
            0x0D => self.carriage_return(),
            0x0E => self.charsets.shift_out(),
            0x0F => self.charsets.shift_in(),
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        _params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}
}

#[cfg(test)]
mod tests {
    use crate::outbox::TermEvent;
    use crate::term::test_support::{feed, term};

    #[test]
    fn bell_emits_an_event() {
        let mut t = term(5, 1);
        feed(&mut t, "\x07");
        assert_eq!(t.drain_events(), vec![TermEvent::Bell]);
        assert!(t.drain_events().is_empty(), "drain vide la file");
    }

    #[test]
    fn shift_out_and_in_select_g1_then_g0() {
        let mut t = term(5, 1);
        t.charsets
            .designate(1, crate::charset::Charset::DecSpecialGraphics);
        feed(&mut t, "q\x0eq\x0fq");
        assert_eq!(t.text(), vec!["q─q"]);
    }
}
