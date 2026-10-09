//! Tests de la grille de cellules.

use super::*;
use rustty_config::Colors;
use rustty_vt::Term;

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

fn pane_of<'a>(snapshot: &'a rustty_vt::Snapshot, focused: bool) -> PaneFrame<'a> {
    PaneFrame {
        rect: PixelRect::new(100, 50, 200, 100),
        snapshot,
        focused,
    }
}

#[test]
fn geometry_accounts_for_padding_and_cell_size() {
    let g = grid_geometry(PixelRect::new(100, 50, 100, 50), metrics(), 4);
    assert_eq!((g.origin_x, g.origin_y, g.cols, g.rows), (104, 54, 9, 2));
}

#[test]
fn grid_geometry_survives_tiny_rects() {
    for rect in [
        PixelRect::new(0, 0, 0, 0),
        PixelRect::new(0, 0, 1, 1),
        PixelRect::new(5, 5, 7, 30),
        PixelRect::new(0, 0, 9, 19),
    ] {
        let g = grid_geometry(rect, metrics(), 4);
        assert!(
            g.cols == 0 || g.rows == 0 || (g.cols >= 1 && g.rows >= 1),
            "{rect:?} → {g:?}"
        );
    }
    assert_eq!(
        grid_geometry(PixelRect::new(0, 0, 9, 19), metrics(), 0).cols,
        0
    );
    assert_eq!(
        grid_geometry(PixelRect::new(0, 0, 10, 20), metrics(), 0).cols,
        1
    );
}

#[test]
fn backgrounds_are_merged_into_runs_and_default_is_skipped() {
    let mut t = Term::new(10, 1, 0);
    t.input(b"\x1b[44mab\x1b[0mc\x1b[41md");
    let snap = t.snapshot();
    let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
    let runs: Vec<_> = out
        .backgrounds
        .iter()
        .filter(|q| q.color != palette().cursor.to_array())
        .collect();
    assert_eq!(runs.len(), 2, "{:?}", out.backgrounds);
    assert_eq!(runs[0].pos, [100.0, 50.0]);
    assert_eq!(runs[0].size, [20.0, 20.0], "a et b fusionnés");
    assert_eq!(runs[0].color, palette().ansi[4].to_array());
    assert_eq!(runs[1].pos, [130.0, 50.0]);
}

#[test]
fn glyph_requests_skip_blanks_and_wide_continuations() {
    let mut t = Term::new(10, 1, 0);
    t.input("a b\u{1b}[1m漢".as_bytes());
    let snap = t.snapshot();
    let out = pane_instances(&pane_of(&snap, false), metrics(), &palette(), 0);
    let chars: Vec<(char, f32, bool, Variant)> = out
        .glyphs
        .iter()
        .map(|g| (g.ch, g.x, g.wide, g.variant))
        .collect();
    assert_eq!(
        chars,
        vec![
            ('a', 100.0, false, Variant::Regular),
            ('b', 120.0, false, Variant::Regular),
            ('漢', 130.0, true, Variant::Bold)
        ]
    );
    assert_eq!(out.glyphs[0].y, 50.0);
    assert_eq!(out.glyphs[0].color, palette().foreground);
}

#[test]
fn focused_block_cursor_inverts_the_cell() {
    let mut t = Term::new(5, 1, 0);
    t.input(b"ab\x1b[D");
    let snap = t.snapshot();
    let p = palette();
    let out = pane_instances(&pane_of(&snap, true), metrics(), &p, 0);
    let cursor_quad = out.backgrounds.last().unwrap();
    assert_eq!(
        (cursor_quad.pos, cursor_quad.size),
        ([110.0, 50.0], [10.0, 20.0])
    );
    assert_eq!(cursor_quad.color, p.cursor.to_array());
    let b = out.glyphs.iter().find(|g| g.ch == 'b').unwrap();
    assert_eq!(
        b.color, p.background,
        "le glyphe sous le curseur prend la couleur du fond"
    );
    assert!(out.decorations.is_empty());
}

#[test]
fn unfocused_cursor_is_a_hollow_block() {
    let mut t = Term::new(5, 1, 0);
    t.input(b"a");
    let snap = t.snapshot();
    let out = pane_instances(&pane_of(&snap, false), metrics(), &palette(), 0);
    assert_eq!(out.decorations.len(), 4, "quatre bords d'un pixel");
    assert!(
        out.decorations
            .iter()
            .all(|q| q.color == palette().cursor.to_array())
    );
    assert!(out.backgrounds.is_empty());
}

#[test]
fn beam_and_underline_cursor_shapes() {
    let mut t = Term::new(5, 1, 0);
    t.input(b"\x1b[6 q");
    let snap = t.snapshot();
    let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
    assert_eq!(out.decorations.len(), 1);
    assert_eq!(
        (out.decorations[0].pos, out.decorations[0].size),
        ([100.0, 50.0], [2.0, 20.0])
    );
    t.input(b"\x1b[4 q");
    let snap = t.snapshot();
    let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
    assert_eq!(
        (out.decorations[0].pos, out.decorations[0].size),
        ([100.0, 68.0], [10.0, 2.0])
    );
}

#[test]
fn hidden_cursor_draws_nothing() {
    let mut t = Term::new(5, 1, 0);
    t.input(b"\x1b[?25l");
    let snap = t.snapshot();
    let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
    assert!(out.backgrounds.is_empty() && out.decorations.is_empty());
}

#[test]
fn underline_and_strikethrough_decorations() {
    let mut t = Term::new(5, 1, 0);
    t.input(b"\x1b[4ma\x1b[0m\x1b[9mb\x1b[?25l");
    let snap = t.snapshot();
    let out = pane_instances(&pane_of(&snap, true), metrics(), &palette(), 0);
    assert_eq!(out.decorations.len(), 2);
    assert_eq!(
        (out.decorations[0].pos, out.decorations[0].size),
        ([100.0, 68.0], [10.0, 1.0]),
        "soulignement"
    );
    assert_eq!(
        (out.decorations[1].pos, out.decorations[1].size),
        ([110.0, 60.0], [10.0, 1.0]),
        "barré"
    );
}

#[test]
fn cells_outside_the_geometry_are_clipped() {
    let mut t = Term::new(40, 10, 0);
    t.input(b"\x1b[1;1Hy\x1b[10;40Hx");
    let snap = t.snapshot();
    let pane = PaneFrame {
        rect: PixelRect::new(0, 0, 50, 40),
        snapshot: &snap,
        focused: false,
    };
    let out = pane_instances(&pane, metrics(), &palette(), 0);
    assert_eq!(
        out.glyphs.iter().map(|g| g.ch).collect::<Vec<_>>(),
        vec!['y'],
        "5 colonnes × 2 lignes visibles"
    );
    assert!(
        out.decorations.is_empty(),
        "le curseur hors zone n'est pas dessiné"
    );
}
