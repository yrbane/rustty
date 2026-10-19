//! Tests du bandeau de panneau.

use super::*;
use rustty_config::{Colors, Tabs};

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

fn style() -> TabBarStyle {
    TabBarStyle::from_config(
        &Palette::from_config(&Colors::default(), false),
        &Tabs::default(),
    )
}

fn spec(title: &str, show_close: bool) -> PaneHeaderSpec<'_> {
    PaneHeaderSpec {
        rect: PixelRect::new(100, 40, 300, 20),
        title,
        color: Rgba::new(0.2, 0.4, 0.8, 1.0),
        focused: true,
        show_close,
        close_hovered: false,
    }
}

#[test]
fn close_rect_is_inside_and_right_aligned() {
    let rect = PixelRect::new(100, 40, 300, 20);
    let close = pane_header_close_rect(rect, metrics());
    assert_eq!(close.x + close.width, rect.x + rect.width);
    assert_eq!((close.y, close.height), (rect.y, 20));
    assert_eq!(close.width, CLOSE_CELLS * 10);
    assert!(close.x >= rect.x);
}

#[test]
fn close_rect_stays_inside_a_tiny_header() {
    let rect = PixelRect::new(5, 0, 20, 20);
    let close = pane_header_close_rect(rect, metrics());
    assert!(close.x >= rect.x);
}

#[test]
fn long_title_is_truncated_before_the_close_button() {
    let long = "un titre vraiment beaucoup trop long pour tenir dans ce bandeau étroit";
    let s = spec(long, true);
    let close = pane_header_close_rect(s.rect, metrics());
    let chrome = pane_header_chrome(&[s], &style(), metrics());
    let title = &chrome.texts[0];
    assert!(title.text.ends_with('…'), "{}", title.text);
    let end = title.x + title_cells(&title.text) * 10;
    assert!(
        end <= close.x,
        "le titre déborde sur le ✕ : {end} > {}",
        close.x
    );
}

#[test]
fn no_close_quad_unless_show_close() {
    let st = style();
    let without = pane_header_chrome(&[spec("sh", false)], &st, metrics());
    assert_eq!(without.quads.len(), 1, "seulement le fond");
    assert!(without.texts.iter().all(|t| t.text != "✕"));
    let with = pane_header_chrome(&[spec("sh", true)], &st, metrics());
    assert_eq!(with.quads.len(), 2);
    assert!(with.texts.iter().any(|t| t.text == "✕"));
}

#[test]
fn hovered_close_uses_hover_colors() {
    let st = style();
    let mut s = spec("sh", true);
    s.close_hovered = true;
    let chrome = pane_header_chrome(&[s], &st, metrics());
    assert_eq!(chrome.quads[1].color, st.close_hover_background);
}

#[test]
fn text_is_readable_and_unfocused_header_is_dimmed() {
    let st = style();
    let mut s = spec("sh", false);
    s.color = Rgba::new(1.0, 1.0, 0.9, 1.0);
    let chrome = pane_header_chrome(&[s], &st, metrics());
    assert_eq!(chrome.quads[0].color, s.color);
    assert_eq!(chrome.texts[0].color, st.dark_text);
    s.focused = false;
    let chrome = pane_header_chrome(&[s], &st, metrics());
    assert_eq!(chrome.quads[0].color, s.color.dim(UNFOCUSED_DIM));
}
