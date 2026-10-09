//! Tests de la barre d'onglets.

use super::*;

#[test]
fn a_configured_background_without_text_color_gets_readable_text() {
    let colors = TabColors {
        active_background: Some(Rgb::new(0xf0, 0xf0, 0xf0)),
        inactive_background: Some(Rgb::new(0x10, 0x10, 0x10)),
        ..TabColors::default()
    };
    let st = style_with(Tabs {
        colors,
        ..Tabs::default()
    });
    assert_eq!(
        st.active_foreground, st.dark_text,
        "texte sombre sur fond clair"
    );
    assert_eq!(
        st.inactive_foreground, st.light_text,
        "texte clair sur fond sombre"
    );
}

#[test]
fn light_and_dark_text_follow_luminance_even_on_a_light_theme() {
    let light_theme = Colors {
        foreground: Rgb::new(0x20, 0x20, 0x20),
        background: Rgb::new(0xfa, 0xfa, 0xfa),
        ..Colors::default()
    };
    let st = TabBarStyle::from_config(&Palette::from_config(&light_theme, false), &Tabs::default());
    assert!(st.light_text.luminance() > st.dark_text.luminance());
}
use rustty_config::{Colors, Rgb, TabColors, Tabs};

fn metrics() -> CellMetrics {
    CellMetrics {
        width: 10,
        height: 20,
        baseline: 16,
        underline_y: 18,
        underline_thickness: 1,
        strike_y: 10,
    }
}

fn palette() -> Palette {
    Palette::from_config(&Colors::default(), false)
}

fn style_with(tabs: Tabs) -> TabBarStyle {
    TabBarStyle::from_config(&palette(), &tabs)
}

fn style(show_close: bool) -> TabBarStyle {
    style_with(Tabs {
        close_button: show_close,
        ..Tabs::default()
    })
}

fn tabs() -> Vec<TabSpec<'static>> {
    vec![
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
    ]
}

#[test]
fn default_style_keeps_the_previous_geometry() {
    let l = layout_tab_bar(400, 0, &tabs(), &style(true), metrics());
    assert_eq!(l.tabs[0].rect, PixelRect::new(0, 0, 110, 24));
    assert_eq!(l.tabs[0].close.unwrap().x, 60);
    assert_eq!(l.new_tab.x, 230);
}

#[test]
fn horizontal_padding_widens_tabs() {
    let st = style_with(Tabs {
        padding_horizontal: 3,
        ..Tabs::default()
    });
    let l = layout_tab_bar(400, 0, &tabs(), &st, metrics());
    assert_eq!(l.tabs[0].rect.width, 150, "3 + 5 + 4 + 3 cellules");
    assert_eq!(l.tabs[0].close.unwrap().x, 80);
    let c = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::None);
    let title = c.texts.iter().find(|t| t.text == "1: sh").unwrap();
    assert_eq!(title.x, 30);
}

#[test]
fn spacing_separates_tabs() {
    let st = style_with(Tabs {
        spacing: 4,
        ..Tabs::default()
    });
    let l = layout_tab_bar(400, 0, &tabs(), &st, metrics());
    assert_eq!(l.tabs[1].rect.x, 114);
    assert_eq!(l.new_tab.x, 114 + 120 + 4);
    assert_eq!(
        l.hit_test(112, 10),
        HoverTarget::None,
        "l'espace entre onglets n'est pas cliquable"
    );
}

#[test]
fn vertical_padding_sets_the_bar_height() {
    let st = style_with(Tabs {
        padding_vertical: 6,
        ..Tabs::default()
    });
    assert_eq!(tab_bar_height(metrics(), &st), 32);
    let l = layout_tab_bar(400, 0, &tabs(), &st, metrics());
    assert_eq!(l.bar.height, 32);
    assert_eq!(l.tabs[0].close.unwrap().y, 6);
    let c = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::None);
    assert_eq!(c.texts.iter().find(|t| t.text == "1: sh").unwrap().y, 6);
}

#[test]
fn configured_colors_override_the_palette() {
    let colors = TabColors {
        bar_background: Some(Rgb::new(1, 2, 3)),
        active_background: Some(Rgb::new(4, 5, 6)),
        active_foreground: Some(Rgb::new(7, 8, 9)),
        inactive_background: Some(Rgb::new(10, 11, 12)),
        inactive_foreground: Some(Rgb::new(13, 14, 15)),
        random: false,
    };
    let st = style_with(Tabs {
        colors,
        ..Tabs::default()
    });
    assert_eq!(st.background, Rgba::from_rgb(Rgb::new(1, 2, 3)));
    assert_eq!(st.active_background, Rgba::from_rgb(Rgb::new(4, 5, 6)));
    assert_eq!(st.active_foreground, Rgba::from_rgb(Rgb::new(7, 8, 9)));
    assert_eq!(st.inactive_background, Rgba::from_rgb(Rgb::new(10, 11, 12)));
    assert_eq!(st.inactive_foreground, Rgba::from_rgb(Rgb::new(13, 14, 15)));
    let default = style(true);
    assert_eq!(default.active_background, palette().background);
}

