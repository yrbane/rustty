//! Tests du placement, du test de clic et du dessin du menu.

use super::super::chrome::truncate_to_cells;
use super::super::menu_chrome;
use super::*;
use crate::context_menu::MenuItem;
use crate::tab::TermId;
use rustty_config::{Colors, KeyMap};
use rustty_render::Palette;

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

fn menu(x: u32, y: u32) -> ContextMenu {
    let mut m = ContextMenu::pane(TermId(1), x, y, true, false, &KeyMap::empty());
    m.shortcuts = vec![None; m.entries.len()];
    m
}

fn item_index(item: MenuItem) -> usize {
    menu(0, 0)
        .entries
        .iter()
        .position(|e| *e == MenuEntry::Item(item))
        .unwrap()
}

#[test]
fn clicks_inside_the_menu_never_close_it_by_accident() {
    let mut m = menu(100, 50);
    m.can_copy = false;
    let l = layout(&m, (800, 600), metrics());
    let sep = m
        .entries
        .iter()
        .position(|e| *e == MenuEntry::Separator)
        .unwrap();
    let r = l.rows[sep].1;
    let mid = |r: PixelRect| (f64::from(r.x + 5), f64::from(r.y + r.height / 2));
    let (x, y) = mid(r);
    assert_eq!(l.click(&m, true, x, y), MenuClick::Keep, "séparateur");
    let (x, y) = mid(l.rows[item_index(MenuItem::Copy)].1);
    assert_eq!(l.click(&m, true, x, y), MenuClick::Keep, "entrée grisée");
    assert_eq!(
        l.click(&m, true, f64::from(l.rect.x + 2), f64::from(l.rect.y + 1)),
        MenuClick::Keep,
        "marge"
    );
    let close = item_index(MenuItem::ClosePane);
    let (x, y) = mid(l.rows[close].1);
    assert_eq!(l.click(&m, true, x, y), MenuClick::Run(close));
    assert_eq!(
        l.click(&m, false, x, y),
        MenuClick::Keep,
        "un clic droit sur le menu ne fait rien"
    );
    assert_eq!(
        l.click(&m, true, 5.0, 5.0),
        MenuClick::Close,
        "hors du menu"
    );
}

#[test]
fn menu_opens_at_the_click() {
    let l = layout(&menu(100, 50), (800, 600), metrics());
    assert_eq!((l.rect.x, l.rect.y), (100, 50));
    assert_eq!(l.rows.len(), menu(0, 0).entries.len());
    assert!(
        l.rows
            .windows(2)
            .all(|w| w[0].1.y + w[0].1.height == w[1].1.y),
        "rangées jointives"
    );
    assert_eq!(
        l.rows.last().unwrap().1.y + l.rows.last().unwrap().1.height + MENU_PADDING,
        l.rect.y + l.rect.height
    );
}

#[test]
fn menu_stays_inside_the_window() {
    let l = layout(&menu(790, 590), (800, 600), metrics());
    assert!(
        l.rect.x + l.rect.width <= 800 && l.rect.y + l.rect.height <= 600,
        "{:?}",
        l.rect
    );
    let tiny = layout(&menu(5, 5), (40, 30), metrics());
    assert_eq!(
        (tiny.rect.x, tiny.rect.y),
        (0, 0),
        "plus grand que la fenêtre : collé en haut à gauche"
    );
}

#[test]
fn hit_finds_items_by_row() {
    let m = menu(100, 50);
    let l = layout(&m, (800, 600), metrics());
    let close = item_index(MenuItem::ClosePane);
    let row = l.rows[close].1;
    assert_eq!(
        l.hit(&m, f64::from(row.x + 5), f64::from(row.y + 2)),
        Some(close)
    );
    assert_eq!(l.hit(&m, 10.0, 10.0), None, "hors du menu");
}

#[test]
fn separators_and_disabled_items_are_not_clickable() {
    let mut m = menu(100, 50);
    m.can_copy = false;
    let l = layout(&m, (800, 600), metrics());
    let sep = m
        .entries
        .iter()
        .position(|e| *e == MenuEntry::Separator)
        .unwrap();
    let r = l.rows[sep].1;
    assert_eq!(
        l.hit(&m, f64::from(r.x + 5), f64::from(r.y + r.height / 2)),
        None
    );
    let copy = l.rows[item_index(MenuItem::Copy)].1;
    assert_eq!(
        l.hit(&m, f64::from(copy.x + 5), f64::from(copy.y + 2)),
        None,
        "copier grisé sans sélection"
    );
}

#[test]
fn chrome_highlights_the_hovered_item_and_greys_disabled_ones() {
    let palette = Palette::from_config(&Colors::default(), false);
    let mut m = menu(100, 50);
    m.can_copy = false;
    let close = item_index(MenuItem::ClosePane);
    m.hovered = Some(close);
    m.shortcuts[close] = Some("ctrl+shift+w".into());
    let l = layout(&m, (800, 600), metrics());
    let c = menu_chrome(&m, &l, &palette, metrics());
    assert!(
        c.quads
            .iter()
            .any(|q| q.rect == l.rows[close].1 && q.color == palette.selection),
        "survol"
    );
    let copy_text = c.texts.iter().find(|t| t.text == "Copier").unwrap();
    assert_eq!(copy_text.color, palette.ansi[8], "grisé");
    let close_text = c
        .texts
        .iter()
        .find(|t| t.text == "Fermer le panneau")
        .unwrap();
    assert_eq!(close_text.color, palette.foreground);
    let hint = c.texts.iter().find(|t| t.text == "ctrl+shift+w").unwrap();
    assert!(hint.x > close_text.x, "raccourci à droite du libellé");
    assert!(
        hint.x + 12 * 10 <= l.rect.x + l.rect.width,
        "raccourci dans le menu"
    );
}

