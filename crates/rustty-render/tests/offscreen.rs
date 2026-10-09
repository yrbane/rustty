//! Rendu complet hors écran, comparé à des images de référence produites
//! avec la police embarquée : identiques sur les trois OS à l'arrondi près.

mod common;
mod support;

use rustty_config::Colors;
use rustty_render::{Frame, Offscreen, Palette, PaneFrame, PixelRect, Rgba};
use rustty_vt::Term;
use support::{PADDING, assert_matches_golden, pixel, render_term, renderer};

#[test]
fn hello_world_matches_golden() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let mut term = Term::new(14, 2, 0);
    term.input(b"Hello, \x1b[1;31mworld\x1b[0m!\r\n\x1b[4mrustty\x1b[0m \x1b[44m  \x1b[0m");
    let (px, w, h) = render_term(&ctx, &mut r, &term, true, 1.0);
    assert_matches_golden("hello_world", &px, w, h);
}

#[test]
fn box_drawing_and_powerline_match_golden() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let mut term = Term::new(8, 3, 0);
    term.input("┌──┐\u{E0B0}\u{E0B6}\u{E0B4}\r\n│漢│\r\n└──┘░▒▓█\x1b[?25l".as_bytes());
    let (px, w, h) = render_term(&ctx, &mut r, &term, true, 1.0);
    assert_matches_golden("box_drawing", &px, w, h);
}

#[test]
fn cursor_block_is_visible_and_inverts_the_glyph() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let mut term = Term::new(4, 1, 0);
    term.input(b"ab\x1b[D");
    let (px, w, _) = render_term(&ctx, &mut r, &term, true, 1.0);
    let m = r.metrics();
    let cursor_px = pixel(&px, w, PADDING + m.width + 1, PADDING + 1);
    let cursor = Palette::from_config(&Colors::default(), false)
        .cursor
        .to_u8();
    assert_eq!(cursor_px, cursor, "coin du bloc curseur");
}

#[test]
fn unknown_characters_render_as_blank() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let mut term = Term::new(3, 1, 0);
    term.input("\u{E000}\u{10FFFF}\x1b[?25l".as_bytes());
    let (px, w, h) = render_term(&ctx, &mut r, &term, true, 1.0);
    let bg = Palette::from_config(&Colors::default(), false)
        .background
        .to_u8();
    let m = r.metrics();
    for y in PADDING..PADDING + m.height {
        for x in PADDING..PADDING + 2 * m.width {
            assert_eq!(pixel(&px, w, x, y), bg, "({x},{y}) doit rester fond");
        }
    }
    let _ = h;
}

#[test]
fn background_opacity_is_written_to_alpha() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let mut term = Term::new(2, 1, 0);
    term.input(b"\x1b[?25l");
    let (px, w, _) = render_term(&ctx, &mut r, &term, true, 0.5);
    let a = pixel(&px, w, 0, 0)[3];
    assert!((127..=128).contains(&a), "alpha du fond : {a}");
}

#[test]
fn render_survives_tiny_viewport() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let mut term = Term::new(10, 3, 0);
    term.input(b"abc");
    let snap = term.snapshot();
    for (vw, vh) in [(0, 0), (1, 1), (5, 3)] {
        let target = Offscreen::new(&ctx, vw, vh);
        let frame = Frame {
            viewport: (vw, vh),
            background: Rgba::new(0.0, 0.0, 0.0, 1.0),
            panes: vec![PaneFrame {
                rect: PixelRect::new(0, 0, vw, vh),
                snapshot: &snap,
                focused: true,
            }],
            chrome: Default::default(),
        };
        r.render(&ctx, target.view(), &frame);
        let px = target.read_rgba(&ctx).unwrap();
        assert_eq!(px.len() as u32, target.size().0 * target.size().1 * 4);
    }
}

#[test]
fn tab_bar_with_close_buttons_matches_golden() {
    use rustty_render::{
        HoverTarget, TabBarStyle, TabSpec, layout_tab_bar, tab_bar_chrome, tab_bar_height,
    };
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let m = r.metrics();
    let palette = Palette::from_config(&Colors::default(), false);
    let style = TabBarStyle::from_config(&palette, &rustty_config::Tabs::default());
    let tabs = [
        TabSpec {
            title: "1: sh",
            active: true,
            accent: None,
        },
        TabSpec {
            title: "2: vim",
            active: false,
            accent: None,
        },
    ];
    let width = 30 * m.width;
    let bar_h = tab_bar_height(m, &style);
    let layout = layout_tab_bar(width, 0, &tabs, &style, m);
    let chrome = tab_bar_chrome(&layout, &tabs, &style, m, HoverTarget::CloseButton(1));
    let mut term = Term::new(30, 1, 0);
    term.input(b"$ \x1b[?25l");
    let snap = term.snapshot();
    let height = bar_h + m.height + 2 * PADDING;
    let target = Offscreen::new(&ctx, width, height);
    let frame = Frame {
        viewport: (width, height),
        background: palette.background,
        panes: vec![PaneFrame {
            rect: PixelRect::new(0, bar_h, width, height - bar_h),
            snapshot: &snap,
            focused: true,
        }],
        chrome,
    };
    r.render(&ctx, target.view(), &frame);
    let px = target.read_rgba(&ctx).unwrap();
    assert_matches_golden("tab_bar", &px, width, height);
    let hover_px = pixel(&px, width, layout.tabs[1].close.unwrap().x + 15, 10);
    assert_eq!(
        hover_px,
        style.close_hover_background.to_u8(),
        "le bouton survolé est dans sa couleur de survol"
    );
}

#[test]
fn render_onto_keeps_what_is_already_there() {
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let target = Offscreen::new(&ctx, 60, 30);
    rustty_render::clear(&ctx, target.view(), Rgba::new(1.0, 0.0, 0.0, 1.0));
    let mut term = Term::new(2, 1, 0);
    term.input(b"\x1b[?25l");
    let snap = term.snapshot();
    let frame = Frame {
        viewport: (60, 30),
        background: Rgba::new(0.0, 1.0, 0.0, 1.0),
        panes: vec![PaneFrame {
            rect: PixelRect::new(30, 0, 30, 30),
            snapshot: &snap,
            focused: true,
        }],
        chrome: Default::default(),
    };
    r.render_onto(&ctx, target.view(), &frame);
    let px = target.read_rgba(&ctx).unwrap();
    assert_eq!(
        pixel(&px, 60, 5, 5),
        [255, 0, 0, 255],
        "la moitié gauche n'est pas effacée"
    );
    assert_eq!(
        pixel(&px, 60, 45, 15),
        [255, 0, 0, 255],
        "fond par défaut du terminal : rien n'est peint, pas de vert"
    );
}
