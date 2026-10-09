//! Tests du redimensionnement de `Term` (reflow, historique, curseurs).

use crate::term::test_support::{feed, term};

#[test]
fn lines_below_the_cursor_are_never_lost() {
    let mut t = term(10, 4);
    feed(
        &mut t,
        "aaaaaaaaaa\r\nbbbbbbbbbb\r\ncccccccccc\r\ndddddddddd\x1b[H",
    );
    t.resize(5, 4);
    let history: Vec<String> = (0..t.scrollback().len())
        .rev()
        .map(|i| t.scrollback().get(i).unwrap().text().trim_end().to_string())
        .collect();
    let all = [history, t.text()].concat().join("|");
    for part in ["aaaaa", "bbbbb", "ccccc", "ddddd"] {
        assert_eq!(all.matches(part).count(), 2, "{part} perdu : {all}");
    }
}

#[test]
fn alt_screen_resize_reflows_the_primary_screen() {
    let mut t = term(10, 3);
    feed(&mut t, "0123456789AB\x1b[?1049hvim");
    t.resize(6, 3);
    feed(&mut t, "\x1b[?1049l");
    assert_eq!(
        t.text(),
        ["012345", "6789AB", ""],
        "l'écran principal n'est pas tronqué"
    );
}

#[test]
fn resize_narrower_reflows_screen_and_history() {
    let mut t = term(10, 3);
    feed(&mut t, "0123456789AB");
    t.resize(6, 3);
    assert_eq!(t.text(), ["012345", "6789AB", ""]);
    let c = t.cursor();
    assert_eq!(
        (c.row, c.col, c.pending_wrap),
        (1, 5, true),
        "après le B, retour à la ligne en attente"
    );
}

#[test]
fn resize_wider_unwraps() {
    let mut t = term(4, 3);
    feed(&mut t, "abcdef");
    assert_eq!(t.text(), ["abcd", "ef", ""]);
    t.resize(8, 3);
    assert_eq!(t.text(), ["abcdef", "", ""]);
    let c = t.cursor();
    assert_eq!((c.row, c.col), (0, 6));
}

#[test]
fn history_is_reflowed_too() {
    let mut t = term(4, 2);
    feed(&mut t, "abcdefgh\r\nzz\r\nyy");
    assert_eq!(
        t.scrollback().len(),
        2,
        "abcd et efgh sont sortis par le haut"
    );
    t.resize(8, 2);
    assert_eq!(t.text(), ["zz", "yy"]);
    assert_eq!(t.scrollback().len(), 1);
    assert_eq!(t.scrollback().get(0).unwrap().text().trim_end(), "abcdefgh");
}

#[test]
fn alt_screen_is_not_reflowed() {
    let mut t = term(4, 2);
    feed(&mut t, "\x1b[?1049habcdef");
    t.resize(8, 2);
    assert_eq!(
        t.text(),
        ["abcd", "ef"],
        "l'application redessine elle-même l'écran alternatif"
    );
}

#[test]
fn cursor_stays_visible_after_reflow() {
    let mut t = term(10, 2);
    feed(&mut t, "0123456789\r\n0123456789\r\nab");
    t.resize(3, 2);
    let c = t.cursor();
    assert!(c.row < 2);
    assert_eq!(t.text()[c.row], "ab");
    assert!(
        t.scrollback().len() >= 6,
        "les lignes repliées sont parties dans l'historique"
    );
}

#[test]
fn rows_only_change_keeps_previous_behavior() {
    let mut t = term(5, 2);
    feed(&mut t, "a\r\nb\r\nc");
    t.resize(5, 3);
    assert_eq!(
        t.text(),
        ["a", "b", "c"],
        "une ligne rapatriée de l'historique"
    );
}

#[test]
fn resize_grow_keeps_content_and_cursor() {
    let mut t = term(3, 2);
    feed(&mut t, "abc\r\nd");
    t.resize(5, 4);
    assert_eq!(t.text(), vec!["abc", "d", "", ""]);
    assert_eq!((t.cursor().col, t.cursor().row), (1, 1));
    assert_eq!((t.region.top, t.region.bottom), (0, 3));
}

#[test]
fn resize_shrink_clamps_cursor_and_region() {
    let mut t = term(10, 5);
    feed(&mut t, "\x1b[2;4r\x1b[4;9H\x1b7");
    t.resize(4, 2);
    assert_eq!((t.cursor().col, t.cursor().row), (3, 1));
    assert_eq!((t.region.top, t.region.bottom), (0, 1));
    feed(&mut t, "\x1b8");
    assert_eq!(
        (t.cursor().col, t.cursor().row),
        (3, 1),
        "le curseur sauvegardé est borné aussi"
    );
    feed(&mut t, "x");
    assert_eq!(
        t.text(),
        vec!["", "   x"],
        "écrire après resize ne panique pas"
    );
}

