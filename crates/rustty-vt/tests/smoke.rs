//! Test de fumée : la crate se compile et expose sa version.

#[test]
fn version_is_the_workspace_prerelease() {
    assert!(
        rustty_vt::VERSION.starts_with("0.1.0-alpha."),
        "got {}",
        rustty_vt::VERSION
    );
}
