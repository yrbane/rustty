//! Tests du module pur des hyperliens.

use std::time::Instant;

use rustty_vt::Term;

use super::*;
use crate::mouse::click::DOUBLE_CLICK;

fn lines(cols: usize, bytes: &[u8]) -> Vec<Line> {
    let mut t = Term::new(cols, 4, 0);
    t.input(bytes);
    t.snapshot().lines
}

fn run_of(id: u16, range: CellRange) -> Option<(TermId, LinkId, CellRange)> {
    Some((TermId(1), LinkId::new(id).unwrap(), range))
}

#[test]
fn a_run_spans_wrapped_lines_from_either_half() {
    let l = lines(4, b"xx\x1b]8;;https://a\x1b\\abcd\x1b]8;;\x1b\\yy");
    let expected = CellRange {
        start: (0, 2),
        end: (1, 1),
    };
    assert_eq!(link_run(&l, 0, 3).map(|(_, r)| r), Some(expected));
    assert_eq!(link_run(&l, 1, 0).map(|(_, r)| r), Some(expected));
    assert_eq!(link_run(&l, 1, 2), None);
}

#[test]
fn a_run_stops_at_a_different_link_and_at_a_hard_newline() {
    let l = lines(
        8,
        b"\x1b]8;;https://a\x1b\\ab\x1b]8;;https://b\x1b\\cd\x1b]8;;\x1b\\\r\n\x1b]8;;https://b\x1b\\e",
    );
    let range = |s, e| CellRange { start: s, end: e };
    assert_eq!(
        link_run(&l, 0, 1).map(|(_, r)| r),
        Some(range((0, 0), (0, 1)))
    );
    assert_eq!(
        link_run(&l, 0, 3).map(|(_, r)| r),
        Some(range((0, 2), (0, 3)))
    );
    assert_eq!(
        link_run(&l, 1, 0).map(|(_, r)| r),
        Some(range((1, 0), (1, 0))),
        "pas de wrapped : la ligne 0 ne se joint pas"
    );
}

#[test]
fn the_gesture_needs_ctrl_or_shift_as_configured() {
    let g = |click, captured| LinkGesture { click, captured };
    assert!(gesture_armed(g(ClickMode::Plain, false), Mods::empty()));
    assert!(!gesture_armed(g(ClickMode::Ctrl, false), Mods::empty()));
    assert!(gesture_armed(g(ClickMode::Ctrl, false), Mods::CTRL));
    assert!(!gesture_armed(g(ClickMode::Plain, true), Mods::empty()));
    assert!(gesture_armed(g(ClickMode::Plain, true), Mods::SHIFT));
    assert!(!gesture_armed(g(ClickMode::Ctrl, true), Mods::CTRL));
    assert!(gesture_armed(
        g(ClickMode::Ctrl, true),
        Mods::CTRL | Mods::SHIFT
    ));
}

#[test]
fn a_drag_selects_and_a_still_click_opens() {
    let run = run_of(
        1,
        CellRange {
            start: (0, 0),
            end: (0, 3),
        },
    );
    let mut c = LinkClick::default();
    let t0 = Instant::now();
    assert!(
        !c.press(run, true, false),
        "mode plain : la sélection démarre"
    );
    assert_eq!(c.release(run, false, t0), None, "glisser = copie");
    c.press(run, true, false);
    assert_eq!(c.release(run, true, t0 + DOUBLE_CLICK * 2), LinkId::new(1));
}

#[test]
fn the_modifier_counts_at_press() {
    let run = run_of(
        1,
        CellRange {
            start: (0, 0),
            end: (0, 3),
        },
    );
    let mut c = LinkClick::default();
    c.press(run, false, true);
    assert_eq!(
        c.release(run, true, Instant::now()),
        None,
        "armé seulement à la relâche : rien"
    );
    assert!(
        c.press(run, true, true),
        "mode ctrl armé : l'appui est capté"
    );
    assert_eq!(c.release(run, true, Instant::now()), LinkId::new(1));
}

#[test]
fn a_release_elsewhere_or_a_second_click_does_not_open() {
    let run = run_of(
        1,
        CellRange {
            start: (0, 0),
            end: (0, 3),
        },
    );
    let other = run_of(
        2,
        CellRange {
            start: (1, 0),
            end: (1, 3),
        },
    );
    let mut c = LinkClick::default();
    let t0 = Instant::now();
    c.press(run, true, false);
    assert_eq!(c.release(other, true, t0), None);
    c.press(run, true, false);
    assert_eq!(c.release(run, true, t0), LinkId::new(1));
    c.press(run, true, false);
    assert_eq!(
        c.release(run, true, t0 + DOUBLE_CLICK / 2),
        None,
        "double clic : une seule ouverture"
    );
    c.press(run, true, false);
    assert_eq!(c.release(run, true, t0 + DOUBLE_CLICK * 2), LinkId::new(1));
}

#[test]
fn tooltip_text_truncates_in_the_middle_and_adds_the_hint() {
    let long = format!("https://exemple.org/{}/fin.html", "a".repeat(80));
    let text = tooltip_text(&long, Some("ctrl+clic"));
    assert!(text.starts_with("https://exemple.org/"), "{text}");
    assert!(text.ends_with("/fin.html · ctrl+clic"), "{text}");
    assert!(text.contains('…'));
    assert_eq!(text.split(" · ").next().unwrap().chars().count(), 60);
    assert_eq!(tooltip_text("https://x", None), "https://x");
    assert_eq!(tooltip_hint(ClickMode::Plain, false), None);
    assert_eq!(tooltip_hint(ClickMode::Ctrl, false), Some("ctrl+clic"));
    assert_eq!(tooltip_hint(ClickMode::Plain, true), Some("shift+clic"));
    assert_eq!(tooltip_hint(ClickMode::Ctrl, true), Some("shift+ctrl+clic"));
}
