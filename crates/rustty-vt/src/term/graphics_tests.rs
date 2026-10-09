//! Tests de l'exécution des commandes graphiques par `Term`.

use crate::graphics::test_support::{apc, apc_png};
use crate::term::test_support::{feed, term};

fn strips_on(t: &crate::term::Term, row: usize) -> usize {
    t.grid().line(row).images().len()
}

fn reply(t: &mut crate::term::Term) -> String {
    String::from_utf8(t.drain_responses()).unwrap()
}

#[test]
fn transmit_and_put_places_strips_and_moves_the_cursor() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=T,f=100,i=1", 30, 50));
    for row in 0..3 {
        let strips = t.grid().line(row).images();
        assert_eq!(strips.len(), 1, "ligne {row}");
        assert_eq!((strips[0].col, strips[0].row), (0, row as u16));
        assert_eq!(strips[0].placement.cols, 3);
        assert_eq!(strips[0].placement.rows, 3);
    }
    assert_eq!(strips_on(&t, 3), 0);
    assert_eq!((t.cursor().row, t.cursor().col), (2, 3));
}

#[test]
fn put_starts_at_the_cursor_column() {
    let mut t = term(10, 5);
    feed(&mut t, "ab");
    feed(&mut t, &apc_png("a=T,f=100", 10, 20));
    assert_eq!(t.grid().line(0).images()[0].col, 2);
    assert_eq!((t.cursor().row, t.cursor().col), (0, 3));
}

#[test]
fn image_reaching_the_right_edge_sets_pending_wrap() {
    let mut t = term(3, 5);
    feed(&mut t, &apc_png("a=T,f=100", 30, 20));
    assert_eq!(t.cursor().col, 2);
    assert!(t.cursor().pending_wrap);
}

#[test]
fn image_at_the_bottom_scrolls_the_screen() {
    let mut t = term(10, 5);
    feed(&mut t, "\n\n\n\n");
    assert_eq!(t.cursor().row, 4);
    feed(&mut t, &apc_png("a=T,f=100", 30, 50));
    for row in 2..5 {
        assert_eq!(strips_on(&t, row), 1, "ligne {row}");
    }
    assert_eq!(strips_on(&t, 1), 0);
    assert_eq!(t.cursor().row, 4);
}

#[test]
fn image_scrolls_into_history_with_its_lines() {
    let mut t = term(10, 3);
    feed(&mut t, &apc_png("a=T,f=100", 10, 20));
    feed(&mut t, &"\n".repeat(10));
    assert_eq!(t.grid().line(0).images().len(), 0);
    let hist = (0..t.scrollback().len())
        .filter(|&i| !t.scrollback().get(i).unwrap().images().is_empty())
        .count();
    assert_eq!(hist, 1);
    t.scroll_display(100);
    let snap = t.snapshot();
    assert!(snap.lines.iter().any(|l| !l.images().is_empty()));
}

#[test]
fn transmit_then_put_by_id() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=t,f=100,i=4,q=2", 10, 20));
    assert_eq!(strips_on(&t, 0), 0);
    feed(&mut t, &apc("a=p,i=4,q=2"));
    assert_eq!(strips_on(&t, 0), 1);
}

#[test]
fn put_unknown_id_reports_enoent() {
    let mut t = term(10, 5);
    feed(&mut t, &apc("a=p,i=9"));
    let r = reply(&mut t);
    assert!(r.starts_with("\x1b_Gi=9;ENOENT:"), "{r:?}");
    assert!(r.ends_with("\x1b\\"));
}

#[test]
fn ok_reply_only_with_id_and_not_quiet() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=T,f=100,i=7", 10, 20));
    assert_eq!(reply(&mut t), "\x1b_Gi=7;OK\x1b\\");
    feed(&mut t, &apc_png("a=T,f=100,i=7,q=1", 10, 20));
    assert_eq!(reply(&mut t), "");
    feed(&mut t, &apc_png("a=T,f=100", 10, 20));
    assert_eq!(reply(&mut t), "");
}

#[test]
fn errors_are_silenced_by_q2_or_missing_id() {
    let mut t = term(10, 5);
    feed(&mut t, &apc("a=p,i=9,q=1"));
    assert!(reply(&mut t).contains("ENOENT"));
    feed(&mut t, &apc("a=p,i=9,q=2"));
    assert_eq!(reply(&mut t), "");
    feed(&mut t, &apc("a=p"));
    assert_eq!(reply(&mut t), "");
}

#[test]
fn transmit_without_display_forgets_nothing_visible() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=t,f=100", 10, 20));
    assert_eq!(strips_on(&t, 0), 0);
    assert_eq!((t.cursor().row, t.cursor().col), (0, 0));
}

#[test]
fn garbage_payload_reports_an_error_and_keeps_going() {
    let mut t = term(10, 5);
    feed(&mut t, "\x1b_Ga=T,f=100,i=3;!!!notbase64\x1b\\ok");
    assert!(reply(&mut t).starts_with("\x1b_Gi=3;EINVAL:"));
    assert_eq!(t.text()[0].trim_end(), "ok");
}

#[test]
fn erase_display_removes_images() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=T,f=100", 10, 20));
    assert_eq!(strips_on(&t, 0), 1);
    feed(&mut t, "\x1b[2J");
    assert_eq!(strips_on(&t, 0), 0);
}

#[test]
fn delete_all_removes_visible_strips() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=T,f=100,i=1", 10, 40));
    feed(&mut t, &apc("a=d,d=a,q=2"));
    assert_eq!((0..5).map(|r| strips_on(&t, r)).sum::<usize>(), 0);
    // Le magasin garde l'image pour d=a (minuscule) : un put réussit encore.
    feed(&mut t, &apc("a=p,i=1,q=2"));
    assert_eq!((0..5).map(|r| strips_on(&t, r)).sum::<usize>(), 2);
}

#[test]
fn delete_by_id_removes_strips_and_forgets_the_image() {
    let mut t = term(10, 8);
    feed(&mut t, &apc_png("a=T,f=100,i=1", 10, 20));
    feed(&mut t, &apc_png("a=T,f=100,i=2", 10, 20));
    feed(&mut t, &apc("a=d,d=i,i=1"));
    let total: usize = (0..8).map(|r| strips_on(&t, r)).sum();
    assert_eq!(total, 1);
    feed(&mut t, &apc("a=p,i=1"));
    assert!(reply(&mut t).contains("ENOENT"));
}

#[test]
fn set_cell_pixels_changes_extent_and_ignores_zero() {
    let mut t = term(20, 10);
    t.set_cell_pixels(0, 0);
    t.set_cell_pixels(5, 10);
    feed(&mut t, &apc_png("a=T,f=100", 20, 40));
    assert_eq!(t.grid().line(0).images()[0].placement.cols, 4);
    assert_eq!(t.grid().line(0).images()[0].placement.rows, 4);
}

#[test]
fn ris_abandons_an_interrupted_transfer() {
    let mut t = term(10, 5);
    feed(&mut t, &apc("a=T,m=1;AAAA"));
    feed(&mut t, "texte\x1bc");
    feed(&mut t, &apc_png("a=T,f=100,i=5", 10, 20));
    assert_eq!(reply(&mut t), "\x1b_Gi=5;OK\x1b\\");
    assert_eq!(strips_on(&t, 0), 1);
}

#[test]
fn ris_forgets_stored_images() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=t,f=100,i=4,q=2", 10, 20));
    feed(&mut t, "\x1bc");
    feed(&mut t, &apc("a=p,i=4"));
    assert!(reply(&mut t).contains("ENOENT"));
}
