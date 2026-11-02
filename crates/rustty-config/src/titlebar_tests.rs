use super::*;
use crate::color::Rgba8;
use crate::config::Config;

#[test]
fn defaults_match_the_mockup() {
    let t = TitleBar::default();
    assert!(t.enabled);
    assert_eq!(
        (t.height, t.padding, t.spacing, t.resize_border),
        (28, 6, 4, 6)
    );
    assert_eq!(t.background, Rgba8::new(0x14, 0x14, 0x14, 0xff));
    assert_eq!(t.gloss, 0.10);
    let close = t.button_style(TitleButton::Close);
    assert_eq!((close.size, close.radius, close.border_width), (14, 7, 1));
    assert_eq!(close.background, Rgba8::new(0xff, 0x5f, 0x57, 0xff));
    assert_eq!(close.hover_background, close.background);
    assert_eq!(close.icon_color, Rgba8::new(0, 0, 0, 0));
    assert_eq!(close.hover_icon_color, Rgba8::new(0x11, 0x11, 0x11, 0xff));
    let maximize = t.button_style(TitleButton::Maximize);
    assert_eq!(maximize.background, Rgba8::new(0x3d, 0xc8, 0x4f, 0xff));
    let settings = t.button_style(TitleButton::Settings);
    assert_eq!(settings, t.button, "sans surcharge, le style commun");
    assert!(t.button_visible(TitleButton::Settings));
}

#[test]
fn overrides_only_what_they_name() {
    let c = Config::from_str(
        "[titlebar.button]\nradius = 2\n[titlebar.buttons.close]\nbackground = \"#ff0000\"\n",
    )
    .unwrap();
    let close = c.titlebar.button_style(TitleButton::Close);
    assert_eq!(
        close.radius, 7,
        "le préréglage de la pastille garde son rayon"
    );
    assert_eq!(close.background, Rgba8::new(255, 0, 0, 255));
    let settings = c.titlebar.button_style(TitleButton::Settings);
    assert_eq!(
        settings.radius, 2,
        "le style commun atteint les autres boutons"
    );
    let c = Config::from_str("[titlebar.buttons.close]\nvisible = false\n").unwrap();
    assert!(!c.titlebar.button_visible(TitleButton::Close));
    assert!(c.titlebar.button_visible(TitleButton::Minimize));
}

#[test]
fn bounds_are_validated_with_the_key_name() {
    for (toml, key) in [
        ("[titlebar]\nheight = 8\n", "titlebar.height"),
        ("[titlebar]\ngloss = 1.5\n", "titlebar.gloss"),
        ("[titlebar.button]\nsize = 40\n", "titlebar.button.size"),
        (
            "[titlebar.buttons.close]\nradius = 99\n",
            "titlebar.buttons.close.radius",
        ),
        ("[titlebar]\nresize_border = 17\n", "titlebar.resize_border"),
    ] {
        let err = Config::from_str(toml).unwrap_err().to_string();
        assert!(err.contains(key), "{toml}: {err}");
    }
    assert!(
        Config::from_str("[titlebar]\nvisible = true\n").is_err(),
        "clé inconnue"
    );
}
