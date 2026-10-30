//! Le mot souligné sous une cellule : plage de cellules soulignées contiguës
//! d'une ligne, son texte, la ligne entière en contexte et la position du mot
//! dans ce contexte en unités UTF-16 (celles des offsets de LanguageTool). Pur.

use rustty_vt::{Attrs, Line};

/// Une plage de cellules soulignées, bornes d'espaces retirées.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnderlinedRun {
    pub row: usize,
    /// Première colonne du mot.
    pub start: usize,
    /// Colonne après la dernière du mot.
    pub end: usize,
    /// Le texte souligné, tel que lu dans les cellules.
    pub word: String,
    /// Texte de la ligne entière, espaces de fin retirés.
    pub context: String,
    /// Position du mot dans `context`, en unités UTF-16.
    pub offset: usize,
    /// Longueur du mot, en unités UTF-16.
    pub length: usize,
}

/// Les plages soulignées d'une ligne, `[start, end)` en colonnes, espaces de
/// bord retirés ; une plage réduite à des espaces est ignorée.
pub fn runs_in_row(line: &Line) -> Vec<(usize, usize)> {
    let cells = line.cells();
    let mut runs = Vec::new();
    let mut col = 0;
    while col < cells.len() {
        if !cells[col].style.attrs.contains(Attrs::UNDERLINE) {
            col += 1;
            continue;
        }
        let start = col;
        while col < cells.len() && cells[col].style.attrs.contains(Attrs::UNDERLINE) {
            col += 1;
        }
        if let Some(trimmed) = trim_blank(line, start, col) {
            runs.push(trimmed);
        }
    }
    runs
}

/// Retire les cellules blanches aux deux bouts de `[start, end)`.
fn trim_blank(line: &Line, mut start: usize, mut end: usize) -> Option<(usize, usize)> {
    let blank = |col: usize| {
        let cell = line.get(col);
        cell.c == ' ' && !cell.is_wide_continuation()
    };
    while start < end && blank(start) {
        start += 1;
    }
    while end > start && blank(end - 1) {
        end -= 1;
    }
    (start < end).then_some((start, end))
}

/// Texte de `[start, end)` de la ligne, continuations de caractères larges
/// exclues, combinants inclus.
fn text_of(line: &Line, start: usize, end: usize) -> String {
    let mut out = String::new();
    for col in start..end.min(line.len()) {
        let cell = line.get(col);
        if cell.is_wide_continuation() {
            continue;
        }
        out.push(cell.c);
        if let Some(zw) = line.zerowidth(col) {
            out.push_str(zw);
        }
    }
    out
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// La plage soulignée de `lines[row]` qui contient `col`, s'il y en a une.
pub fn underlined_run(lines: &[Line], row: usize, col: usize) -> Option<UnderlinedRun> {
    let line = lines.get(row)?;
    let (start, end) = runs_in_row(line)
        .into_iter()
        .find(|(s, e)| (*s..*e).contains(&col))?;
    let before = text_of(line, 0, start);
    let word = text_of(line, start, end);
    let after = text_of(line, end, line.len());
    let context = format!("{before}{word}{}", after.trim_end());
    Some(UnderlinedRun {
        row,
        start,
        end,
        offset: utf16_len(&before),
        length: utf16_len(&word),
        word,
        context,
    })
}

/// Rang de `run` parmi les plages soulignées de même texte, de haut en bas
/// puis de gauche à droite : ce que le mod ortho utilise pour viser la bonne
/// correspondance quand le même mot est fautif plusieurs fois.
pub fn occurrence_index(lines: &[Line], run: &UnderlinedRun) -> usize {
    lines
        .iter()
        .take(run.row + 1)
        .enumerate()
        .flat_map(|(row, line)| {
            runs_in_row(line)
                .into_iter()
                .map(move |(start, end)| (row, start, end, line))
        })
        .filter(|(row, start, _, _)| (*row, *start) < (run.row, run.start))
        .filter(|(_, start, end, line)| text_of(line, *start, *end) == run.word)
        .count()
}

/// Jeton envoyé au programme quand une suggestion est choisie : le mod ortho
/// de Claude Code le consomme et remplace le mot dans le brouillon.
/// Voir `docs/superpowers/specs/2026-10-30-spell-popup-design.md`.
pub fn apply_token(word: &str, replacement: &str, occurrence: usize) -> String {
    format!("\u{E000}ortho\u{E001}{word}\u{E001}{replacement}\u{E001}{occurrence}\u{E002}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustty_vt::{Cell, Style};

    fn underlined(c: char) -> Cell {
        Cell::new(
            c,
            Style {
                attrs: Attrs::UNDERLINE,
                ..Style::default()
            },
        )
    }

    /// Une ligne depuis `text`, où les caractères entre `_` sont soulignés.
    fn line(text: &str) -> Line {
        let mut l = Line::new(text.chars().filter(|c| *c != '_').count());
        let mut col = 0;
        let mut under = false;
        for c in text.chars() {
            if c == '_' {
                under = !under;
                continue;
            }
            l.set(
                col,
                if under {
                    underlined(c)
                } else {
                    Cell::new(c, Style::default())
                },
            );
            col += 1;
        }
        l
    }

    #[test]
    fn runs_are_the_maximal_underlined_spans_without_blank_edges() {
        assert_eq!(
            runs_in_row(&line("> yo _cé_ coule ma _poulle _ ok")),
            vec![(5, 7), (17, 23)]
        );
        assert_eq!(runs_in_row(&line("rien de souligné")), vec![]);
        assert_eq!(runs_in_row(&line("_   _ blanc")), vec![], "que des espaces");
    }

    #[test]
    fn run_under_a_cell_carries_word_context_and_utf16_offsets() {
        let lines = vec![line("> yo _cé_ coule ma _poulle_   ")];
        let run = underlined_run(&lines, 0, 20).unwrap();
        assert_eq!(run.word, "poulle");
        assert_eq!((run.start, run.end), (17, 23));
        assert_eq!(run.context, "> yo cé coule ma poulle");
        assert_eq!(run.offset, 17);
        assert_eq!(run.length, 6);
        assert_eq!(underlined_run(&lines, 0, 10), None, "cellule non soulignée");
        assert_eq!(underlined_run(&lines, 3, 0), None, "ligne absente");
    }

    #[test]
    fn offsets_count_utf16_units_not_cells() {
        // Un caractère hors du plan de base occupe deux unités UTF-16.
        let lines = vec![line("𝄞 _fote_")];
        let run = underlined_run(&lines, 0, 2).unwrap();
        assert_eq!(run.offset, 3);
        assert_eq!(run.length, 4);
    }

    #[test]
    fn occurrence_counts_same_words_before_the_run() {
        let lines = vec![line("_fote_ et _fote_"), line("_autre_ _fote_")];
        let third = underlined_run(&lines, 1, 8).unwrap();
        assert_eq!(occurrence_index(&lines, &third), 2);
        let first = underlined_run(&lines, 0, 0).unwrap();
        assert_eq!(occurrence_index(&lines, &first), 0);
        let other = underlined_run(&lines, 1, 0).unwrap();
        assert_eq!(occurrence_index(&lines, &other), 0);
    }

    #[test]
    fn token_follows_the_ortho_contract() {
        assert_eq!(
            apply_token("poulle", "poule", 1),
            "\u{E000}ortho\u{E001}poulle\u{E001}poule\u{E001}1\u{E002}"
        );
    }
}
