//! Traduit une touche winit en `KeyCombo` de la configuration, pour résoudre
//! les raccourcis avant tout encodage.

use rustty_config::{Key as ConfigKey, KeyCombo, Mods, NamedKey as ConfigNamed};
use winit::keyboard::{Key, ModifiersState, NamedKey};

pub fn mods_from_winit(state: ModifiersState) -> Mods {
    let mut mods = Mods::empty();
    mods.set(Mods::CTRL, state.control_key());
    mods.set(Mods::SHIFT, state.shift_key());
    mods.set(Mods::ALT, state.alt_key());
    mods.set(Mods::SUPER, state.super_key());
    mods
}

/// La combinaison de la configuration correspondant à cette touche, ou `None`
/// si elle ne peut pas porter de raccourci.
pub fn key_combo(logical: &Key, mods: Mods) -> Option<KeyCombo> {
    let key = match logical {
        Key::Character(s) => {
            let mut chars = s.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            ConfigKey::Char(c.to_lowercase().next().unwrap_or(c))
        }
        Key::Named(named) => ConfigKey::Named(named_key(*named)?),
        _ => return None,
    };
    Some(KeyCombo { mods, key })
}

fn named_key(named: NamedKey) -> Option<ConfigNamed> {
    Some(match named {
        NamedKey::Enter => ConfigNamed::Enter,
        NamedKey::Tab => ConfigNamed::Tab,
        NamedKey::Space => ConfigNamed::Space,
        NamedKey::Backspace => ConfigNamed::Backspace,
        NamedKey::Escape => ConfigNamed::Escape,
        NamedKey::Delete => ConfigNamed::Delete,
        NamedKey::Insert => ConfigNamed::Insert,
        NamedKey::Home => ConfigNamed::Home,
        NamedKey::End => ConfigNamed::End,
        NamedKey::PageUp => ConfigNamed::PageUp,
        NamedKey::PageDown => ConfigNamed::PageDown,
        NamedKey::ArrowLeft => ConfigNamed::Left,
        NamedKey::ArrowRight => ConfigNamed::Right,
        NamedKey::ArrowUp => ConfigNamed::Up,
        NamedKey::ArrowDown => ConfigNamed::Down,
        NamedKey::F1 => ConfigNamed::F(1),
        NamedKey::F2 => ConfigNamed::F(2),
        NamedKey::F3 => ConfigNamed::F(3),
        NamedKey::F4 => ConfigNamed::F(4),
        NamedKey::F5 => ConfigNamed::F(5),
        NamedKey::F6 => ConfigNamed::F(6),
        NamedKey::F7 => ConfigNamed::F(7),
        NamedKey::F8 => ConfigNamed::F(8),
        NamedKey::F9 => ConfigNamed::F(9),
        NamedKey::F10 => ConfigNamed::F(10),
        NamedKey::F11 => ConfigNamed::F(11),
        NamedKey::F12 => ConfigNamed::F(12),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::NamedKey as W;

    fn combo(key: Key, mods: Mods) -> Option<KeyCombo> {
        key_combo(&key, mods)
    }

    #[test]
    fn modifiers_map_one_to_one() {
        let all = ModifiersState::CONTROL
            | ModifiersState::SHIFT
            | ModifiersState::ALT
            | ModifiersState::SUPER;
        assert_eq!(
            mods_from_winit(all),
            Mods::CTRL | Mods::SHIFT | Mods::ALT | Mods::SUPER
        );
        assert_eq!(mods_from_winit(ModifiersState::empty()), Mods::empty());
    }

    #[test]
    fn letters_are_lowercased_so_config_combos_match() {
        let c = combo(Key::Character("T".into()), Mods::CTRL | Mods::SHIFT).unwrap();
        assert_eq!(c, "ctrl+shift+t".parse().unwrap());
        assert_eq!(
            combo(Key::Character("é".into()), Mods::empty())
                .unwrap()
                .key,
            ConfigKey::Char('é')
        );
    }

    #[test]
    fn named_keys_and_function_keys() {
        assert_eq!(
            combo(Key::Named(W::ArrowLeft), Mods::SHIFT).unwrap(),
            "shift+left".parse().unwrap()
        );
        assert_eq!(
            combo(Key::Named(W::F5), Mods::empty()).unwrap().key,
            ConfigKey::Named(ConfigNamed::F(5))
        );
        assert_eq!(
            combo(Key::Named(W::Enter), Mods::empty()).unwrap().key,
            ConfigKey::Named(ConfigNamed::Enter)
        );
        assert_eq!(
            combo(Key::Named(W::Space), Mods::CTRL).unwrap().key,
            ConfigKey::Named(ConfigNamed::Space)
        );
    }

    #[test]
    fn unknown_or_modifier_only_keys_are_none() {
        assert!(combo(Key::Named(W::Shift), Mods::SHIFT).is_none());
        assert!(combo(Key::Named(W::AudioVolumeUp), Mods::empty()).is_none());
        assert!(combo(Key::Dead(None), Mods::empty()).is_none());
        assert!(
            combo(Key::Character("ab".into()), Mods::empty()).is_none(),
            "deux caractères = pas une touche"
        );
    }
}