#[test]
fn accent_colors_the_tab_and_dims_inactive_ones() {
    let st = style(true);
    let accent = Rgba::from_rgb(Rgb::new(0x89, 0xb4, 0xfa));
    let specs = [
        TabSpec {
            title: "a",
            active: true,
            accent: Some(accent),
        },
        TabSpec {
            title: "b",
            active: false,
            accent: Some(accent),
        },
    ];
    let l = layout_tab_bar(400, 0, &specs, &st, metrics());
    let c = tab_bar_chrome(&l, &specs, &st, metrics(), HoverTarget::None);
    let bg = |i: usize| {
        c.quads
            .iter()
            .find(|q| q.rect == l.tabs[i].rect)
            .unwrap()
            .color
    };
    assert_eq!(bg(0), accent);
    assert_eq!(bg(1), accent.dim(INACTIVE_ACCENT_DIM));
}

#[test]
fn text_contrasts_with_the_tab_color() {
    let st = style(true);
    let light = Rgba::from_rgb(Rgb::new(0xf9, 0xe2, 0xaf));
    let dark = Rgba::from_rgb(Rgb::new(0x31, 0x32, 0x44));
    let specs = [
        TabSpec {
            title: "clair",
            active: true,
            accent: Some(light),
        },
        TabSpec {
            title: "sombre",
            active: true,
            accent: Some(dark),
        },
    ];
    let l = layout_tab_bar(400, 0, &specs, &st, metrics());
    let c = tab_bar_chrome(&l, &specs, &st, metrics(), HoverTarget::None);
    let fg = |t: &str| c.texts.iter().find(|x| x.text == t).unwrap().color;
    assert_eq!(fg("clair"), st.dark_text);
    assert_eq!(fg("sombre"), st.light_text);
}

#[test]
fn huge_paddings_never_overflow() {
    let st = style_with(Tabs {
        padding_horizontal: 8,
        padding_vertical: 32,
        spacing: 64,
        ..Tabs::default()
    });
    let l = layout_tab_bar(100, 0, &tabs(), &st, metrics());
    assert!(l.tabs.is_empty());
    let c = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::None);
    assert_eq!(c.quads.len(), 1, "le fond de barre seulement");
    let tiny = layout_tab_bar(0, 0, &tabs(), &st, metrics());
    assert!(tiny.tabs.is_empty());
}

#[test]
fn height_is_a_cell_plus_padding() {
    assert_eq!(tab_bar_height(metrics(), &style(true)), 24);
}

#[test]
fn tabs_are_laid_out_left_to_right_with_close_buttons() {
    let l = layout_tab_bar(400, 0, &tabs(), &style(true), metrics());
    assert_eq!(l.bar, PixelRect::new(0, 0, 400, 24));
    assert_eq!(l.tabs.len(), 2);
    // « 1: sh » = 5 colonnes + 1 + 1 + 4 (bouton) = 11 cellules = 110 px.
    assert_eq!(l.tabs[0].rect, PixelRect::new(0, 0, 110, 24));
    assert_eq!(
        l.tabs[0].close,
        Some(PixelRect::new(60, 2, 40, 20)),
        "espace, demi-disque, ✕, demi-disque"
    );
    assert_eq!(l.tabs[1].rect.x, 110);
    assert_eq!(l.tabs[1].rect.width, 120);
    assert_eq!(l.new_tab, PixelRect::new(230, 0, 30, 24));
}

#[test]
fn without_close_button_tabs_are_narrower() {
    let l = layout_tab_bar(400, 0, &tabs(), &style(false), metrics());
    assert_eq!(l.tabs[0].rect.width, 70);
    assert!(l.tabs[0].close.is_none());
}

#[test]
fn tabs_that_do_not_fit_are_dropped() {
    let l = layout_tab_bar(150, 0, &tabs(), &style(true), metrics());
    assert_eq!(
        l.tabs.len(),
        1,
        "le second onglet (120 px) ne tient pas après 110 px"
    );
    assert_eq!(l.new_tab.width, 30);
    let tiny = layout_tab_bar(20, 0, &tabs(), &style(true), metrics());
    assert!(tiny.tabs.is_empty());
    assert_eq!(tiny.new_tab.width, 0, "pas de place pour « + »");
}