#[test]
fn contains_is_true_inside_only() {
    let l = layout(&menu(100, 50), (800, 600), metrics());
    let r = l.rect;
    assert!(l.contains(f64::from(r.x), f64::from(r.y)));
    assert!(l.contains(f64::from(r.x + r.width - 1), f64::from(r.y + r.height - 1)));
    assert!(!l.contains(f64::from(r.x + r.width), f64::from(r.y)));
    assert!(!l.contains(f64::from(r.x), f64::from(r.y + r.height)));
    assert!(!l.contains(f64::from(r.x) - 1.0, f64::from(r.y)));
}

#[test]
fn menu_never_exceeds_a_narrow_window() {
    let mut m = menu(5, 5);
    m.shortcuts[item_index(MenuItem::ClosePane)] = Some("ctrl+shift+w".into());
    let palette = Palette::from_config(&Colors::default(), false);
    let l = layout(&m, (150, 600), metrics());
    assert!(l.rect.x + l.rect.width <= 150, "{:?}", l.rect);
    let c = menu_chrome(&m, &l, &palette, metrics());
    for t in &c.texts {
        let end = t.x + t.text.width() as u32 * 10;
        assert!(end <= l.rect.x + l.rect.width, "{} dépasse", t.text);
        assert!(t.x >= l.rect.x, "{} déborde à gauche", t.text);
    }
}

#[test]
fn labels_are_truncated_with_an_ellipsis() {
    assert_eq!(truncate_to_cells("Copier", 10), "Copier");
    assert_eq!(truncate_to_cells("Copier", 6), "Copier");
    assert_eq!(truncate_to_cells("Copier", 4), "Cop…");
    assert_eq!(truncate_to_cells("Copier", 1), "…");
    assert_eq!(truncate_to_cells("Copier", 0), "");
    assert_eq!(truncate_to_cells("日本語", 4), "日…");
}

#[test]
fn palette_menu_places_fourteen_rows_and_hits_a_color() {
    let m = ContextMenu::palette(0, 100, 20);
    let l = layout(&m, (800, 600), metrics());
    assert_eq!(l.rows.len(), 14);
    let r = l.rows[3].1;
    assert_eq!(
        l.hit(&m, f64::from(r.x + 5), f64::from(r.y + 2)),
        Some(3),
        "une couleur est cliquable"
    );
    let sep = l.rows[12].1;
    assert_eq!(l.hit(&m, f64::from(sep.x + 5), f64::from(sep.y + 2)), None);
    let last = l.rows[13].1;
    assert_eq!(
        l.click(&m, true, f64::from(last.x + 5), f64::from(last.y + 2)),
        MenuClick::Run(13)
    );
}

#[test]
fn chrome_draws_a_swatch_before_color_labels_only() {
    let palette = Palette::from_config(&Colors::default(), false);
    let m = ContextMenu::palette(0, 100, 20);
    let l = layout(&m, (800, 600), metrics());
    let c = menu_chrome(&m, &l, &palette, metrics());
    let row = l.rows[0].1;
    let swatch = c
        .quads
        .iter()
        .find(|q| q.color == palette.ansi[1] && q.rect.y > row.y - 1)
        .expect("pastille");
    assert_eq!(swatch.rect.width, swatch.rect.height, "carrée");
    let label = c.texts.iter().find(|t| t.text == "Rouge").unwrap();
    assert!(
        label.x >= swatch.rect.x + swatch.rect.width,
        "libellé après la pastille"
    );
    let default = c
        .texts
        .iter()
        .find(|t| t.text == "Couleur par défaut")
        .unwrap();
    assert_eq!(
        default.x,
        l.rows[13].1.x + 10,
        "pas de pastille sur le défaut"
    );
    for t in &c.texts {
        assert!(t.x + t.text.width() as u32 * 10 <= l.rect.x + l.rect.width);
    }
}

#[test]
fn each_swatch_shows_the_colour_the_tab_gets() {
    let palette = Palette::from_config(&Colors::default(), false);
    let m = ContextMenu::palette(0, 100, 20);
    let l = layout(&m, (800, 600), metrics());
    let c = menu_chrome(&m, &l, &palette, metrics());
    let mut seen = 0;
    for (i, row) in l.rows.iter().map(|(i, r)| (*i, r)) {
        let MenuEntry::Item(MenuItem::Color(slot)) = &m.entries[i] else {
            continue;
        };
        let mut tab = crate::tab::Tab::new(TermId(1), 0);
        tab.color = Some(*slot);
        let applied =
            crate::appearance::tab_accents(&rustty_config::Tabs::default(), &palette, &[tab]);
        let swatch = c
            .quads
            .iter()
            .find(|q| q.rect.x == row.x + 10 && q.rect.y >= row.y && q.rect.y < row.y + row.height)
            .unwrap_or_else(|| panic!("pastille de la rangée {i}"));
        assert_eq!(Some(swatch.color), applied[0], "rangée {i}, ANSI {slot}");
        seen += 1;
    }
    assert_eq!(seen, 12);
}
