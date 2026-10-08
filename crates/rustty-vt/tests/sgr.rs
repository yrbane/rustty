//! SGR vu de l'extérieur : on écrit un caractère et on lit le style de sa cellule.

use rustty_vt::{Attrs, Color, Style, Term};

fn style_after(seq: &str) -> Style {
    let mut t = Term::new(10, 1, 0);
    t.input(seq.as_bytes());
    t.input(b"a");
    t.grid().cell(0, 0).style
}

#[test]
fn basic_attributes() {
    let a = style_after("\x1b[1;3;4;5;7;8;9m").attrs;
    for f in [
        Attrs::BOLD,
        Attrs::ITALIC,
        Attrs::UNDERLINE,
        Attrs::BLINK,
        Attrs::INVERSE,
        Attrs::HIDDEN,
        Attrs::STRIKETHROUGH,
    ] {
        assert!(a.contains(f), "{f:?} manquant");
    }
    assert_eq!(style_after("\x1b[2m").attrs, Attrs::DIM);
}

#[test]
fn reset_explicit_and_empty() {
    assert_eq!(style_after("\x1b[1;31m\x1b[0m"), Style::default());
    assert_eq!(style_after("\x1b[1;31m\x1b[m"), Style::default());
}

#[test]
fn individual_resets() {
    let a = style_after("\x1b[1;2;3;4;5;7;8;9m\x1b[22;23;24;25;27;28;29m").attrs;
    assert!(a.is_empty(), "22 retire gras et atténué");
}

#[test]
fn indexed_colors() {
    let s = style_after("\x1b[31;42m");
    assert_eq!((s.fg, s.bg), (Color::Indexed(1), Color::Indexed(2)));
    let s = style_after("\x1b[94;105m");
    assert_eq!((s.fg, s.bg), (Color::Indexed(12), Color::Indexed(13)));
    assert_eq!(style_after("\x1b[31;42m\x1b[39;49m"), Style::default());
}

#[test]
fn extended_colors_with_semicolons_and_colons() {
    let s = style_after("\x1b[38;5;200;48;2;10;20;30m");
    assert_eq!((s.fg, s.bg), (Color::Indexed(200), Color::Rgb(10, 20, 30)));
    let s = style_after("\x1b[38:2::1:2:3;48:5:7m");
    assert_eq!(
        s.fg,
        Color::Rgb(1, 2, 3),
        "forme 38:2::r:g:b avec espace colorimétrique vide"
    );
    assert_eq!(s.bg, Color::Indexed(7));
    assert_eq!(
        style_after("\x1b[38:2:4:5:6m").fg,
        Color::Rgb(4, 5, 6),
        "forme 38:2:r:g:b sans espace"
    );
}

#[test]
fn truncated_extended_color_is_ignored_but_rest_applies() {
    let s = style_after("\x1b[38;2;10m\x1b[1m");
    assert_eq!(s.fg, Color::Default);
    assert!(s.attrs.contains(Attrs::BOLD));
}

#[test]
fn underline_styles_via_subparams() {
    assert!(style_after("\x1b[4:3m").attrs.contains(Attrs::UNDERLINE));
    assert!(
        !style_after("\x1b[4m\x1b[4:0m")
            .attrs
            .contains(Attrs::UNDERLINE)
    );
}
