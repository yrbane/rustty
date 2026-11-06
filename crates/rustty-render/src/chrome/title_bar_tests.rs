use super::*;
use rustty_config::TitleButton;

fn style() -> TitleBarStyle {
    TitleBarStyle::from_config(&rustty_config::TitleBar::default(), 1.0)
}

fn metrics() -> CellMetrics {
    CellMetrics {
        width: 8,
        height: 16,
        baseline: 12,
        underline_y: 14,
        underline_thickness: 1,
        strike_y: 8,
    }
}

#[test]
fn style_scales_logical_sizes_to_physical() {
    let s = TitleBarStyle::from_config(&rustty_config::TitleBar::default(), 2.0);
    assert_eq!((s.height, s.padding, s.spacing), (56, 12, 8));
    assert_eq!(s.buttons.len(), 3, "ce jalon : réduire, agrandir, fermer");
    assert_eq!(s.buttons[0].1.size, 28);
    assert_eq!(s.buttons[0].1.radius, 14);
    assert_eq!(title_bar_height(&s), 57, "hauteur plus le séparateur");
}

#[test]
fn window_controls_sit_at_the_right_in_order() {
    let l = layout_title_bar(400, &style());
    assert_eq!(l.bar, PixelRect::new(0, 0, 400, 28));
    assert_eq!(l.separator, PixelRect::new(0, 28, 400, 1));
    let names: Vec<_> = l.buttons.iter().map(|(b, _)| *b).collect();
    assert_eq!(
        names,
        [
            TitleButton::Minimize,
            TitleButton::Maximize,
            TitleButton::Close
        ]
    );
    let close = l.buttons[2].1;
    assert_eq!(close, PixelRect::new(400 - 6 - 14, 7, 14, 14));
    assert_eq!(l.buttons[1].1.x, close.x - 4 - 14);
    assert_eq!(
        l.title,
        PixelRect::new(6 + 4, 0, l.buttons[0].1.x - 4 - 10, 28)
    );
}

#[test]
fn hit_test_distinguishes_buttons_title_and_background() {
    let l = layout_title_bar(400, &style());
    assert_eq!(l.hit_test(390, 10), TitleTarget::Button(TitleButton::Close));
    assert_eq!(l.hit_test(200, 10), TitleTarget::Title);
    assert_eq!(l.hit_test(2, 2), TitleTarget::Background);
    assert_eq!(l.hit_test(200, 40), TitleTarget::None);
}

#[test]
fn narrow_window_keeps_buttons_and_drops_the_title() {
    let l = layout_title_bar(20, &style());
    assert_eq!(l.buttons.len(), 3);
    assert_eq!(l.title.width, 0);
    assert!(
        l.buttons
            .iter()
            .all(|(_, r)| r.x + r.width <= 20 || r.x == 0)
    );
}

#[test]
fn hidden_window_control_is_left_out() {
    let mut cfg = rustty_config::TitleBar::default();
    cfg.buttons.minimize.visible = false;
    let s = TitleBarStyle::from_config(&cfg, 1.0);
    let names: Vec<_> = s.buttons.iter().map(|(b, _)| *b).collect();
    assert_eq!(names, [TitleButton::Maximize, TitleButton::Close]);
}

