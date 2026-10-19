//! Tests de l'aperçu de dépôt.

use rustty_layout::Side;
use rustty_render::{PixelRect, Rgba, TabRect};

use super::*;
use crate::drag::DropTarget;
use crate::tab::TermId;

const COLOR: Rgba = Rgba::new(1.0, 1.0, 1.0, 1.0);

fn panes() -> Vec<(TermId, PixelRect)> {
    vec![
        (TermId(1), PixelRect::new(0, 0, 200, 100)),
        (TermId(2), PixelRect::new(210, 0, 201, 101)),
    ]
}

fn tabs() -> Vec<TabRect> {
    vec![
        TabRect {
            rect: PixelRect::new(0, 0, 100, 30),
            close: None,
        },
        TabRect {
            rect: PixelRect::new(104, 0, 100, 30),
            close: None,
        },
    ]
}

fn rects(target: DropTarget) -> Vec<PixelRect> {
    drop_preview(&target, &panes(), &tabs(), COLOR)
        .into_iter()
        .map(|q| q.rect)
        .collect()
}

#[test]
fn pane_preview_covers_the_chosen_half() {
    let half = |side| {
        rects(DropTarget::Pane {
            term: TermId(2),
            side,
        })
    };
    assert_eq!(half(Side::Left), vec![PixelRect::new(210, 0, 100, 101)]);
    assert_eq!(half(Side::Right), vec![PixelRect::new(310, 0, 101, 101)]);
    assert_eq!(half(Side::Top), vec![PixelRect::new(210, 0, 201, 50)]);
    assert_eq!(half(Side::Bottom), vec![PixelRect::new(210, 50, 201, 51)]);
    let quads = drop_preview(
        &DropTarget::Pane {
            term: TermId(1),
            side: Side::Left,
        },
        &panes(),
        &tabs(),
        COLOR,
    );
    // Translucide : le contenu du panneau reste lisible dessous.
    assert!(quads.iter().all(|q| q.color.a < 1.0));
}

#[test]
fn tab_slot_preview_is_a_vertical_bar_between_tabs() {
    let between = rects(DropTarget::TabSlot(1));
    assert_eq!(between.len(), 1);
    let bar = between[0];
    assert_eq!(bar.width, MARK_WIDTH);
    assert_eq!((bar.y, bar.height), (0, 30));
    // Dans l'espace entre le premier onglet (fin à 100) et le second (début à 104).
    assert!(bar.x >= 100 && bar.x + bar.width <= 104, "{bar:?}");
    let first = rects(DropTarget::TabSlot(0))[0];
    assert_eq!(first.x, 0);
    let last = rects(DropTarget::TabSlot(2))[0];
    assert_eq!(last.x + last.width, 204);
}

#[test]
fn on_tab_preview_outlines_the_tab() {
    let outline = rects(DropTarget::OnTab(1));
    assert_eq!(outline.len(), 4);
    let tab = PixelRect::new(104, 0, 100, 30);
    for r in &outline {
        assert!(r.x >= tab.x && r.x + r.width <= tab.x + tab.width, "{r:?}");
        assert!(
            r.y >= tab.y && r.y + r.height <= tab.y + tab.height,
            "{r:?}"
        );
    }
}

#[test]
fn no_preview_without_target() {
    assert!(rects(DropTarget::None).is_empty());
    // Cibles qui ne désignent plus rien : pas d'aperçu, pas de panique.
    assert!(
        rects(DropTarget::Pane {
            term: TermId(9),
            side: Side::Top,
        })
        .is_empty()
    );
    assert!(rects(DropTarget::OnTab(5)).is_empty());
    assert!(rects(DropTarget::TabSlot(5)).is_empty());
    assert!(drop_preview(&DropTarget::TabSlot(0), &panes(), &[], COLOR).is_empty());
}