#[test]
fn resize_applies_to_scrollback_and_alt_screen() {
    let mut t = term(3, 1);
    feed(&mut t, "a\r\nb");
    t.resize(5, 1);
    assert_eq!(t.scrollback().get(0).unwrap().len(), 5);
    feed(&mut t, "\x1b[?1049h");
    assert_eq!((t.grid().cols(), t.grid().rows()), (5, 1));
}

#[test]
fn resize_resets_tabs_and_pending_wrap() {
    let mut t = term(3, 1);
    feed(&mut t, "abc");
    assert!(t.cursor().pending_wrap);
    t.resize(20, 1);
    assert!(!t.cursor().pending_wrap);
    feed(&mut t, "\x1b[1;1H\t\t");
    assert_eq!(t.cursor().col, 16);
}

#[test]
fn resize_to_zero_is_clamped_to_one() {
    let mut t = term(3, 1);
    t.resize(0, 0);
    assert_eq!((t.grid().cols(), t.grid().rows()), (1, 1));
}

#[test]
fn resize_shrink_rows_keeps_cursor_line_by_scrolling_into_history() {
    let mut t = term(10, 4);
    feed(&mut t, "line1\r\nline2\r\nline3\r\n$ ");
    t.resize(10, 2);
    assert_eq!(t.text(), vec!["line3", "$"]);
    assert_eq!((t.cursor().col, t.cursor().row), (2, 1));
    assert_eq!(t.scrollback().len(), 2);
    assert_eq!(t.scrollback().get(0).unwrap().text().trim_end(), "line2");
}

#[test]
fn resize_grow_rows_pulls_lines_back_from_history() {
    let mut t = term(10, 4);
    feed(&mut t, "line1\r\nline2\r\nline3\r\n$ ");
    t.resize(10, 2);
    t.resize(10, 4);
    assert_eq!(t.text(), vec!["line1", "line2", "line3", "$"]);
    assert_eq!((t.cursor().col, t.cursor().row), (2, 3));
    assert!(t.scrollback().is_empty());
}

#[test]
fn resize_shrink_rows_on_alt_screen_never_touches_history() {
    let mut t = term(10, 3);
    feed(&mut t, "\x1b[?1049ha\r\nb\r\nc");
    t.resize(10, 1);
    assert!(t.scrollback().is_empty());
    assert_eq!(t.cursor().row, 0);
}

#[test]
fn saved_cursor_follows_the_reflow() {
    let mut t = term(6, 3);
    // ESC 7 sur le « h » de la ligne enroulée, puis le curseur s'en va.
    feed(&mut t, "abcdefgh\x1b[2;2H\x1b7\x1b[3;1H");
    t.resize(4, 3);
    assert_eq!(t.text(), ["abcd", "efgh", ""]);
    feed(&mut t, "\x1b8X");
    assert_eq!(
        t.text(),
        ["abcd", "efgX", ""],
        "ESC 8 revient sur le même caractère"
    );
}

#[test]
fn full_history_keeps_whole_logical_lines() {
    let mut t = crate::term::Term::new(6, 2, 4);
    feed(&mut t, "aaaaaa\r\nbbbbbb\r\ncccccc\r\n$");
    t.resize(3, 2);
    t.resize(6, 2);
    let history: Vec<String> = (0..t.scrollback().len())
        .rev()
        .map(|i| t.scrollback().get(i).unwrap().text().trim_end().to_string())
        .collect();
    assert_eq!(
        history,
        ["bbbbbb"],
        "aucune ligne d'historique ne commence par une suite enroulée"
    );
    assert_eq!(t.text(), ["cccccc", "$"]);
}

#[test]
fn image_rows_below_the_cursor_survive_a_reflow() {
    let mut t = term(10, 5);
    feed(
        &mut t,
        &crate::graphics::test_support::apc_png("a=T,f=100", 30, 50),
    );
    feed(&mut t, "\x1b[H");
    t.resize(8, 5);
    for row in 0..3 {
        assert_eq!(t.grid().line(row).images().len(), 1, "ligne {row}");
    }
}

#[test]
fn a_stale_saved_cursor_never_pushes_the_prompt_into_history() {
    let mut t = term(10, 6);
    // DECSC laissé en bas (sortie de vim), puis `clear` et une invite en haut.
    feed(&mut t, "\x1b[6;1H\x1b7\x1b[2J\x1b[H$ ");
    t.resize(8, 3);
    let c = t.cursor();
    assert_eq!(
        t.text()[c.row],
        "$",
        "l'invite reste à l'écran sous le curseur"
    );
    assert_eq!(c.col, 2);
}

#[test]
fn colored_blank_rows_below_the_cursor_are_not_kept() {
    let mut t = term(10, 4);
    // Effacement à fond rouge : toutes les rangées sont des espaces colorées.
    feed(&mut t, "\x1b[41m\x1b[2J\x1b[H$");
    t.resize(8, 2);
    assert_eq!(t.text()[t.cursor().row], "$");
    assert!(
        t.scrollback().is_empty(),
        "aucune rangée blanche ne pousse l'invite"
    );
}