#[test]
fn chrome_has_gloss_hover_and_centered_title() {
    let s = style();
    let l = layout_title_bar(400, &s);
    let c = title_bar_chrome(
        &l,
        &s,
        TitleBarState {
            title: "zsh — rustty",
            hover: Some(TitleButton::Close),
            maximized: false,
        },
        metrics(),
    );
    assert_eq!(c.shapes[0].rect, l.bar);
    assert_eq!(c.shapes[0].style.fill, s.background.lighten(0.10));
    assert_eq!(c.shapes[0].style.fill_bottom, s.background);
    assert_eq!(c.shapes[1].rect, PixelRect::new(0, 0, 400, 1), "liseré");
    assert_eq!(c.shapes[1].style.fill, Rgba::new(1.0, 1.0, 1.0, 0.10));
    let close = c
        .shapes
        .iter()
        .find(|sh| sh.rect == l.buttons[2].1)
        .unwrap();
    assert_eq!(close.style.fill, s.buttons[2].1.hover_background);
    assert_eq!(close.style.border, s.buttons[2].1.hover_border);
    assert_eq!(close.style.radius, 7.0);
    let minimize = c
        .shapes
        .iter()
        .find(|sh| sh.rect == l.buttons[0].1)
        .unwrap();
    assert_eq!(minimize.style.fill, s.buttons[0].1.background);
    let title = c.texts.iter().find(|t| t.text == "zsh — rustty").unwrap();
    assert_eq!(title.x, l.title.x + (l.title.width - 12 * 8) / 2);
    assert_eq!(title.y, (28 - 16) / 2);
    assert_eq!(title.color, s.title_color);
    assert!(
        c.icons
            .iter()
            .any(|i| i.ch == '✕' && i.color == s.buttons[2].1.hover_icon)
    );
    assert!(
        c.icons
            .iter()
            .any(|i| i.ch == '□' && i.color == s.buttons[1].1.icon)
    );
}

#[test]
fn long_title_is_truncated_to_the_title_zone() {
    let s = style();
    let l = layout_title_bar(120, &s);
    let long = "a".repeat(100);
    let c = title_bar_chrome(
        &l,
        &s,
        TitleBarState {
            title: &long,
            hover: None,
            maximized: false,
        },
        metrics(),
    );
    let title = c.texts.iter().find(|t| t.text.ends_with('…')).unwrap();
    assert!(title.text.chars().count() as u32 * 8 <= l.title.width);
}

#[test]
fn light_background_darkens_the_title() {
    let mut s = style();
    s.background = Rgba::new(0.95, 0.85, 0.2, 1.0);
    let l = layout_title_bar(400, &s);
    let c = title_bar_chrome(
        &l,
        &s,
        TitleBarState {
            title: "x",
            hover: None,
            maximized: false,
        },
        metrics(),
    );
    let title = c.texts.iter().find(|t| t.text == "x").unwrap();
    assert_eq!(title.color, Rgba::new(0.07, 0.07, 0.07, 1.0));
}

#[test]
fn zero_gloss_gives_a_flat_bar() {
    let mut s = style();
    s.gloss = 0.0;
    let l = layout_title_bar(100, &s);
    let c = title_bar_chrome(
        &l,
        &s,
        TitleBarState {
            title: "",
            hover: None,
            maximized: true,
        },
        metrics(),
    );
    assert_eq!(c.shapes[0].style.fill, s.background);
    assert_eq!(c.shapes[0].style.fill_bottom, s.background);
    assert_eq!(
        c.shapes[1].rect, l.separator,
        "pas de liseré : la deuxième forme est le séparateur"
    );
    assert!(
        c.icons.iter().any(|i| i.ch == '❐'),
        "agrandi : icône restaurer"
    );
    assert!(
        c.texts.iter().all(|t| !t.text.is_empty()),
        "pas de titre vide"
    );
}

#[test]
fn icons_are_centered_on_their_button() {
    let s = TitleBarStyle::from_config(&TitleBar::default(), 1.0);
    let l = layout_title_bar(400, &s);
    let c = title_bar_chrome(
        &l,
        &s,
        TitleBarState {
            title: "x",
            hover: None,
            maximized: false,
        },
        metrics(),
    );
    assert_eq!(c.icons.len(), l.buttons.len(), "une icône par pastille");
    for ((_, rect), icon) in l.buttons.iter().zip(&c.icons) {
        assert_eq!(
            (icon.x, icon.y),
            (rect.x + rect.width / 2, rect.y + rect.height / 2),
            "l'icône {} est centrée sur sa pastille, pas posée sur une cellule",
            icon.ch
        );
    }
}
