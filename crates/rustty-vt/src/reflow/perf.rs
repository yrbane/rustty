//! Performance du reflow sur un long historique.

use super::*;

/// Un historique de 10 000 lignes courtes, redécoupé à chaque colonne
/// franchie pendant un glisser : doit rester bien sous la frame.
#[test]
fn reflowing_ten_thousand_short_lines_is_fast() {
    let lines: Vec<Line> = (0..10_000)
        .map(|i| {
            let mut l = Line::new(120);
            for (c, ch) in format!("ligne {i} avec un peu de texte")
                .chars()
                .enumerate()
            {
                l.set(c, Cell::new(ch, Default::default()));
            }
            l
        })
        .collect();
    let start = std::time::Instant::now();
    let (out, _) = reflow(lines, &[], 119);
    let elapsed = start.elapsed();
    assert_eq!(out.len(), 10_000);
    assert_eq!(
        out[9_999].text().trim_end(),
        "ligne 9999 avec un peu de texte"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "reflow de 10 000 lignes : {elapsed:?}"
    );
}
