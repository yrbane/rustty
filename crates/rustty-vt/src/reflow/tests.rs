//! Tests du reflow : découpe, jonction, curseurs et bandes d'image.

use super::*;
use crate::color::Color;
use crate::graphics::{ImageData, ImageStrip, Placement};
use std::sync::Arc;

/// Une ligne de `cols` colonnes contenant `text` (ASCII).
fn line(text: &str, cols: usize, wrapped: bool) -> Line {
    let mut l = Line::new(cols);
    for (i, c) in text.chars().enumerate() {
        l.set(i, Cell::new(c, Default::default()));
    }
    l.wrapped = wrapped;
    l
}

fn texts(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.text().trim_end().to_string())
        .collect()
}

fn wraps(lines: &[Line]) -> Vec<bool> {
    lines.iter().map(|l| l.wrapped).collect()
}

#[test]
fn a_long_line_rewraps_narrower() {
    let (out, _) = reflow(vec![line("abcdefgh", 8, false)], &[], 3);
    assert_eq!(texts(&out), ["abc", "def", "gh"]);
    assert_eq!(wraps(&out), [true, true, false]);
    assert!(out.iter().all(|l| l.len() == 3));
}

#[test]
fn wrapped_lines_join_when_wider() {
    let input = vec![
        line("abc", 3, true),
        line("def", 3, true),
        line("g", 3, false),
    ];
    let (out, _) = reflow(input, &[], 10);
    assert_eq!(texts(&out), ["abcdefg"]);
    assert_eq!(wraps(&out), [false]);
}

#[test]
fn hard_line_breaks_are_kept() {
    let input = vec![
        line("ab", 4, false),
        line("", 4, false),
        line("cd", 4, false),
    ];
    let (out, _) = reflow(input, &[], 10);
    assert_eq!(texts(&out), ["ab", "", "cd"]);
}

#[test]
fn trailing_blanks_are_dropped() {
    let (out, _) = reflow(vec![line("ab", 10, false)], &[], 1);
    assert_eq!(
        texts(&out),
        ["a", "b"],
        "les 8 blancs de fin ne deviennent pas 8 lignes"
    );
}

#[test]
fn wide_char_never_splits() {
    let mut l = Line::new(4);
    l.set(0, Cell::new('a', Default::default()));
    let mut wide = Cell::new('漢', Default::default());
    wide.style.attrs |= Attrs::WIDE;
    let mut cont = Cell::new(' ', Default::default());
    cont.style.attrs |= Attrs::WIDE_CONTINUATION;
    l.set(1, wide);
    l.set(2, cont);
    l.set(3, Cell::new('b', Default::default()));
    let (out, _) = reflow(vec![l], &[], 2);
    assert_eq!(
        texts(&out),
        ["a", "漢", "b"],
        "le caractère large passe entier à la ligne suivante"
    );
    assert!(out[1].get(0).is_wide() && out[1].get(1).is_wide_continuation());
    assert!(out[0].wrapped && out[1].wrapped && !out[2].wrapped);
}

#[test]
fn width_one_keeps_every_char() {
    let (out, _) = reflow(vec![line("xyz", 3, false)], &[], 1);
    assert_eq!(texts(&out), ["x", "y", "z"]);
}

#[test]
fn cursor_follows_its_character() {
    let input = vec![line("abcdef", 6, true), line("gh", 6, false)];
    let (out, cursor) = reflow(input.clone(), &[(1, 1)], 4);
    assert_eq!(texts(&out), ["abcd", "efgh"]);
    assert_eq!(cursor, [Some((1, 3))], "sur le h");
    let (_, after_end) = reflow(input, &[(1, 2)], 4);
    assert_eq!(
        after_end,
        [Some((1, 4))],
        "après le dernier caractère, colonne = largeur"
    );
}

#[test]
fn blanks_before_the_cursor_are_kept() {
    let (out, cursor) = reflow(vec![line("$", 10, false)], &[(0, 2)], 5);
    assert_eq!(cursor, [Some((0, 2))], "l'espace après l'invite reste");
    assert_eq!(texts(&out), ["$"]);
}

#[test]
fn empty_input_is_empty() {
    let (out, cursor) = reflow(Vec::new(), &[], 5);
    assert!(out.is_empty());
    assert!(cursor.is_empty());
}

