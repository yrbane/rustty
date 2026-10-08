//! Modes DEC privés (DECSET/DECRST), modes ANSI (SM/RM), écrans alternatifs et
//! forme du curseur (DECSCUSR).

use super::Term;
use crate::cursor::CursorShape;
use crate::modes::{Modes, MouseMode};
use crate::outbox::TermEvent;

impl Term {
    /// Signale à l'hôte un changement de mode qui le concerne, une seule fois.
    pub(crate) fn notify_if_host_modes_changed(&mut self, before: Modes) {
        if before.differs_for_host(&self.modes) {
            self.outbox.event(TermEvent::ModeChanged(self.modes));
        }
    }

    pub(crate) fn enter_alt_screen(&mut self, save_cursor: bool, clear: bool) {
        if self.modes.alt_screen {
            return;
        }
        if save_cursor {
            self.save_cursor();
        }
        self.modes.alt_screen = true;
        if clear {
            let template = self.erase_template();
            self.alt_grid.clear(template);
        }
    }

    pub(crate) fn leave_alt_screen(&mut self, restore_cursor: bool) {
        if !self.modes.alt_screen {
            return;
        }
        self.modes.alt_screen = false;
        if restore_cursor {
            self.restore_cursor();
        }
    }

    pub(crate) fn set_dec_mode(&mut self, mode: u16, on: bool) {
        let before = self.modes;
        let mouse = |m: MouseMode| if on { m } else { MouseMode::None };
        match mode {
            1 => self.modes.app_cursor_keys = on,
            6 => {
                self.modes.origin = on;
                self.cursor_to(0, 0);
            }
            7 => self.modes.autowrap = on,
            9 => self.modes.mouse = mouse(MouseMode::X10),
            12 => self.modes.cursor_blink = on,
            25 => self.modes.cursor_visible = on,
            47 => {
                if on {
                    self.enter_alt_screen(false, false);
                } else {
                    self.leave_alt_screen(false);
                }
            }
            1000 => self.modes.mouse = mouse(MouseMode::Normal),
            1002 => self.modes.mouse = mouse(MouseMode::ButtonEvent),
            1003 => self.modes.mouse = mouse(MouseMode::AnyEvent),
            1004 => self.modes.focus_events = on,
            1006 => self.modes.mouse_sgr = on,
            1047 => {
                if on {
                    self.enter_alt_screen(false, false);
                } else {
                    let template = self.erase_template();
                    self.alt_grid.clear(template);
                    self.leave_alt_screen(false);
                }
            }
            1049 => {
                if on {
                    self.enter_alt_screen(true, true);
                } else {
                    self.leave_alt_screen(true);
                }
            }
            2004 => self.modes.bracketed_paste = on,
            _ => {}
        }
        self.notify_if_host_modes_changed(before);
    }

    pub(crate) fn set_ansi_mode(&mut self, mode: u16, on: bool) {
        match mode {
            4 => self.modes.insert = on,
            20 => self.modes.line_feed_new_line = on,
            _ => {}
        }
    }

    pub(crate) fn set_cursor_shape(&mut self, param: u16) {
        self.cursor_shape = CursorShape::from_decscusr(param);
    }
}

#[cfg(test)]
mod tests {
    use crate::cursor::CursorShape;
    use crate::modes::{Modes, MouseMode};
    use crate::term::test_support::{feed, term};

    #[test]
    fn mode_1049_saves_cursor_switches_and_restores() {
        let mut t = term(5, 2);
        feed(&mut t, "main\x1b[?1049h");
        assert!(t.modes().alt_screen);
        assert_eq!(t.text(), vec!["", ""], "l'écran alternatif démarre vide");
        assert_eq!(t.cursor().col, 4, "la position est conservée à l'entrée");
        feed(&mut t, "\x1b[Halt\x1b[?1049l");
        assert!(!t.modes().alt_screen);
        assert_eq!(t.text(), vec!["main", ""]);
        assert_eq!(t.cursor().col, 4, "le curseur est restauré à la sortie");
        feed(&mut t, "\x1b[?1049h");
        assert_eq!(
            t.text(),
            vec!["", ""],
            "une nouvelle entrée repart d'un écran vide"
        );
    }

    #[test]
    fn mode_47_switches_without_saving_cursor() {
        let mut t = term(5, 2);
        feed(&mut t, "ab\x1b[?47h\x1b[2;3Hx\x1b[?47l");
        assert_eq!((t.cursor().col, t.cursor().row), (3, 1));
        assert_eq!(t.text(), vec!["ab", ""]);
    }

