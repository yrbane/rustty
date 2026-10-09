//! Encodage xterm des touches et du collage, pur et sans winit : ce qui part
//! vers le shell quand une touche n'est pas un raccourci.

use rustty_config::{Key, KeyCombo, Mods, NamedKey};
use rustty_vt::Modes;

const ESC: u8 = 0x1b;

/// Paramètre xterm des modificateurs : 1 + shift + 2·alt + 4·ctrl + 8·super.
fn modifier_param(mods: Mods) -> u8 {
    1 + u8::from(mods.contains(Mods::SHIFT))
        + 2 * u8::from(mods.contains(Mods::ALT))
        + 4 * u8::from(mods.contains(Mods::CTRL))
        + 8 * u8::from(mods.contains(Mods::SUPER))
}

pub fn encode_key(
    combo: Option<KeyCombo>,
    text: Option<&str>,
    mods: Mods,
    modes: &Modes,
) -> Vec<u8> {
    match combo.map(|c| c.key) {
        Some(Key::Named(named)) => encode_named(named, mods, modes),
        Some(Key::Char(c)) if mods.contains(Mods::CTRL) => {
            with_alt_prefix(control_char(c).map(|b| vec![b]).unwrap_or_default(), mods)
        }
        _ => with_alt_prefix(
            text.map(|t| t.as_bytes().to_vec()).unwrap_or_default(),
            mods,
        ),
    }
}

/// Alt sur une touche à un octet : préfixe `ESC` (meta), comme xterm.
fn with_alt_prefix(mut bytes: Vec<u8>, mods: Mods) -> Vec<u8> {
    if mods.contains(Mods::ALT) && !bytes.is_empty() {
        bytes.insert(0, ESC);
    }
    bytes
}

fn control_char(c: char) -> Option<u8> {
    Some(match c {
        'a'..='z' => (c as u8) & 0x1f,
        'A'..='Z' => (c.to_ascii_lowercase() as u8) & 0x1f,
        ' ' | '@' | '2' => 0x00,
        '[' | '3' => 0x1b,
        '\\' | '4' => 0x1c,
        ']' | '5' => 0x1d,
        '^' | '6' => 0x1e,
        '_' | '7' => 0x1f,
        '?' | '8' => 0x7f,
        // Sans contrôle associé, xterm envoie le chiffre tel quel.
        '0' | '1' | '9' => c as u8,
        _ => return None,
    })
}

/// Séquence `ESC [ … X` ou `ESC O X` d'une touche à lettre finale.
fn letter_key(letter: u8, mods: Mods, application: bool) -> Vec<u8> {
    if mods.is_empty() {
        let intro = if application { b'O' } else { b'[' };
        vec![ESC, intro, letter]
    } else {
        format!("\x1b[1;{}{}", modifier_param(mods), letter as char).into_bytes()
    }
}

/// Séquence `ESC [ n ~` ou `ESC [ n ; m ~`.
fn tilde_key(code: u8, mods: Mods) -> Vec<u8> {
    if mods.is_empty() {
        format!("\x1b[{code}~").into_bytes()
    } else {
        format!("\x1b[{code};{}~", modifier_param(mods)).into_bytes()
    }
}

/// Touches nommées : les touches à un octet prennent le préfixe `ESC` pour
/// alt ; les séquences CSI/SS3 portent alt dans le paramètre de modificateurs.
fn encode_named(named: NamedKey, mods: Mods, modes: &Modes) -> Vec<u8> {
    let ctrl = mods.contains(Mods::CTRL);
    let app = modes.app_cursor_keys;
    let simple = |bytes: Vec<u8>| with_alt_prefix(bytes, mods);
    match named {
        NamedKey::Enter => simple(vec![b'\r']),
        NamedKey::Tab if mods.contains(Mods::SHIFT) => simple(b"\x1b[Z".to_vec()),
        NamedKey::Tab => simple(vec![b'\t']),
        NamedKey::Backspace => simple(vec![if ctrl { 0x08 } else { 0x7f }]),
        NamedKey::Escape => simple(vec![ESC]),
        NamedKey::Space => simple(vec![if ctrl { 0x00 } else { b' ' }]),
        NamedKey::Insert => tilde_key(2, mods),
        NamedKey::Delete => tilde_key(3, mods),
        NamedKey::PageUp => tilde_key(5, mods),
        NamedKey::PageDown => tilde_key(6, mods),
        NamedKey::Up => letter_key(b'A', mods, app),
        NamedKey::Down => letter_key(b'B', mods, app),
        NamedKey::Right => letter_key(b'C', mods, app),
        NamedKey::Left => letter_key(b'D', mods, app),
        NamedKey::Home => letter_key(b'H', mods, app),
        NamedKey::End => letter_key(b'F', mods, app),
        NamedKey::F(n @ 1..=4) => letter_key(b'P' + (n - 1), mods, true),
        NamedKey::F(n @ 5..=12) => {
            const CODES: [u8; 8] = [15, 17, 18, 19, 20, 21, 23, 24];
            tilde_key(CODES[usize::from(n - 5)], mods)
        }
        NamedKey::F(_) => Vec::new(),
    }
}

