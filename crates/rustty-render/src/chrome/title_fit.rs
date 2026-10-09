//! Ajustement d'un titre d'onglet à un nombre de colonnes.

use unicode_width::UnicodeWidthStr;

/// Largeur d'un titre en colonnes d'affichage.
pub(super) fn title_cells(title: &str) -> u32 {
    title.width() as u32
}

/// Tronque `title` à `max_cells` colonnes, avec `…` si nécessaire.
pub(super) fn fit_title(title: &str, max_cells: u32) -> String {
    if title_cells(title) <= max_cells {
        return title.to_string();
    }
    match max_cells {
        0 => return String::new(),
        1 => return "…".to_string(),
        _ => {}
    }
    let mut out = String::new();
    for ch in title.chars() {
        if title_cells(&out) + 2 > max_cells {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_titles_are_kept() {
        assert_eq!(fit_title("sh", 5), "sh");
    }

    #[test]
    fn long_titles_end_with_an_ellipsis_within_the_width() {
        let t = fit_title("un titre vraiment trop long", 8);
        assert!(t.ends_with('…'));
        assert!(title_cells(&t) <= 8, "{t}");
    }

    #[test]
    fn fit_title_handles_tiny_widths() {
        assert_eq!(fit_title("titre", 0), "");
        assert_eq!(fit_title("titre", 1), "…");
        assert_eq!(fit_title("", 0), "");
    }
}
