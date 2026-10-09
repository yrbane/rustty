//! Rapports souris pour les applications qui les demandent (modes 9, 1000,
//! 1002, 1003 ; encodage legacy ou SGR 1006). Pur.

use rustty_config::Mods;
use rustty_vt::{Modes, MouseMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

impl MouseButton {
    fn code(self) -> u32 {
        match self {
            Self::Left => 0,
            Self::Middle => 1,
            Self::Right => 2,
            Self::WheelUp => 64,
            Self::WheelDown => 65,
        }
    }

    fn is_wheel(self) -> bool {
        matches!(self, Self::WheelUp | Self::WheelDown)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseKind {
    Press,
    Release,
    Motion,
}

/// Plafond des coordonnées de l'encodage legacy (un octet, `32 + n`).
const LEGACY_MAX: usize = 223;

fn modifier_bits(mods: Mods) -> u32 {
    4 * u32::from(mods.contains(Mods::SHIFT))
        + 8 * u32::from(mods.contains(Mods::ALT))
        + 16 * u32::from(mods.contains(Mods::CTRL))
}

/// Vrai si le mode courant veut connaître cet événement.
fn wanted(kind: MouseKind, button: Option<MouseButton>, mode: MouseMode) -> bool {
    match (mode, kind) {
        (MouseMode::None, _) => false,
        (MouseMode::X10, kind) => kind == MouseKind::Press,
        (_, MouseKind::Press) => true,
        (_, MouseKind::Release) => !button.is_some_and(MouseButton::is_wheel),
        (MouseMode::ButtonEvent, MouseKind::Motion) => button.is_some_and(|b| !b.is_wheel()),
        (MouseMode::AnyEvent, MouseKind::Motion) => true,
        (MouseMode::Normal, MouseKind::Motion) => false,
    }
}

pub fn encode_mouse(
    kind: MouseKind,
    button: Option<MouseButton>,
    cell: Option<(usize, usize)>,
    mods: Mods,
    modes: &Modes,
) -> Option<Vec<u8>> {
    let (col, row) = cell?;
    if !wanted(kind, button, modes.mouse) {
        return None;
    }
    let base = match button {
        Some(b) => b.code(),
        None => 3,
    };
    let mods_bits = if modes.mouse == MouseMode::X10 {
        0
    } else {
        modifier_bits(mods)
    };
    let motion = if kind == MouseKind::Motion { 32 } else { 0 };
    if modes.mouse_sgr {
        let suffix = if kind == MouseKind::Release { 'm' } else { 'M' };
        let cb = base + mods_bits + motion;
        return Some(format!("\x1b[<{cb};{};{}{suffix}", col + 1, row + 1).into_bytes());
    }
    if col >= LEGACY_MAX || row >= LEGACY_MAX {
        return None;
    }
    let cb = if kind == MouseKind::Release { 3 } else { base } + mods_bits + motion;
    Some(vec![
        0x1b,
        b'[',
        b'M',
        32 + cb as u8,
        32 + col as u8 + 1,
        32 + row as u8 + 1,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modes(mouse: MouseMode, sgr: bool) -> Modes {
        Modes {
            mouse,
            mouse_sgr: sgr,
            ..Default::default()
        }
    }

    #[test]
    fn no_mode_means_no_report() {
        assert!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::Left),
                Some((0, 0)),
                Mods::empty(),
                &modes(MouseMode::None, true)
            )
            .is_none()
        );
    }

    #[test]
    fn reports_require_a_cell() {
        assert!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::Left),
                None,
                Mods::empty(),
                &modes(MouseMode::Normal, true)
            )
            .is_none()
        );
    }

    #[test]
    fn legacy_press_and_release() {
        let m = modes(MouseMode::Normal, false);
        assert_eq!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::Left),
                Some((0, 0)),
                Mods::empty(),
                &m
            )
            .unwrap(),
            b"\x1b[M\x20\x21\x21"
        );
        assert_eq!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::Right),
                Some((4, 9)),
                Mods::empty(),
                &m
            )
            .unwrap(),
            b"\x1b[M\x22\x25\x2a"
        );
        assert_eq!(
            encode_mouse(
                MouseKind::Release,
                Some(MouseButton::Left),
                Some((0, 0)),
                Mods::empty(),
                &m
            )
            .unwrap(),
            b"\x1b[M\x23\x21\x21"
        );
        assert_eq!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::WheelUp),
                Some((0, 0)),
                Mods::empty(),
                &m
            )
            .unwrap(),
            b"\x1b[M\x60\x21\x21"
        );
        assert!(
            encode_mouse(
                MouseKind::Release,
                Some(MouseButton::WheelDown),
                Some((0, 0)),
                Mods::empty(),
                &m
            )
            .is_none()
        );
        assert!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::Left),
                Some((223, 0)),
                Mods::empty(),
                &m
            )
            .is_none(),
            "au-delà de 223 colonnes, le legacy ne peut rien dire"
        );
    }

    #[test]
    fn sgr_press_release_and_modifiers() {
        let m = modes(MouseMode::Normal, true);
        assert_eq!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::Left),
                Some((0, 0)),
                Mods::empty(),
                &m
            )
            .unwrap(),
            b"\x1b[<0;1;1M"
        );
        assert_eq!(
            encode_mouse(
                MouseKind::Release,
                Some(MouseButton::Middle),
                Some((10, 2)),
                Mods::empty(),
                &m
            )
            .unwrap(),
            b"\x1b[<1;11;3m"
        );
        assert_eq!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::Left),
                Some((300, 0)),
                Mods::CTRL | Mods::SHIFT,
                &m
            )
            .unwrap(),
            b"\x1b[<20;301;1M"
        );
        assert_eq!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::WheelDown),
                Some((0, 0)),
                Mods::ALT,
                &m
            )
            .unwrap(),
            b"\x1b[<73;1;1M"
        );
    }

    #[test]
    fn x10_reports_presses_only_without_modifiers() {
        let m = modes(MouseMode::X10, true);
        assert_eq!(
            encode_mouse(
                MouseKind::Press,
                Some(MouseButton::Left),
                Some((0, 0)),
                Mods::CTRL,
                &m
            )
            .unwrap(),
            b"\x1b[<0;1;1M"
        );
        assert!(
            encode_mouse(
                MouseKind::Release,
                Some(MouseButton::Left),
                Some((0, 0)),
                Mods::empty(),
                &m
            )
            .is_none()
        );
        assert!(
            encode_mouse(
                MouseKind::Motion,
                Some(MouseButton::Left),
                Some((0, 0)),
                Mods::empty(),
                &m
            )
            .is_none()
        );
    }

    #[test]
    fn motion_depends_on_the_mode_and_the_held_button() {
        let button_mode = modes(MouseMode::ButtonEvent, true);
        assert_eq!(
            encode_mouse(
                MouseKind::Motion,
                Some(MouseButton::Left),
                Some((1, 1)),
                Mods::empty(),
                &button_mode
            )
            .unwrap(),
            b"\x1b[<32;2;2M"
        );
        assert!(
            encode_mouse(
                MouseKind::Motion,
                None,
                Some((1, 1)),
                Mods::empty(),
                &button_mode
            )
            .is_none(),
            "sans bouton, le mode 1002 ne rapporte pas"
        );
        let any = modes(MouseMode::AnyEvent, true);
        assert_eq!(
            encode_mouse(MouseKind::Motion, None, Some((1, 1)), Mods::empty(), &any).unwrap(),
            b"\x1b[<35;2;2M"
        );
        let legacy_any = modes(MouseMode::AnyEvent, false);
        assert_eq!(
            encode_mouse(
                MouseKind::Motion,
                None,
                Some((0, 0)),
                Mods::empty(),
                &legacy_any
            )
            .unwrap(),
            b"\x1b[M\x43\x21\x21"
        );
    }
}