/// Le texte collé, prêt pour le PTY : fins de ligne en `\r`, contrôles
/// filtrés, encadrement si l'application l'a demandé (mode 2004).
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    let normalized = text.replace("\r\n", "\r").replace('\n', "\r");
    let body: String = if bracketed {
        normalized.replace("\x1b[201~", "")
    } else {
        normalized
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\r' | '\t'))
            .collect()
    };
    if bracketed {
        format!("\x1b[200~{body}\x1b[201~").into_bytes()
    } else {
        body.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustty_config::NamedKey as N;

    fn named(n: N, mods: Mods) -> Option<KeyCombo> {
        Some(KeyCombo {
            mods,
            key: Key::Named(n),
        })
    }

    fn ch(c: char, mods: Mods) -> Option<KeyCombo> {
        Some(KeyCombo {
            mods,
            key: Key::Char(c),
        })
    }

    fn enc(combo: Option<KeyCombo>, text: Option<&str>, mods: Mods) -> Vec<u8> {
        encode_key(combo, text, mods, &Modes::default())
    }

    #[test]
    fn plain_text_goes_through_as_utf8() {
        assert_eq!(enc(ch('a', Mods::empty()), Some("a"), Mods::empty()), b"a");
        assert_eq!(
            enc(ch('é', Mods::SHIFT), Some("É"), Mods::SHIFT),
            "É".as_bytes()
        );
    }

    #[test]
    fn control_letters_and_symbols() {
        assert_eq!(enc(ch('c', Mods::CTRL), None, Mods::CTRL), vec![0x03]);
        assert_eq!(enc(ch('[', Mods::CTRL), None, Mods::CTRL), vec![0x1b]);
        assert_eq!(enc(ch('?', Mods::CTRL), None, Mods::CTRL), vec![0x7f]);
        assert_eq!(enc(ch('@', Mods::CTRL), None, Mods::CTRL), vec![0x00]);
        assert_eq!(
            enc(named(N::Space, Mods::CTRL), Some(" "), Mods::CTRL),
            vec![0x00]
        );
    }

    #[test]
    fn ctrl_digits_without_a_control_send_the_digit() {
        for d in ['0', '1', '9'] {
            assert_eq!(
                enc(ch(d, Mods::CTRL), None, Mods::CTRL),
                vec![d as u8],
                "ctrl+{d}"
            );
        }
        assert_eq!(enc(ch('2', Mods::CTRL), None, Mods::CTRL), vec![0x00]);
        assert_eq!(enc(ch('8', Mods::CTRL), None, Mods::CTRL), vec![0x7f]);
    }

    #[test]
    fn alt_prefixes_with_escape() {
        assert_eq!(enc(ch('x', Mods::ALT), Some("x"), Mods::ALT), b"\x1bx");
        assert_eq!(
            enc(
                ch('c', Mods::ALT | Mods::CTRL),
                None,
                Mods::ALT | Mods::CTRL
            ),
            vec![0x1b, 0x03]
        );
        assert_eq!(
            enc(named(N::Backspace, Mods::ALT), None, Mods::ALT),
            vec![0x1b, 0x7f]
        );
    }

    #[test]
    fn named_keys_without_modifiers() {
        assert_eq!(
            enc(named(N::Enter, Mods::empty()), Some("\r"), Mods::empty()),
            b"\r"
        );
        assert_eq!(
            enc(named(N::Tab, Mods::empty()), Some("\t"), Mods::empty()),
            b"\t"
        );
        assert_eq!(
            enc(named(N::Tab, Mods::SHIFT), None, Mods::SHIFT),
            b"\x1b[Z"
        );
        assert_eq!(
            enc(named(N::Backspace, Mods::empty()), None, Mods::empty()),
            vec![0x7f]
        );
        assert_eq!(
            enc(named(N::Backspace, Mods::CTRL), None, Mods::CTRL),
            vec![0x08]
        );
        assert_eq!(
            enc(named(N::Escape, Mods::empty()), None, Mods::empty()),
            vec![0x1b]
        );
        assert_eq!(
            enc(named(N::Delete, Mods::empty()), None, Mods::empty()),
            b"\x1b[3~"
        );
        assert_eq!(
            enc(named(N::PageUp, Mods::empty()), None, Mods::empty()),
            b"\x1b[5~"
        );
        assert_eq!(
            enc(named(N::Up, Mods::empty()), None, Mods::empty()),
            b"\x1b[A"
        );
        assert_eq!(
            enc(named(N::End, Mods::empty()), None, Mods::empty()),
            b"\x1b[F"
        );
        assert_eq!(
            enc(named(N::F(1), Mods::empty()), None, Mods::empty()),
            b"\x1bOP"
        );
        assert_eq!(
            enc(named(N::F(5), Mods::empty()), None, Mods::empty()),
            b"\x1b[15~"
        );
        assert_eq!(
            enc(named(N::F(12), Mods::empty()), None, Mods::empty()),
            b"\x1b[24~"
        );
    }

    #[test]
    fn modifier_parameter_follows_xterm() {
        assert_eq!(
            enc(named(N::Up, Mods::SHIFT), None, Mods::SHIFT),
            b"\x1b[1;2A"
        );
        assert_eq!(
            enc(named(N::Right, Mods::CTRL), None, Mods::CTRL),
            b"\x1b[1;5C"
        );
        assert_eq!(
            enc(
                named(N::Left, Mods::ALT | Mods::SHIFT),
                None,
                Mods::ALT | Mods::SHIFT
            ),
            b"\x1b[1;4D"
        );
        assert_eq!(
            enc(named(N::Delete, Mods::CTRL), None, Mods::CTRL),
            b"\x1b[3;5~"
        );
        assert_eq!(
            enc(named(N::F(1), Mods::SHIFT), None, Mods::SHIFT),
            b"\x1b[1;2P"
        );
        assert_eq!(
            enc(named(N::F(5), Mods::SUPER), None, Mods::SUPER),
            b"\x1b[15;9~"
        );
    }

    #[test]
    fn application_cursor_mode_uses_ss3_only_without_modifiers() {
        let modes = Modes {
            app_cursor_keys: true,
            ..Default::default()
        };
        assert_eq!(
            encode_key(named(N::Up, Mods::empty()), None, Mods::empty(), &modes),
            b"\x1bOA"
        );
        assert_eq!(
            encode_key(named(N::Home, Mods::empty()), None, Mods::empty(), &modes),
            b"\x1bOH"
        );
        assert_eq!(
            encode_key(named(N::Up, Mods::CTRL), None, Mods::CTRL, &modes),
            b"\x1b[1;5A"
        );
    }

    #[test]
    fn unknown_keys_send_nothing() {
        assert!(enc(None, None, Mods::empty()).is_empty());
        assert!(enc(None, None, Mods::CTRL | Mods::SHIFT).is_empty());
        assert_eq!(
            enc(None, Some("ß"), Mods::empty()),
            "ß".as_bytes(),
            "un texte sans combo (touche morte résolue) part quand même"
        );
    }

    #[test]
    fn paste_is_sanitized() {
        assert_eq!(paste_bytes("a\r\nb\nc", false), b"a\rb\rc");
        assert_eq!(paste_bytes("x\x1b[201~y", true), b"\x1b[200~xy\x1b[201~");
        assert_eq!(paste_bytes("t\tv\x07w\x1b[31m", false), b"t\tvw[31m");
        assert_eq!(paste_bytes("", true), b"\x1b[200~\x1b[201~");
    }
}