    #[test]
    fn alt_screen_never_feeds_scrollback() {
        let mut t = term(1, 1);
        feed(&mut t, "\x1b[?1049ha\r\nb\r\nc\x1b[?1049l");
        assert!(t.scrollback().is_empty());
    }

    #[test]
    fn dec_private_flags() {
        let mut t = term(5, 2);
        feed(
            &mut t,
            "\x1b[?1h\x1b[?7l\x1b[?25l\x1b[?12l\x1b[?1004h\x1b[?2004h",
        );
        let m = *t.modes();
        assert!(
            m.app_cursor_keys
                && !m.autowrap
                && !m.cursor_visible
                && !m.cursor_blink
                && m.focus_events
                && m.bracketed_paste
        );
        feed(
            &mut t,
            "\x1b[?1l\x1b[?7h\x1b[?25h\x1b[?12h\x1b[?1004l\x1b[?2004l",
        );
        assert_eq!(*t.modes(), Modes::default());
    }

    #[test]
    fn several_modes_in_one_sequence() {
        let mut t = term(5, 2);
        feed(&mut t, "\x1b[?1;25;2004h");
        assert!(t.modes().app_cursor_keys && t.modes().bracketed_paste && t.modes().cursor_visible);
    }

    #[test]
    fn origin_mode_homes_cursor_inside_region() {
        let mut t = term(5, 5);
        feed(&mut t, "\x1b[2;4r\x1b[4;4H\x1b[?6h");
        assert!(t.modes().origin);
        assert_eq!((t.cursor().col, t.cursor().row), (0, 1));
        feed(&mut t, "\x1b[?6l");
        assert_eq!((t.cursor().col, t.cursor().row), (0, 0));
    }

    #[test]
    fn mouse_modes() {
        let mut t = term(5, 1);
        for (seq, expected) in [
            ("\x1b[?9h", MouseMode::X10),
            ("\x1b[?1000h", MouseMode::Normal),
            ("\x1b[?1002h", MouseMode::ButtonEvent),
            ("\x1b[?1003h", MouseMode::AnyEvent),
            ("\x1b[?1003l", MouseMode::None),
        ] {
            feed(&mut t, seq);
            assert_eq!(t.modes().mouse, expected, "{seq:?}");
        }
        feed(&mut t, "\x1b[?1006h");
        assert!(t.modes().mouse_sgr);
    }

    #[test]
    fn ansi_insert_mode_and_line_feed_new_line() {
        let mut t = term(5, 2);
        feed(&mut t, "abc\x1b[1;1H\x1b[4hX\x1b[4l");
        assert_eq!(t.text(), vec!["Xabc", ""]);
        feed(&mut t, "\x1b[20h\x1b[1;3H\n");
        assert_eq!(
            (t.cursor().col, t.cursor().row),
            (0, 1),
            "LNM : LF implique CR"
        );
        feed(&mut t, "\x1b[20l");
        assert!(!t.modes().line_feed_new_line);
    }

    #[test]
    fn cursor_shape_via_decscusr() {
        let mut t = term(5, 1);
        for (seq, shape) in [
            ("\x1b[3 q", CursorShape::Underline),
            ("\x1b[6 q", CursorShape::Beam),
            ("\x1b[1 q", CursorShape::Block),
            ("\x1b[0 q", CursorShape::Block),
        ] {
            feed(&mut t, seq);
            assert_eq!(t.cursor_shape(), shape, "{seq:?}");
        }
    }

    #[test]
    fn host_relevant_mode_changes_emit_an_event_once() {
        use crate::outbox::TermEvent;
        let mut t = term(5, 1);
        feed(&mut t, "\x1b[?1000h");
        let evs = t.drain_events();
        assert_eq!(evs.len(), 1);
        assert!(matches!(&evs[0], TermEvent::ModeChanged(m) if m.mouse == MouseMode::Normal));
        feed(&mut t, "\x1b[?1000h");
        assert!(
            t.drain_events().is_empty(),
            "pas de changement, pas d'événement"
        );
        feed(&mut t, "\x1b[?25l\x1b[4h");
        assert!(
            t.drain_events().is_empty(),
            "curseur et insertion n'intéressent pas l'hôte"
        );
        for seq in [
            "\x1b[?1049h",
            "\x1b[?2004h",
            "\x1b[?1004h",
            "\x1b[?1006h",
            "\x1b[?1h",
        ] {
            feed(&mut t, seq);
            assert_eq!(t.drain_events().len(), 1, "{seq:?}");
        }
        feed(&mut t, "\x1bc");
        assert!(
            t.drain_events()
                .iter()
                .any(|e| matches!(e, TermEvent::ModeChanged(_))),
            "RIS rétablit les modes"
        );
    }
}
