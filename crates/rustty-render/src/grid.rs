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
#[path = "grid_tests.rs"]
mod tests;