#[test]
fn hit_test_prefers_close_button_then_tab_then_new_tab() {
    let l = layout_tab_bar(400, 0, &tabs(), &style(true), metrics());
    assert_eq!(l.hit_test(5, 5), HoverTarget::Tab(0));
    assert_eq!(l.hit_test(75, 10), HoverTarget::CloseButton(0));
    assert_eq!(l.hit_test(115, 10), HoverTarget::Tab(1));
    assert_eq!(l.hit_test(240, 10), HoverTarget::NewTabButton);
    assert_eq!(l.hit_test(300, 10), HoverTarget::None);
    assert_eq!(l.hit_test(5, 30), HoverTarget::None, "sous la barre");
}

#[test]
fn chrome_colors_follow_active_state_and_hover() {
    let st = style(true);
    let l = layout_tab_bar(400, 0, &tabs(), &st, metrics());
    let normal = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::None);
    assert_eq!(
        normal.quads[0].color, st.background,
        "fond de barre d'abord"
    );
    let active_bg = normal
        .quads
        .iter()
        .find(|q| q.rect == l.tabs[0].rect)
        .unwrap();
    assert_eq!(active_bg.color, st.active_background);
    let inactive_bg = normal
        .quads
        .iter()
        .find(|q| q.rect == l.tabs[1].rect)
        .unwrap();
    assert_eq!(inactive_bg.color, st.inactive_background);
    let close0 = normal
        .quads
        .iter()
        .find(|q| q.rect.x == 80 && q.rect.width == 10)
        .unwrap();
    assert_eq!(close0.color, st.close_background);
    let x_text = normal
        .texts
        .iter()
        .find(|t| t.text == "✕" && t.x == 80)
        .unwrap();
    assert_eq!(x_text.color, st.close_foreground);
    let discs: Vec<_> = normal
        .texts
        .iter()
        .filter(|t| t.text == "\u{E0B6}" || t.text == "\u{E0B4}")
        .collect();
    assert_eq!(discs.len(), 4, "deux demi-disques par onglet");
    assert!(discs.iter().all(|t| t.color == st.close_background));
    let title = normal.texts.iter().find(|t| t.text == "1: sh").unwrap();
    assert_eq!(
        (title.x, title.y, title.color),
        (10, 2, st.active_foreground)
    );
    let plus = normal.texts.iter().find(|t| t.text == "+").unwrap();
    assert_eq!(plus.x, 240);

    let hovered = tab_bar_chrome(&l, &tabs(), &st, metrics(), HoverTarget::CloseButton(0));
    let close0 = hovered
        .quads
        .iter()
        .find(|q| q.rect.x == 80 && q.rect.width == 10)
        .unwrap();
    assert_eq!(close0.color, st.close_hover_background);
    let x_text = hovered
        .texts
        .iter()
        .find(|t| t.text == "✕" && t.x == 80)
        .unwrap();
    assert_eq!(x_text.color, st.close_hover_foreground);
    let close1 = hovered
        .quads
        .iter()
        .find(|q| q.rect.x == 200 && q.rect.width == 10)
        .unwrap();
    assert_eq!(
        close1.color, st.close_background,
        "l'autre bouton garde ses couleurs"
    );
}

#[test]
fn long_titles_are_truncated_with_an_ellipsis() {
    let long = [TabSpec {
        title: "un titre vraiment beaucoup trop long pour la barre",
        active: true,
        accent: None,
    }];
    let l = layout_tab_bar(200, 0, &long, &style(false), metrics());
    assert_eq!(l.tabs.len(), 1);
    assert!(l.tabs[0].rect.width <= 200);
    let chrome = tab_bar_chrome(&l, &long, &style(false), metrics(), HoverTarget::None);
    let title = chrome
        .texts
        .iter()
        .find(|t| t.text.ends_with('…'))
        .expect("titre tronqué");
    assert!(title.text.chars().count() * 10 <= 180, "{}", title.text);
}

#[test]
fn last_tab_is_truncated_not_dropped() {
    // 1er onglet : 110 px ; il reste 90 px = 9 cellules pour le second,
    // dont 6 de marges et de bouton : 3 cellules de titre.
    let l = layout_tab_bar(200, 0, &tabs(), &style(true), metrics());
    assert_eq!(l.tabs.len(), 2, "le second onglet est tronqué, pas omis");
    let second = l.tabs[1].rect;
    assert!(second.x + second.width <= 200);
    let chrome = tab_bar_chrome(&l, &tabs(), &style(true), metrics(), HoverTarget::None);
    assert!(chrome.texts.iter().any(|t| t.text.ends_with('…')));
    // Moins de 3 cellules de titre : omis.
    let l = layout_tab_bar(190, 0, &tabs(), &style(true), metrics());
    assert_eq!(l.tabs.len(), 1);
}
