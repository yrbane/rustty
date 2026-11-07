//! Rendu complet hors écran, comparé à des images de référence produites
//! avec la police embarquée : identiques sur les trois OS à l'arrondi près.

mod common;
mod support;

use rustty_config::Colors;
use rustty_render::{Chrome, Frame, Offscreen, Palette, PaneFrame, PixelRect, Rgba};
use rustty_vt::Term;
use support::{PADDING, assert_matches_golden, pixel, render_term, renderer};

/// Le rectangle d'un bouton de la barre de titre, quel que soit son rang.
fn button_rect(
    layout: &rustty_render::TitleBarLayout,
    button: rustty_config::TitleButton,
) -> PixelRect {
    layout
        .buttons
        .iter()
        .find(|(b, _)| *b == button)
        .map(|(_, r)| *r)
        .unwrap_or_else(|| panic!("bouton {button:?} absent de la barre"))
}

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
                link_hover: None,
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
            link_hover: None,
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
            link_hover: None,
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

#[test]
fn pane_header_matches_golden() {
    use rustty_render::{PaneHeaderSpec, TabBarStyle, pane_header_chrome};
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let m = r.metrics();
    let palette = Palette::from_config(&Colors::default(), false);
    let style = TabBarStyle::from_config(&palette, &rustty_config::Tabs::default());
    let (width, half) = (40 * m.width, 20 * m.width);
    let height = m.height * 3;
    let headers = [
        PaneHeaderSpec {
            rect: PixelRect::new(0, 0, half, m.height),
            title: "1: sh",
            color: Rgba::new(0.20, 0.45, 0.80, 1.0),
            focused: true,
            show_close: true,
            close_hovered: true,
        },
        PaneHeaderSpec {
            rect: PixelRect::new(half, 0, half, m.height),
            title: "un titre vraiment beaucoup trop long",
            color: Rgba::new(0.85, 0.75, 0.30, 1.0),
            focused: false,
            show_close: false,
            close_hovered: false,
        },
    ];
    let chrome = pane_header_chrome(&headers, &style, m);
    let target = Offscreen::new(&ctx, width, height);
    let frame = Frame {
        viewport: (width, height),
        background: palette.background,
        panes: vec![],
        chrome,
    };
    r.render(&ctx, target.view(), &frame);
    let px = target.read_rgba(&ctx).unwrap();
    assert_matches_golden("pane_header", &px, width, height);
    let close = rustty_render::pane_header_close_rect(headers[0].rect, m);
    assert_eq!(
        pixel(&px, width, close.x + close.width / 2, 2),
        style.close_hover_background.to_u8(),
        "le ✕ survolé est dans sa couleur de survol"
    );
}

#[test]
fn styled_quad_matches_golden() {
    use rustty_render::{ChromeShape, QuadStyle};
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let (w, h) = (64, 40);
    let target = Offscreen::new(&ctx, w, h);
    let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
    let blue = Rgba::new(0.0, 0.0, 1.0, 1.0);
    let shapes = vec![
        ChromeShape {
            rect: PixelRect::new(4, 4, 24, 32),
            style: QuadStyle {
                fill: red,
                fill_bottom: blue,
                radius: 8.0,
                border_width: 2.0,
                border: Rgba::new(1.0, 1.0, 1.0, 1.0),
            },
        },
        ChromeShape {
            rect: PixelRect::new(36, 4, 24, 32),
            style: QuadStyle::flat(red),
        },
    ];
    let frame = Frame {
        viewport: (w, h),
        background: Rgba::new(0.0, 0.0, 0.0, 1.0),
        panes: vec![],
        chrome: Chrome {
            shapes,
            ..Default::default()
        },
    };
    r.render(&ctx, target.view(), &frame);
    let px = target.read_rgba(&ctx).unwrap();
    assert_matches_golden("styled_quad", &px, w, h);
    assert_eq!(
        pixel(&px, w, 4, 4),
        [0, 0, 0, 255],
        "le coin arrondi laisse voir le fond"
    );
    assert_eq!(
        pixel(&px, w, 16, 5),
        [255, 255, 255, 255],
        "bordure en haut au milieu"
    );
    assert_eq!(
        pixel(&px, w, 36, 4),
        [255, 0, 0, 255],
        "le quad plat garde son coin"
    );
    let top = pixel(&px, w, 16, 8);
    let bottom = pixel(&px, w, 16, 31);
    assert!(
        top[0] > bottom[0] && bottom[2] > top[2],
        "dégradé rouge → bleu : {top:?} / {bottom:?}"
    );
}

#[test]
fn title_bar_matches_golden() {
    use rustty_config::TitleButton;
    use rustty_render::{TitleBarState, TitleBarStyle, layout_title_bar, title_bar_chrome};
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let m = r.metrics();
    let style = TitleBarStyle::from_config(&rustty_config::TitleBar::default(), 1.0);
    let (w, h) = (320, 30);
    let layout = layout_title_bar(w, &style);
    let chrome = title_bar_chrome(
        &layout,
        &style,
        TitleBarState {
            title: "1: zsh — rustty 0.1",
            hover: Some(TitleButton::Maximize),
            maximized: false,
            background: None,
        },
        m,
    );
    let target = Offscreen::new(&ctx, w, h);
    let frame = Frame {
        viewport: (w, h),
        background: Rgba::new(0.12, 0.12, 0.18, 1.0),
        panes: vec![],
        chrome,
    };
    r.render(&ctx, target.view(), &frame);
    let px = target.read_rgba(&ctx).unwrap();
    assert_matches_golden("title_bar", &px, w, h);
    let close = button_rect(&layout, TitleButton::Close);
    assert_eq!(
        pixel(&px, w, close.x + 2, close.y + 7),
        [255, 95, 87, 255],
        "pastille fermer"
    );
    let maximize = button_rect(&layout, TitleButton::Maximize);
    assert_eq!(
        pixel(&px, w, maximize.x + 1, maximize.y + 7),
        [61, 200, 79, 255],
        "pastille agrandir, même fond au survol (entre bordure et icône)"
    );
}

