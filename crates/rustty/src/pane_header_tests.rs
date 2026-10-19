use super::*;
use crate::cell_hit::cell_at;
use rustty_layout::Axis;

fn metrics(width: u32, height: u32) -> CellMetrics {
    CellMetrics {
        width,
        height,
        baseline: height - 4,
        underline_y: height - 2,
        underline_thickness: 1,
        strike_y: height / 2,
    }
}

fn two_pane_tab() -> Tab {
    let mut tab = Tab::new(TermId(1), 0);
    assert!(tab.split(Axis::Vertical, TermId(2), 1));
    tab
}

#[test]
fn headers_need_two_visible_panes() {
    let mut tab = Tab::new(TermId(1), 0);
    assert!(!headers_visible(&tab), "un seul panneau");
    assert!(tab.split(Axis::Vertical, TermId(2), 1));
    assert!(headers_visible(&tab), "deux panneaux");
    let focused = tab.layout.focused().expect("focus");
    tab.layout.toggle_zoom(focused);
    assert!(!headers_visible(&tab), "un panneau agrandi");
}

#[test]
fn split_header_takes_one_row_from_the_top() {
    let (header, content) = split_header(PixelRect::new(10, 30, 400, 200), 20);
    assert_eq!(header, PixelRect::new(10, 30, 400, 20));
    assert_eq!(content, PixelRect::new(10, 50, 400, 180));
}

#[test]
fn tiny_pane_keeps_a_one_row_grid() {
    let (header, content) = split_header(PixelRect::new(0, 0, 100, 19), 20);
    assert_eq!(header.height, 19);
    assert_eq!(content.height, 0);
    let (cols, rows) = crate::geometry::grid_size(content, metrics(10, 20), 4);
    assert!(cols >= 1 && rows >= 1);
}

#[test]
fn close_wins_over_title_in_header_at() {
    let m = metrics(10, 20);
    let header = header_for(TermId(7), PixelRect::new(0, 0, 400, 20), m);
    let headers = [header];
    assert_eq!(
        header_at(&headers, 5.0, 5.0),
        Some(HeaderHit::Title(TermId(7)))
    );
    let x = f64::from(header.close.x) + 1.0;
    assert_eq!(
        header_at(&headers, x, 5.0),
        Some(HeaderHit::Close(TermId(7)))
    );
    assert_eq!(header_at(&headers, 5.0, 25.0), None);
}

#[test]
fn narrow_header_has_no_close_button() {
    let m = metrics(10, 20);
    let narrow = header_for(TermId(1), PixelRect::new(0, 0, 50, 20), m);
    assert!(!narrow.has_close());
    assert_eq!(
        header_at(&[narrow], 45.0, 5.0),
        Some(HeaderHit::Title(TermId(1)))
    );
    let enough = header_for(TermId(1), PixelRect::new(0, 0, 60, 20), m);
    assert!(enough.has_close());
}

#[test]
fn cell_under_header_is_row_zero_for_zoomed_font() {
    let base = metrics(10, 20);
    let zoomed = metrics(15, 30);
    let (header, content) = split_header(PixelRect::new(0, 0, 300, 200), base.height);
    assert_eq!(header.height, 20);
    // Un pixel sous le bandeau et le padding : première rangée du terminal.
    let y = f64::from(content.y) + 4.0 + 1.0;
    assert_eq!(cell_at(content, zoomed, 4, 50.0, y).map(|c| c.1), Some(0));
}

#[test]
fn layout_headers_splits_every_pane_of_a_split_tab() {
    let tab = two_pane_tab();
    let full: Vec<_> = tab
        .layout
        .windows()
        .into_iter()
        .enumerate()
        .map(|(i, w)| (w, PixelRect::new(i as u32 * 200, 0, 198, 100)))
        .collect();
    let (headers, contents) = layout_headers(&tab, &full, metrics(10, 20));
    assert_eq!(headers.len(), 2);
    assert!(contents.iter().all(|(_, r)| r.y == 20 && r.height == 80));
    let single = Tab::new(TermId(1), 0);
    let one = [(
        single.layout.focused().expect("w"),
        PixelRect::new(0, 0, 100, 100),
    )];
    let (headers, contents) = layout_headers(&single, &one, metrics(10, 20));
    assert!(headers.is_empty());
    assert_eq!(contents[0].1.height, 100);
}
