use super::*;

#[test]
fn apply_config_toggles_decorations() {
    let on = rustty_config::TitleBar::default();
    let mut off = on.clone();
    off.enabled = false;
    assert!(!decorations_wanted(&on));
    assert!(decorations_wanted(&off));
    assert_eq!(decorations_change(&on, &off), Some(true));
    assert_eq!(decorations_change(&off, &on), Some(false));
    assert_eq!(decorations_change(&on, &on), None);
}