/// La fenêtre entière telle que la compose `render.rs` : barre de titre,
/// barre d'onglets, deux panneaux sous leurs bandeaux. Sert de capture.
#[test]
fn window_mockup_matches_golden() {
    use rustty_config::TitleButton;
    use rustty_render::{
        HoverTarget, PaneHeaderSpec, TabBarStyle, TabSpec, TitleBarState, TitleBarStyle,
        layout_tab_bar, layout_title_bar, pane_header_chrome, tab_bar_chrome, tab_bar_height,
        title_bar_chrome, title_bar_height,
    };
    let Some(ctx) = common::gpu_or_skip() else {
        return;
    };
    let mut r = renderer(&ctx);
    let m = r.metrics();
    let palette = Palette::from_config(&Colors::default(), false);
    let blue = Rgba::new(0.20, 0.45, 0.80, 1.0);
    let title_style = TitleBarStyle::from_config(&rustty_config::TitleBar::default(), 1.0);
    let tab_style = TabBarStyle::from_config(&palette, &rustty_config::Tabs::default());
    let w = 64 * m.width;
    let title_h = title_bar_height(&title_style);
    let bar_h = tab_bar_height(m, &tab_style);
    let h = title_h + bar_h + 8 * m.height + 2 * PADDING;

    let title_layout = layout_title_bar(w, &title_style);
    let mut chrome = title_bar_chrome(
        &title_layout,
        &title_style,
        TitleBarState {
            title: "déploiement — rustty 0.1",
            hover: Some(TitleButton::Color),
            maximized: false,
            background: None,
        },
        m,
    );
    let tabs = [
        TabSpec {
            title: "1: déploiement",
            active: true,
            accent: Some(blue),
        },
        TabSpec {
            title: "2: logs",
            active: false,
            accent: None,
        },
    ];
    let tab_layout = layout_tab_bar(w, title_h, &tabs, &tab_style, m);
    let tabs_chrome = tab_bar_chrome(&tab_layout, &tabs, &tab_style, m, HoverTarget::None);
    let half = w / 2;
    let top = title_h + bar_h;
    let headers = [
        PaneHeaderSpec {
            rect: PixelRect::new(0, top, half, m.height),
            title: "1: zsh",
            color: blue,
            focused: true,
            show_close: true,
            close_hovered: false,
        },
        PaneHeaderSpec {
            rect: PixelRect::new(half, top, w - half, m.height),
            title: "2: tail -f",
            color: Rgba::new(0.85, 0.75, 0.30, 1.0),
            focused: false,
            show_close: false,
            close_hovered: false,
        },
    ];
    let headers_chrome = pane_header_chrome(&headers, &tab_style, m);
    for part in [tabs_chrome, headers_chrome] {
        chrome.quads.extend(part.quads);
        chrome.shapes.extend(part.shapes);
        chrome.texts.extend(part.texts);
        chrome.icons.extend(part.icons);
    }

    let cols = (half / m.width).saturating_sub(1) as usize;
    let mut left = Term::new(cols, 7, 0);
    left.input(
        b"$ cargo test --workspace\r\n\
          running 837 tests\r\n\
          test result: \x1b[32mok\x1b[0m. 837 passed\r\n\
          $ git status --short\r\n\
          \x1b[36mM\x1b[0m README.md\r\n\
          $ ",
    );
    let mut right = Term::new(cols, 7, 0);
    right.input(
        b"$ tail -f /var/log/deploy.log\r\n\
          \x1b[2m14:02:11\x1b[0m build \x1b[32mok\x1b[0m\r\n\
          \x1b[2m14:02:40\x1b[0m tests \x1b[32mok\x1b[0m\r\n\
          \x1b[2m14:03:05\x1b[0m \x1b[33mwarn\x1b[0m cache froid\r\n\
          \x1b[2m14:03:19\x1b[0m deploy \x1b[1;32m\xe2\x9c\x93\x1b[0m\r\n\
          \x1b[?25l",
    );
    let (left_snap, right_snap) = (left.snapshot(), right.snapshot());
    let pane_top = top + m.height;
    let target = Offscreen::new(&ctx, w, h);
    let frame = Frame {
        viewport: (w, h),
        background: palette.background,
        panes: vec![
            PaneFrame {
                rect: PixelRect::new(0, pane_top, half, h - pane_top),
                snapshot: &left_snap,
                focused: true,
                link_hover: None,
            },
            PaneFrame {
                rect: PixelRect::new(half, pane_top, w - half, h - pane_top),
                snapshot: &right_snap,
                focused: false,
                link_hover: None,
            },
        ],
        chrome,
    };
    r.render(&ctx, target.view(), &frame);
    let px = target.read_rgba(&ctx).unwrap();
    assert_matches_golden("window", &px, w, h);
    let color_button = button_rect(&title_layout, rustty_config::TitleButton::Color);
    let sum = |p: [u8; 4]| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]);
    assert!(
        sum(pixel(&px, w, color_button.x + 2, color_button.y + 2)) > sum(pixel(&px, w, w / 2, 2)),
        "le bouton couleur survolé est plus clair que la barre"
    );
    assert_eq!(
        pixel(&px, w, 0, title_style.height),
        [0, 0, 0, 255],
        "séparateur noir sous la barre"
    );
}
