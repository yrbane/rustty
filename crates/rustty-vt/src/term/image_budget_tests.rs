//! Tests de la mémoire des images posées : copies réduites et budget.

use crate::graphics::test_support::{apc, apc_png};
use crate::term::Term;
use crate::term::test_support::{feed, term};

/// Octets RGBA d'une copie 200×200 (400×400 posée sur 10 colonnes de 10 px).
const REDUCED: usize = 200 * 200 * 4;

fn strips(t: &Term) -> usize {
    (0..t.rows()).map(|r| t.grid().line(r).images().len()).sum()
}

#[test]
fn placed_image_keeps_only_its_display_pixels() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=T,f=100", 400, 400));
    let p = &t.grid().line(0).images()[0].placement;
    assert_eq!((p.cols, p.rows), (10, 5));
    assert_eq!((p.image.width, p.image.height), (200, 200));
    assert!(
        p.image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|px| *px == [200, 30, 30, 255])
    );
}

#[test]
fn small_put_shares_the_stored_image() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=t,f=100,i=4,q=2", 10, 20));
    feed(&mut t, &apc("a=p,i=4,q=2"));
    let shown = t.grid().line(0).images()[0].placement.image.id;
    assert_eq!(shown, t.images.get(4).unwrap().id);
}

#[test]
fn delete_by_id_reaches_reduced_copies() {
    let mut t = term(10, 5);
    feed(&mut t, &apc_png("a=T,f=100,i=1,q=2", 400, 400));
    assert_eq!(strips(&t), 5);
    feed(&mut t, &apc("a=d,d=i,i=1"));
    assert_eq!(strips(&t), 0);
}

#[test]
fn many_large_placements_stay_under_the_budget() {
    let mut t = Term::new(10, 5, 1000);
    t.placed_budget = 3 * REDUCED;
    for _ in 0..10 {
        feed(&mut t, &apc_png("a=T,f=100", 400, 400));
        feed(&mut t, "\r\n");
        assert!(t.placed_bytes() <= 3 * REDUCED, "{}", t.placed_bytes());
    }
    // L'image la plus récente reste à l'écran, intacte.
    let newest = &t.grid().line(3).images()[0].placement;
    assert_eq!((newest.image.width, newest.image.height), (200, 200));
    assert_eq!(strips(&t), 4);
    // L'historique garde les plus récentes, pas les plus anciennes.
    let oldest_with_image = (0..t.scrollback().len())
        .rev()
        .find(|&i| !t.scrollback().get(i).unwrap().images().is_empty())
        .unwrap();
    assert!(oldest_with_image < 15, "{oldest_with_image}");
}

#[test]
fn screen_alone_over_budget_keeps_the_newest() {
    let mut t = term(20, 10);
    t.placed_budget = 1;
    feed(&mut t, &apc_png("a=T,f=100", 100, 100));
    feed(&mut t, "\x1b[1;11H");
    feed(&mut t, &apc_png("a=T,f=100", 100, 100));
    let row0 = t.grid().line(0).images();
    assert_eq!(row0.len(), 1);
    assert_eq!(row0[0].col, 10);
}
