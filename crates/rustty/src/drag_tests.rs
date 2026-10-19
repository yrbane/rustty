//! Tests de la machine de glisser pure.

use std::time::{Duration, Instant};

use rustty_layout::Side;
use rustty_render::{PixelRect, TabRect};

use super::*;
use crate::tab::TermId;

#[test]
fn drop_side_follows_the_diagonals() {
    let r = PixelRect {
        x: 0,
        y: 0,
        width: 200,
        height: 100,
    };
    assert_eq!(drop_side(r, 100.0, 10.0), Side::Top);
    assert_eq!(drop_side(r, 100.0, 90.0), Side::Bottom);
    assert_eq!(drop_side(r, 10.0, 50.0), Side::Left);
    assert_eq!(drop_side(r, 190.0, 50.0), Side::Right);
}

#[test]
fn drop_side_respects_the_rect_origin() {
    let r = PixelRect::new(100, 50, 200, 100);
    assert_eq!(drop_side(r, 110.0, 100.0), Side::Left);
    assert_eq!(drop_side(r, 200.0, 55.0), Side::Top);
}

#[test]
fn small_moves_stay_a_click() {
    let mut d = Drag::press(DragSource::Tab(0), 0, 10.0, 10.0);
    assert!(!d.moved(13.0, 10.0));
    assert!(!d.is_active());
    assert!(d.moved(15.0, 10.0));
    assert!(d.is_active());
    assert_eq!(Drag::press(DragSource::Tab(0), 0, 0.0, 0.0).finish(), None);
}

#[test]
fn threshold_is_crossed_only_once() {
    let mut d = Drag::press(DragSource::Tab(0), 0, 0.0, 0.0);
    assert!(d.moved(10.0, 0.0));
    assert!(!d.moved(20.0, 0.0), "déjà actif");
}

#[test]
fn finish_returns_source_and_target_once_active() {
    let mut d = Drag::press(DragSource::Pane(TermId(3)), 1, 0.0, 0.0);
    d.moved(50.0, 0.0);
    let target = DropTarget::Pane {
        term: TermId(4),
        side: Side::Left,
    };
    d.set_target(target, Instant::now());
    assert_eq!(d.start_tab(), 1);
    assert_eq!(d.finish(), Some((DragSource::Pane(TermId(3)), target)));
}

#[test]
fn hovering_a_tab_half_a_second_switches_once() {
    let t0 = Instant::now();
    let mut d = Drag::press(DragSource::Pane(TermId(1)), 0, 0.0, 0.0);
    d.moved(50.0, 0.0);
    d.set_target(DropTarget::OnTab(2), t0);
    assert_eq!(d.due_tab_switch(t0 + Duration::from_millis(499)), None);
    assert_eq!(d.due_tab_switch(t0 + TAB_HOVER_DELAY), Some(2));
    assert_eq!(d.deadline(), Some(t0 + TAB_HOVER_DELAY));
}

#[test]
fn tab_switch_fires_once_per_hover() {
    let t0 = Instant::now();
    let mut d = Drag::press(DragSource::Pane(TermId(1)), 0, 0.0, 0.0);
    d.moved(50.0, 0.0);
    d.set_target(DropTarget::OnTab(2), t0);
    d.set_target(DropTarget::OnTab(2), t0 + Duration::from_millis(300));
    assert_eq!(d.deadline(), Some(t0 + TAB_HOVER_DELAY), "même survol");
    let late = t0 + Duration::from_secs(1);
    assert_eq!(d.due_tab_switch(late), Some(2));
    d.mark_switched();
    assert_eq!(d.due_tab_switch(late), None);
    assert_eq!(d.deadline(), None);
}

#[test]
fn leaving_the_tab_resets_the_hover() {
    let t0 = Instant::now();
    let mut d = Drag::press(DragSource::Pane(TermId(1)), 0, 0.0, 0.0);
    d.moved(50.0, 0.0);
    d.set_target(DropTarget::OnTab(2), t0);
    d.set_target(DropTarget::None, t0 + Duration::from_millis(100));
    assert_eq!(d.deadline(), None);
    assert_eq!(d.due_tab_switch(t0 + Duration::from_secs(2)), None);
}

fn three_tabs() -> Vec<TabRect> {
    (0..3)
        .map(|i| TabRect {
            rect: PixelRect::new(i * 100, 0, 100, 30),
            close: None,
        })
        .collect()
}

#[test]
fn tab_slot_uses_tab_midpoints() {
    let tabs = three_tabs();
    assert_eq!(tab_slot(&tabs, 40.0), 0);
    assert_eq!(tab_slot(&tabs, 60.0), 1);
    assert_eq!(tab_slot(&tabs, 290.0), 3);
    assert_eq!(tab_slot(&[], 10.0), 0);
}
