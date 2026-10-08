//! Du `Snapshot` aux instances : géométrie des cellules, fonds fusionnés,
//! glyphes à demander, décorations et curseur. Pur, sans GPU.

use rustty_vt::{Attrs, Cell, CursorShape, Snapshot};

use crate::color::{Palette, Rgba};
use crate::font::{CellMetrics, Variant};
use crate::frame::{PaneFrame, PixelRect};
use crate::pipeline::quad::QuadInstance;

/// Largeur de la barre du curseur « beam » et hauteur du curseur souligné.
pub const CURSOR_BAR_WIDTH: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridGeometry {
    pub origin_x: u32,
    pub origin_y: u32,
    pub cols: usize,
    pub rows: usize,
}

/// Combien de cellules tiennent dans `rect` une fois `padding` retiré de
/// chaque côté. Zéro si rien ne tient, jamais de panique.
pub fn grid_geometry(rect: PixelRect, metrics: CellMetrics, padding: u32) -> GridGeometry {
    let inner_w = rect.width.saturating_sub(padding * 2);
    let inner_h = rect.height.saturating_sub(padding * 2);
    GridGeometry {
        origin_x: rect.x + padding,
        origin_y: rect.y + padding,
        cols: (inner_w / metrics.width.max(1)) as usize,
        rows: (inner_h / metrics.height.max(1)) as usize,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlyphRequest {
    /// Coin haut-gauche de la cellule, en pixels.
    pub x: f32,
    pub y: f32,
    pub ch: char,
    pub variant: Variant,
    pub color: Rgba,
    pub wide: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaneInstances {
    pub backgrounds: Vec<QuadInstance>,
    pub glyphs: Vec<GlyphRequest>,
    pub decorations: Vec<QuadInstance>,
}

/// Position du curseur dans la grille visible, si elle y tient.
fn visible_cursor(snapshot: &Snapshot, geometry: &GridGeometry) -> Option<(usize, usize)> {
    let c = snapshot.cursor.as_ref()?;
    (c.col < geometry.cols && c.row < geometry.rows).then_some((c.col, c.row))
}

fn is_blank(cell: &Cell) -> bool {
    cell.c == ' ' || cell.c == '\0' || cell.is_wide_continuation()
}

pub fn pane_instances(
    pane: &PaneFrame,
    metrics: CellMetrics,
    palette: &Palette,
    padding: u32,
) -> PaneInstances {
    let geometry = grid_geometry(pane.rect, metrics, padding);
    let snapshot = pane.snapshot;
    let (cw, ch) = (metrics.width as f32, metrics.height as f32);
    let cols = geometry.cols.min(snapshot.cols);
    let rows = geometry.rows.min(snapshot.rows);
    let cursor = visible_cursor(snapshot, &geometry);
    let solid_cursor = pane.focused && snapshot.cursor_shape == CursorShape::Block;
    let mut out = PaneInstances::default();

    for (row, line) in snapshot.lines.iter().enumerate().take(rows) {
        let y = geometry.origin_y as f32 + row as f32 * ch;
        let mut run: Option<(usize, usize, Rgba)> = None;
        for (col, cell) in line.cells().iter().enumerate().take(cols) {
            let x = geometry.origin_x as f32 + col as f32 * cw;
            let (fg, bg) = palette.cell_colors(&cell.style);
            // Fonds : fusion des cellules contiguës de même couleur, fond par défaut omis.
            match run {
                Some((start, len, color)) if color == bg => run = Some((start, len + 1, color)),
                _ => {
                    flush_run(
                        &mut out.backgrounds,
                        run,
                        geometry.origin_x,
                        y,
                        cw,
                        ch,
                        palette.background,
                    );
                    run = Some((col, 1, bg));
                }
            }
            let under_cursor = solid_cursor && cursor == Some((col, row));
            if !is_blank(cell) {
                out.glyphs.push(GlyphRequest {
                    x,
                    y,
                    ch: cell.c,
                    variant: Variant::from_attrs(cell.style.attrs),
                    color: if under_cursor { palette.background } else { fg },
                    wide: cell.is_wide(),
                });
            }
            if cell.is_wide_continuation() {
                continue;
            }
            let span = if cell.is_wide() { cw * 2.0 } else { cw };
            if cell.style.attrs.contains(Attrs::UNDERLINE) {
                out.decorations.push(QuadInstance::new(
                    x,
                    y + metrics.underline_y as f32,
                    span,
                    metrics.underline_thickness as f32,
                    fg,
                ));
            }
            if cell.style.attrs.contains(Attrs::STRIKETHROUGH) {
                out.decorations.push(QuadInstance::new(
                    x,
                    y + metrics.strike_y as f32,
                    span,
                    1.0,
                    fg,
                ));
            }
        }
        flush_run(
            &mut out.backgrounds,
            run,
            geometry.origin_x,
            y,
            cw,
            ch,
            palette.background,
        );
    }

    if let Some((col, row)) = cursor {
        let wide = snapshot
            .lines
            .get(row)
            .is_some_and(|l| col < l.len() && l.get(col).is_wide());
        let w = if wide { cw * 2.0 } else { cw };
        let x = geometry.origin_x as f32 + col as f32 * cw;
        let y = geometry.origin_y as f32 + row as f32 * ch;
        push_cursor(
            &mut out,
            pane.focused,
            snapshot.cursor_shape,
            x,
            y,
            w,
            ch,
            palette.cursor,
        );
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn flush_run(
    backgrounds: &mut Vec<QuadInstance>,
    run: Option<(usize, usize, Rgba)>,
    origin_x: u32,
    y: f32,
    cw: f32,
    ch: f32,
    default_bg: Rgba,
) {
    if let Some((start, len, color)) = run
        && color != default_bg
    {
        backgrounds.push(QuadInstance::new(
            origin_x as f32 + start as f32 * cw,
            y,
            len as f32 * cw,
            ch,
            color,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn push_cursor(
    out: &mut PaneInstances,
    focused: bool,
    shape: CursorShape,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Rgba,
) {
    let bar = CURSOR_BAR_WIDTH as f32;
    match (shape, focused) {
        (CursorShape::Block, true) => out.backgrounds.push(QuadInstance::new(x, y, w, h, color)),
        (CursorShape::Block, false) => out.decorations.extend([
            QuadInstance::new(x, y, w, 1.0, color),
            QuadInstance::new(x, y + h - 1.0, w, 1.0, color),
            QuadInstance::new(x, y, 1.0, h, color),
            QuadInstance::new(x + w - 1.0, y, 1.0, h, color),
        ]),
        (CursorShape::Beam, _) => out.decorations.push(QuadInstance::new(x, y, bar, h, color)),
        (CursorShape::Underline, _) => {
            out.decorations
                .push(QuadInstance::new(x, y + h - bar, w, bar, color))
        }
    }
}

#[cfg(test)]
mod tests {
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
}
