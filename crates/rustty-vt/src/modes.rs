//! Modes DEC privés et ANSI qui changent l'interprétation du flux ou ce que
//! l'hôte doit envoyer (touches curseur, souris, collage).

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MouseMode {
    #[default]
    None,
    /// 9 : clics seulement, sans relâchement.
    X10,
    /// 1000 : pressions et relâchements.
    Normal,
    /// 1002 : plus les mouvements bouton enfoncé.
    ButtonEvent,
    /// 1003 : tous les mouvements.
    AnyEvent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modes {
    /// DECCKM (1) : flèches en mode application.
    pub app_cursor_keys: bool,
    /// DECAWM (7).
    pub autowrap: bool,
    /// DECTCEM (25).
    pub cursor_visible: bool,
    /// 12.
    pub cursor_blink: bool,
    /// DECOM (6) : positions relatives à la région de défilement.
    pub origin: bool,
    /// IRM (ANSI 4) : insertion au lieu d'écrasement.
    pub insert: bool,
    /// 2004.
    pub bracketed_paste: bool,
    /// 47 / 1047 / 1049.
    pub alt_screen: bool,
    /// 1004.
    pub focus_events: bool,
    /// LNM (ANSI 20) : LF implique CR.
    pub line_feed_new_line: bool,
    pub mouse: MouseMode,
    /// 1006 : encodage SGR des événements souris.
    pub mouse_sgr: bool,
}

impl Modes {
    /// Vrai si un mode qui change le travail de l'hôte diffère : encodage des
    /// entrées (souris, touches application, collage, focus) ou écran affiché.
    pub fn differs_for_host(&self, other: &Modes) -> bool {
        self.mouse != other.mouse
            || self.mouse_sgr != other.mouse_sgr
            || self.alt_screen != other.alt_screen
            || self.bracketed_paste != other.bracketed_paste
            || self.focus_events != other.focus_events
            || self.app_cursor_keys != other.app_cursor_keys
    }
}

impl Default for Modes {
    fn default() -> Self {
        Self {
            app_cursor_keys: false,
            autowrap: true,
            cursor_visible: true,
            cursor_blink: true,
            origin: false,
            insert: false,
            bracketed_paste: false,
            alt_screen: false,
            focus_events: false,
            line_feed_new_line: false,
            mouse: MouseMode::None,
            mouse_sgr: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_a_fresh_xterm() {
        let m = Modes::default();
        assert!(m.autowrap && m.cursor_visible && m.cursor_blink);
        assert!(
            !m.app_cursor_keys
                && !m.origin
                && !m.insert
                && !m.bracketed_paste
                && !m.alt_screen
                && !m.focus_events
                && !m.line_feed_new_line
                && !m.mouse_sgr
        );
        assert_eq!(m.mouse, MouseMode::None);
    }

    #[test]
    fn host_relevant_modes_are_the_ones_that_change_input_encoding_or_screen() {
        let a = Modes::default();
        let mut b = a;
        b.cursor_visible = false;
        b.insert = true;
        assert!(!a.differs_for_host(&b), "curseur et insertion : interne");
        for f in [
            |m: &mut Modes| m.mouse = MouseMode::X10,
            |m: &mut Modes| m.mouse_sgr = true,
            |m: &mut Modes| m.alt_screen = true,
            |m: &mut Modes| m.bracketed_paste = true,
            |m: &mut Modes| m.focus_events = true,
            |m: &mut Modes| m.app_cursor_keys = true,
        ] {
            let mut c = a;
            f(&mut c);
            assert!(a.differs_for_host(&c));
        }
    }
}
