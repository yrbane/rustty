//! Calcul de l'emprise d'une image en cellules.

/// Emprise d'une image : `(colonnes, rangées, largeur_cellules, hauteur_cellules)`.
///
/// `img` et `cell` sont en pixels. `cols` / `rows` (`c=` / `r=`) fixent
/// l'étendue demandée ; avec un seul des deux, le rapport d'aspect est conservé.
/// Le résultat est réduit pour tenir dans `term_cols`, et occupe au moins 1×1.
pub fn extent(
    img: (u32, u32),
    cell: (u32, u32),
    term_cols: usize,
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
    let max_w = term_cols.max(1) as f32;
    if w > max_w {
        h *= max_w / w;
        w = max_w;
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
        assert_eq!(extent((20, 40), (10, 20), 80, None, None), (2, 2, 2.0, 2.0));
    }

    #[test]
    fn extent_shrinks_to_the_terminal_width() {
        assert_eq!(
            extent((200, 40), (10, 20), 10, None, None),
            (10, 1, 10.0, 1.0)
        );
    }

    #[test]
    fn extent_with_cols_keeps_the_ratio() {
        // 20×40 px : ratio 1:2 ; 4 colonnes de 10 px = 40 px de large, 80 px de haut = 4 rangées.
        assert_eq!(
            extent((20, 40), (10, 20), 80, Some(4), None),
            (4, 4, 4.0, 4.0)
        );
        assert_eq!(
            extent((20, 40), (10, 20), 80, None, Some(1)),
            (1, 1, 1.0, 1.0)
        );
    }

    #[test]
    fn extent_with_both_uses_both() {
        assert_eq!(
            extent((20, 40), (10, 20), 80, Some(5), Some(3)),
            (5, 3, 5.0, 3.0)
        );
    }

    #[test]
    fn extent_never_returns_zero() {
        let (c, r, w, h) = extent((1, 1), (10, 20), 80, None, None);
        assert_eq!((c, r), (1, 1));
        assert!(w > 0.0 && h > 0.0);
    }
}
