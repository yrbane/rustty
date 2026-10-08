//! Combinaisons de touches telles qu'écrites dans la configuration :
//! `ctrl+shift+t`, `shift+page_up`, `f5`.

use std::fmt;
use std::str::FromStr;

use bitflags::bitflags;
use serde::Deserialize;

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Mods: u8 {
        const CTRL = 1 << 0;
        const SHIFT = 1 << 1;
        const ALT = 1 << 2;
        const SUPER = 1 << 3;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Enter,
    Tab,
    Space,
    Backspace,
    Escape,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
    /// F1 à F24.
    F(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// Caractère produit par la touche, en minuscule.
    Char(char),
    Named(NamedKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub mods: Mods,
    pub key: Key,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("raccourci invalide « {input} » : {reason}")]
pub struct KeyParseError {
    pub input: String,
    pub reason: String,
}

const MODIFIERS: &[(&str, Mods)] = &[
    ("ctrl", Mods::CTRL),
    ("control", Mods::CTRL),
    ("shift", Mods::SHIFT),
    ("alt", Mods::ALT),
    ("option", Mods::ALT),
    ("super", Mods::SUPER),
    ("cmd", Mods::SUPER),
    ("win", Mods::SUPER),
    ("meta", Mods::SUPER),
];

const NAMED: &[(&str, NamedKey)] = &[
    ("enter", NamedKey::Enter),
    ("return", NamedKey::Enter),
    ("tab", NamedKey::Tab),
    ("space", NamedKey::Space),
    ("backspace", NamedKey::Backspace),
    ("escape", NamedKey::Escape),
    ("esc", NamedKey::Escape),
    ("delete", NamedKey::Delete),
    ("del", NamedKey::Delete),
    ("insert", NamedKey::Insert),
    ("home", NamedKey::Home),
    ("end", NamedKey::End),
    ("page_up", NamedKey::PageUp),
    ("pageup", NamedKey::PageUp),
    ("page_down", NamedKey::PageDown),
    ("pagedown", NamedKey::PageDown),
    ("left", NamedKey::Left),
    ("right", NamedKey::Right),
    ("up", NamedKey::Up),
    ("down", NamedKey::Down),
];

fn parse_key(token: &str) -> Option<Key> {
    let mut chars = token.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(Key::Char(c));
    }
    if let Some(named) = NAMED.iter().find(|(name, _)| *name == token) {
        return Some(Key::Named(named.1));
    }
    let n: u8 = token.strip_prefix('f')?.parse().ok()?;
    (1..=24).contains(&n).then_some(Key::Named(NamedKey::F(n)))
}

impl FromStr for KeyCombo {
    type Err = KeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fail = |reason: &str| KeyParseError {
            input: s.to_string(),
            reason: reason.to_string(),
        };
        let lowered = s.to_lowercase();
        let tokens: Vec<&str> = lowered.split('+').collect();
        let (key_token, mod_tokens) = tokens.split_last().ok_or_else(|| fail("vide"))?;
        let mut mods = Mods::empty();
        for token in mod_tokens {
            let Some((_, m)) = MODIFIERS.iter().find(|(name, _)| name == token) else {
                return Err(fail(&format!("modificateur inconnu « {token} »")));
            };
            if mods.contains(*m) {
                return Err(fail(&format!("modificateur « {token} » répété")));
            }
            mods |= *m;
        }
        if key_token.is_empty() {
            return Err(fail("il manque la touche après le dernier « + »"));
        }
        let key = parse_key(key_token)
            .ok_or_else(|| fail(&format!("touche inconnue « {key_token} »")))?;
        Ok(Self { mods, key })
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, m) in [
            ("ctrl", Mods::CTRL),
            ("shift", Mods::SHIFT),
            ("alt", Mods::ALT),
            ("super", Mods::SUPER),
        ] {
            if self.mods.contains(m) {
                write!(f, "{name}+")?;
            }
        }
        match self.key {
            Key::Char(c) => write!(f, "{c}"),
            Key::Named(NamedKey::F(n)) => write!(f, "f{n}"),
            Key::Named(named) => {
                let name = NAMED
                    .iter()
                    .find(|(_, k)| *k == named)
                    .map_or("?", |(n, _)| n);
                write!(f, "{name}")
            }
        }
    }
}

impl<'de> Deserialize<'de> for KeyCombo {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(s: &str) -> KeyCombo {
        s.parse().unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn parses_modifiers_and_character_keys() {
        assert_eq!(
            combo("ctrl+shift+t"),
            KeyCombo {
                mods: Mods::CTRL | Mods::SHIFT,
                key: Key::Char('t')
            }
        );
        assert_eq!(
            combo("Ctrl+Shift+T"),
            combo("ctrl+shift+t"),
            "casse indifférente, caractère normalisé en minuscule"
        );
        assert_eq!(
            combo("alt+1"),
            KeyCombo {
                mods: Mods::ALT,
                key: Key::Char('1')
            }
        );
        assert_eq!(
            combo("super+é"),
            KeyCombo {
                mods: Mods::SUPER,
                key: Key::Char('é')
            }
        );
        assert_eq!(
            combo("control+option+cmd+x").mods,
            Mods::CTRL | Mods::ALT | Mods::SUPER
        );
    }

    #[test]
    fn parses_named_keys_and_aliases() {
        assert_eq!(combo("shift+page_up").key, Key::Named(NamedKey::PageUp));
        assert_eq!(combo("shift+PageUp").key, Key::Named(NamedKey::PageUp));
        assert_eq!(combo("esc").key, Key::Named(NamedKey::Escape));
        assert_eq!(combo("return").key, Key::Named(NamedKey::Enter));
        assert_eq!(combo("f5").key, Key::Named(NamedKey::F(5)));
        assert_eq!(combo("ctrl+F12").key, Key::Named(NamedKey::F(12)));
        assert_eq!(combo("left").key, Key::Named(NamedKey::Left));
    }

    #[test]
    fn invalid_combos_are_rejected_with_the_offending_text() {
        for bad in [
            "",
            "ctrl+",
            "+t",
            "ctlr+t",
            "ctrl+shift+",
            "ctrl+ctrl+t",
            "f0",
            "f25",
            "ctrl+toto",
            "a+b",
        ] {
            let err = bad.parse::<KeyCombo>().unwrap_err();
            assert!(err.to_string().contains(bad), "{bad:?} → {err}");
        }
    }

    #[test]
    fn display_is_canonical_and_round_trips() {
        for (input, canonical) in [
            ("Shift+Ctrl+T", "ctrl+shift+t"),
            ("cmd+alt+space", "alt+super+space"),
            ("shift+pageup", "shift+page_up"),
            ("F5", "f5"),
            ("x", "x"),
        ] {
            let c = combo(input);
            assert_eq!(c.to_string(), canonical);
            assert_eq!(combo(canonical), c);
        }
    }

    #[test]
    fn deserializes_from_toml_string() {
        #[derive(Debug, serde::Deserialize)]
        struct Doc {
            k: KeyCombo,
        }
        let d: Doc = toml::from_str("k = \"ctrl+shift+e\"").unwrap();
        assert_eq!(d.k, combo("ctrl+shift+e"));
        let err = toml::from_str::<Doc>("k = \"ctrl+\"").unwrap_err();
        assert!(err.to_string().contains("ctrl+"), "{err}");
    }
}
