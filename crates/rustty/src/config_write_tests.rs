use super::*;

const SAMPLE: &str = "# mon terminal\n[window]\nopacity = 0.9 # translucide\n\n[titlebar]\n# fond\nbackground = \"#141414\"\nheight = 28\n";

#[test]
fn the_background_key_is_replaced_in_place_and_nothing_else_moves() {
    let out = with_titlebar_background(SAMPLE, Rgba8::new(0xcc, 0x00, 0x00, 0xff)).unwrap();
    assert_eq!(out, SAMPLE.replace("\"#141414\"", "\"#cc0000\""));
}

#[test]
fn a_missing_key_or_section_is_added() {
    let blue = Rgba8::new(0, 0, 0xff, 0xff);
    let out = with_titlebar_background("[titlebar]\nheight = 28\n", blue).unwrap();
    assert!(out.contains("background = \"#0000ff\""), "{out}");
    let out = with_titlebar_background("[window]\nopacity = 0.9\n", blue).unwrap();
    assert!(out.starts_with("[window]\nopacity = 0.9\n"), "{out}");
    assert!(
        out.contains("[titlebar]\nbackground = \"#0000ff\""),
        "{out}"
    );
    let out = with_titlebar_background("", Rgba8::new(1, 2, 3, 0x80)).unwrap();
    assert!(out.contains("\"#01020380\""), "{out}");
}

#[test]
fn an_inline_titlebar_table_keeps_its_form() {
    let out =
        with_titlebar_background("titlebar = { enabled = true }\n", Rgba8::new(0, 0, 0, 0xff))
            .unwrap();
    assert!(
        out.contains("titlebar = { enabled = true, background = \"#000000\" }"),
        "{out}"
    );
    assert!(rustty_config::Config::from_str(&out).is_ok());
}

#[test]
fn an_invalid_file_is_left_alone() {
    assert!(with_titlebar_background("[titlebar\n", Rgba8::new(0, 0, 0, 0xff)).is_err());
}

#[test]
fn atomic_write_replaces_the_file_and_leaves_no_temporary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rustty.toml");
    std::fs::write(&path, "old").unwrap();
    write_atomically(&path, "new").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn a_failed_write_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent").join("rustty.toml");
    assert!(write_atomically(&path, "x").is_err());
    assert!(!dir.path().join("absent").exists());
}

#[test]
fn palette_colors_round_trip_to_rgba8() {
    let c = Rgba8::new(0x3d, 0xc8, 0x4f, 0xff);
    assert_eq!(rgba8_of(rustty_render::Rgba::from_rgba8(c)), c);
}

#[test]
fn saving_rewrites_only_the_background_of_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rustty.toml");
    std::fs::write(&path, "[titlebar]\nheight = 30 # haute\n").unwrap();
    save_background(&path, Rgba8::new(0x11, 0x22, 0x33, 0xff)).unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "[titlebar]\nheight = 30 # haute\nbackground = \"#112233\"\n"
    );
}

#[test]
fn saving_creates_a_missing_file_from_the_template_first() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("rustty.toml");
    save_background(&path, Rgba8::new(0x11, 0x22, 0x33, 0xff)).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("background = \"#112233\""));
    assert!(
        text.contains("[keys]"),
        "le modèle complet, pas un fichier vide"
    );
}

#[test]
fn a_titlebar_value_that_is_not_a_table_is_an_error_not_a_panic() {
    let black = Rgba8::new(0, 0, 0, 0xff);
    assert!(with_titlebar_background("titlebar = 3\n", black).is_err());
    assert!(with_titlebar_background("titlebar = \"x\"\n", black).is_err());
    assert!(with_titlebar_background("[[titlebar]]\nheight = 28\n", black).is_err());
}

#[cfg(unix)]
#[test]
fn writing_through_a_symlink_updates_the_target_and_keeps_the_link() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("dotfiles-rustty.toml");
    let link = dir.path().join("rustty.toml");
    std::fs::write(&real, "old").unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    write_atomically(&link, "new").unwrap();
    assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
    assert_eq!(std::fs::read_to_string(&real).unwrap(), "new");
}

#[cfg(unix)]
#[test]
fn writing_keeps_the_permissions_of_the_original_file() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rustty.toml");
    std::fs::write(&path, "old").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    write_atomically(&path, "new").unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn the_parse_error_banner_is_one_line() {
    let err = with_titlebar_background("[titlebar\nheight = 28\n", Rgba8::new(0, 0, 0, 0xff))
        .unwrap_err();
    let text = err.to_string();
    assert!(text.starts_with("configuration invalide : "), "{text}");
    assert!(!text.contains('\n'), "{text}");
}

#[test]
fn a_rename_failure_removes_the_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("rustty.toml");
    std::fs::create_dir(&target).unwrap();
    // Dossier non vide : le renommage par-dessus échoue après l'écriture du temporaire.
    std::fs::write(target.join("occupant"), "").unwrap();
    assert!(write_atomically(&target, "x").is_err());
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        1,
        "pas de .tmp-<pid>"
    );
}
