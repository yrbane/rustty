//! Calcul de l'emprise d'une image en cellules.

/// Emprise d'une image : `(colonnes, rangées, largeur_cellules, hauteur_cellules)`.
///
/// `img` et `cell` sont en pixels. `cols` / `rows` (`c=` / `r=`) fixent
/// l'étendue demandée ; avec un seul des deux, le rapport d'aspect est conservé.
/// Le résultat est réduit, rapport conservé, pour tenir dans `max` (colonnes,
/// rangées), et occupe au moins 1×1.
pub fn extent(
    img: (u32, u32),
    cell: (u32, u32),
    max: (usize, usize),
    cols: Option<u16>,
    rows: Option<u16>,
) -> (u16, u16, f32, f32) {
    let (iw, ih) = (img.0.max(1) as f32, img.1.max(1) as f32);
    let (cw, ch) = (cell.0.max(1) as f32, cell.1.max(1) as f32);
    let (mut w, mut h) = match (cols, rows) {
        (Some(c), Some(r)) => (f32::from(c.max(1)), f32::from(r.max(1))),
        (Some(c), None) => {
            let w = f32::from(c.max(1));
            (w, w * cw * ih / iw / ch)
        }
        (None, Some(r)) => {
            let h = f32::from(r.max(1));
            (h * ch * iw / ih / cw, h)
        }
        (None, None) => (iw / cw, ih / ch),
    };
    let max_w = max.0.max(1) as f32;
    if w > max_w {
        h *= max_w / w;
        w = max_w;
    }
    let max_h = max.1.max(1) as f32;
    if h > max_h {
        w *= max_h / h;
        h = max_h;
    }
    (ceil_cells(w), ceil_cells(h), w, h)
}

/// Nombre de cellules entières occupées, au moins 1.
fn ceil_cells(v: f32) -> u16 {
    (v.ceil() as u16).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extent_keeps_the_natural_size() {
        assert_eq!(
            extent((20, 40), (10, 20), (80, 1000), None, None),
            (2, 2, 2.0, 2.0)
        );
    }

    #[test]
    fn extent_shrinks_to_the_terminal_width() {
        assert_eq!(
            extent((200, 40), (10, 20), (10, 1000), None, None),
            (10, 1, 10.0, 1.0)
        );
    }

    #[test]
    fn extent_with_cols_keeps_the_ratio() {
        // 20×40 px : ratio 1:2 ; 4 colonnes de 10 px = 40 px de large, 80 px de haut = 4 rangées.
        assert_eq!(
            extent((20, 40), (10, 20), (80, 1000), Some(4), None),
            (4, 4, 4.0, 4.0)
        );
        assert_eq!(
            extent((20, 40), (10, 20), (80, 1000), None, Some(1)),
            (1, 1, 1.0, 1.0)
        );
    }

    #[test]
    fn extent_with_both_uses_both() {
        assert_eq!(
            extent((20, 40), (10, 20), (80, 1000), Some(5), Some(3)),
            (5, 3, 5.0, 3.0)
        );
    }

    #[test]
    fn extent_caps_the_rows_and_keeps_the_ratio() {
        assert_eq!(
            extent((10, 20), (10, 20), (80, 96), Some(1), Some(65535)),
            (1, 96, 96.0 / 65535.0, 96.0)
        );
        // 1×8192 px sur 80 colonnes : 327 680 rangées sans plafond.
        let (c, r, _, h) = extent((1, 8192), (10, 20), (80, 96), Some(80), None);
        assert_eq!((c, r, h), (1, 96, 96.0));
    }

    #[test]
    fn extent_never_returns_zero() {
        let (c, r, w, h) = extent((1, 1), (10, 20), (80, 1000), None, None);
        assert_eq!((c, r), (1, 1));
        assert!(w > 0.0 && h > 0.0);
    }
}