fn strip(cols: u16, col: u16) -> ImageStrip {
    ImageStrip {
        placement: Arc::new(Placement {
            image: Arc::new(ImageData::new(1, 1, vec![0; 4])),
            cols,
            rows: 1,
            width_cells: f32::from(cols),
            height_cells: 1.0,
        }),
        col,
        row: 0,
    }
}

#[test]
fn image_strips_survive_a_reflow() {
    let mut l = line("ab", 10, false);
    l.push_image(strip(2, 3));
    l.push_image(strip(1, 9));
    let (fast, _) = reflow(vec![l.clone()], &[], 5);
    assert_eq!(
        fast[0].images(),
        l.images(),
        "chemin rapide : bandes gardées, colonne comprise"
    );
    let (slow, cursor) = reflow(vec![l.clone()], &[(0, 2)], 5);
    assert_eq!(cursor, [Some((0, 2))]);
    assert_eq!(
        slow[0].images(),
        l.images(),
        "chemin lent : la ligne refaite garde ses bandes"
    );
}

#[test]
fn strips_of_a_wrapped_logical_line_go_to_its_first_line() {
    let mut input = vec![
        line("abc", 3, true),
        line("def", 3, true),
        line("g", 3, false),
    ];
    let strips = [strip(1, 0), strip(1, 1), strip(1, 2)];
    for (l, s) in input.iter_mut().zip(&strips) {
        l.push_image(s.clone());
    }
    let (wider, _) = reflow(input, &[], 10);
    assert_eq!(texts(&wider), ["abcdefg"]);
    assert_eq!(wider[0].images(), strips);

    let mut long = line("abcdef", 6, false);
    long.push_image(strip(2, 4));
    let (narrower, _) = reflow(vec![long.clone()], &[], 3);
    assert_eq!(texts(&narrower), ["abc", "def"]);
    assert_eq!(narrower[0].images(), long.images(), "colonne conservée");
    assert!(narrower[1].images().is_empty());
}

#[test]
fn colored_blank_tails_do_not_multiply() {
    let mut red = Cell::default();
    red.style.bg = Color::Indexed(1);
    let l = Line::filled(20, red);
    let (out, _) = reflow(vec![l.clone()], &[], 10);
    assert_eq!(
        out.len(),
        1,
        "le fond coloré de la queue est perdu, pas multiplié"
    );
    let mut wrapped = l;
    wrapped.wrapped = true;
    let (out, _) = reflow(vec![wrapped, line("x", 20, false)], &[], 10);
    assert_eq!(
        texts(&out),
        ["", "", "x"],
        "chemin lent : la queue colorée est retirée aussi"
    );
}

#[test]
fn a_wide_continuation_is_never_a_blank() {
    let mut l = line("a", 10, false);
    let mut wide = Cell::new('漢', Default::default());
    wide.style.attrs |= Attrs::WIDE;
    let mut cont = Cell::new(' ', Default::default());
    cont.style.attrs |= Attrs::WIDE_CONTINUATION;
    l.set(8, wide);
    l.set(9, cont);
    let (out, _) = reflow(vec![l], &[], 9);
    assert_eq!(
        out.len(),
        2,
        "le caractère large ne tient plus : il passe à la ligne"
    );
    assert!(out[1].get(0).is_wide() && out[1].get(1).is_wide_continuation());
}

#[test]
fn only_the_first_cursor_keeps_trailing_blanks() {
    // Le premier curseur est le curseur vivant ; un curseur sauvegardé dans la
    // queue blanche ou au-delà de la ligne n'y retient aucun blanc.
    let (out, moved) = reflow(vec![line("ab", 10, false)], &[(0, 1), (0, 5), (0, 12)], 3);
    assert_eq!(texts(&out), ["ab"], "la queue n'est pas multipliée");
    assert_eq!(moved, [Some((0, 1)), Some((0, 2)), Some((0, 2))]);
}

#[test]
fn a_cursor_on_a_missing_row_maps_to_none() {
    let (out, moved) = reflow(vec![line("ab", 4, false)], &[(0, 1), (5, 0)], 4);
    assert_eq!(texts(&out), ["ab"]);
    assert_eq!(moved, [Some((0, 1)), None]);
}
