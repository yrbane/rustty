//! Dispatch pur : chaque méthode de `vte::Perform` traduit la séquence reçue en
//! appel vers le module propriétaire. Aucune logique ici.

use vte::{Params, Perform};

use super::Term;
use crate::charset::Charset;
use crate::outbox::TermEvent;
use crate::params::{arg_or, args, raw};

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

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        let p = args(params);
        let n = |i: usize| usize::from(arg_or(&p, i, 1));
        match (intermediates, action) {
            ([], 'A') => self.cursor_up(n(0)),
            ([], 'B' | 'e') => self.cursor_down(n(0)),
            ([], 'C' | 'a') => self.cursor_forward(n(0)),
            ([], 'D') => self.cursor_back(n(0)),
            ([], 'E') => {
                self.cursor_down(n(0));
                self.cursor_to_col(0);
            }
            ([], 'F') => {
                self.cursor_up(n(0));
                self.cursor_to_col(0);
            }
            ([], 'G' | '`') => self.cursor_to_col(n(0) - 1),
            ([], 'H' | 'f') => self.cursor_to(n(1) - 1, n(0) - 1),
            ([], 'd') => {
                let col = self.cursor.col;
                self.cursor_to(col, n(0) - 1);
            }
            ([], 'r') => self.set_scroll_region(raw(&p, 0), raw(&p, 1)),
            ([], 's') => self.save_cursor(),
            ([], 'u') => self.restore_cursor(),
            ([], 'K') => self.erase_in_line(raw(&p, 0)),
            ([], 'J') => self.erase_in_display(raw(&p, 0)),
            ([], 'X') => self.erase_chars(n(0)),
            ([], '@') => self.insert_blank_chars(n(0)),
            ([], 'P') => self.delete_chars(n(0)),
            ([], 'L') => self.insert_lines(n(0)),
            ([], 'M') => self.delete_lines(n(0)),
            ([], 'S') => self.scroll_up_region(n(0)),
            ([], 'T') => self.scroll_down_region(n(0)),
            ([], 'm') => crate::sgr::apply_sgr(&mut self.cursor.style, params),
            ([b'?'], 'h') => {
                for &m in &p {
                    self.set_dec_mode(m, true);
                }
            }
            ([b'?'], 'l') => {
                for &m in &p {
                    self.set_dec_mode(m, false);
                }
            }
            ([], 'h') => {
                for &m in &p {
                    self.set_ansi_mode(m, true);
                }
            }
            ([], 'l') => {
                for &m in &p {
                    self.set_ansi_mode(m, false);
                }
            }
            ([b' '], 'q') => self.set_cursor_shape(raw(&p, 0)),
            ([], 'c') => self.device_attributes(),
            ([b'>'], 'c') => self.secondary_device_attributes(),
            ([], 'n') => self.device_status_report(raw(&p, 0), false),
            ([b'?'], 'n') => self.device_status_report(raw(&p, 0), true),
            ([], 'g') => match raw(&p, 0) {
                0 => self.tabs.clear(self.cursor.col),
                3 => self.tabs.clear_all(),
                _ => {}
            },
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (intermediates, byte) {
            ([], b'7') => self.save_cursor(),
            ([], b'8') => self.restore_cursor(),
            ([], b'D') => self.linefeed(),
            ([], b'E') => {
                self.linefeed();
                self.carriage_return();
            }
            ([], b'H') => self.tabs.set(self.cursor.col),
            ([], b'M') => self.reverse_index(),
            ([], b'c') => self.reset(),
            ([b'('], b'0') => self.charsets.designate(0, Charset::DecSpecialGraphics),
            ([b'('], b'B') => self.charsets.designate(0, Charset::Ascii),
            ([b')'], b'0') => self.charsets.designate(1, Charset::DecSpecialGraphics),
            ([b')'], b'B') => self.charsets.designate(1, Charset::Ascii),
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let Some(code) = params.first() else { return };
        match *code {
            b"0" | b"2" => {
                let title = params
                    .get(1)
                    .map(|t| String::from_utf8_lossy(t).into_owned())
                    .unwrap_or_default();
                self.set_title(title);
            }
            b"52" => {
                if let Some(data) = params.get(2) {
                    self.set_clipboard_from_base64(data);
                }
            }
            _ => {}
        }
    }

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

    #[test]
    fn g0_dec_graphics_via_esc_paren_zero() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b(0qx\x1b(Bq");
        assert_eq!(t.text(), vec!["─│q"]);
    }

    #[test]
    fn g1_designated_then_selected_with_shift_out() {
        let mut t = term(5, 1);
        feed(&mut t, "\x1b)0q\x0eq\x0fq");
        assert_eq!(t.text(), vec!["q─q"]);
    }

    #[test]
    fn tab_stops_can_be_set_and_cleared() {
        let mut t = term(20, 1);
        feed(&mut t, "\x1b[1;4H\x1bH\x1b[1;1H\t");
        assert_eq!(t.cursor().col, 3, "HTS pose un taquet en colonne 4");
        feed(&mut t, "\x1b[g\x1b[1;1H\t");
        assert_eq!(
            t.cursor().col,
            8,
            "TBC 0 retire le taquet courant seulement"
        );
        feed(&mut t, "\x1b[3g\x1b[1;1H\t");
        assert_eq!(t.cursor().col, 19, "TBC 3 retire tous les taquets");
    }
}
