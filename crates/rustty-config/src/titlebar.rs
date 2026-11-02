//! La section `[titlebar]` : la barre de titre que rustty dessine lui-même,
//! son fond, son reflet et le style de chacun de ses boutons. Toutes les
//! tailles sont en pixels logiques.
//!
//! Le style d'un bouton se résout en trois couches : le style commun
//! `[titlebar.button]`, le préréglage du bouton (les pastilles rondes des
//! boutons de fenêtre), puis la table `[titlebar.buttons.<nom>]` de
//! l'utilisateur, qui ne remplace que les clés qu'elle nomme.

use serde::Deserialize;

use crate::color::Rgba8;
use crate::error::ConfigError;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TitleBar {
    /// Barre maison (décorations système retirées) ; `false` rend la main à winit.
    pub enabled: bool,
    /// Hauteur en pixels logiques (16 à 64).
    pub height: u32,
    pub background: Rgba8,
    /// Reflet : liseré clair en haut et dégradé, de 0.0 (plat) à 1.0.
    pub gloss: f32,
    /// Trait de 1 px sous la barre.
    pub separator_color: Rgba8,
    pub title_color: Rgba8,
    /// Marge gauche et droite.
    pub padding: u32,
    /// Espace entre deux boutons.
    pub spacing: u32,
    /// Marge invisible sur le pourtour qui redimensionne la fenêtre (0 à 16).
    pub resize_border: u32,
    /// Style commun des boutons.
    pub button: ButtonStyle,
    /// Surcharges écrites par l'utilisateur, bouton par bouton.
    pub buttons: TitleButtons,
}

impl Default for TitleBar {
    fn default() -> Self {
        Self {
            enabled: true,
            height: 28,
            background: Rgba8::new(0x14, 0x14, 0x14, 0xff),
            gloss: 0.10,
            separator_color: Rgba8::new(0, 0, 0, 0xff),
            title_color: Rgba8::new(0xd8, 0xd8, 0xd8, 0xff),
            padding: 6,
            spacing: 4,
            resize_border: 6,
            button: ButtonStyle::default(),
            buttons: TitleButtons::default(),
        }
    }
}

/// Les six boutons de la barre, dans l'ordre d'affichage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TitleButton {
    Settings,
    Color,
    Folder,
    Minimize,
    Maximize,
    Close,
}

impl TitleButton {
    pub const ALL: [Self; 6] = [
        Self::Settings,
        Self::Color,
        Self::Folder,
        Self::Minimize,
        Self::Maximize,
        Self::Close,
    ];

    /// Réduire, agrandir, fermer : le groupe calé à droite.
    pub const fn is_window_control(self) -> bool {
        matches!(self, Self::Minimize | Self::Maximize | Self::Close)
    }

    /// Préfixe des clés du bouton dans le fichier, pour les messages d'erreur.
    const fn prefix(self) -> &'static str {
        match self {
            Self::Settings => "titlebar.buttons.settings",
            Self::Color => "titlebar.buttons.color",
            Self::Folder => "titlebar.buttons.folder",
            Self::Minimize => "titlebar.buttons.minimize",
            Self::Maximize => "titlebar.buttons.maximize",
            Self::Close => "titlebar.buttons.close",
        }
    }

    /// Préréglage du bouton, entre le style commun et la table de l'utilisateur.
    fn preset(self) -> ButtonOverride {
        match self {
            Self::Minimize => ButtonOverride::window_control(Rgba8::new(0xf6, 0xbd, 0x3b, 0xff)),
            Self::Maximize => ButtonOverride::window_control(Rgba8::new(0x3d, 0xc8, 0x4f, 0xff)),
            Self::Close => ButtonOverride::window_control(Rgba8::new(0xff, 0x5f, 0x57, 0xff)),
            _ => ButtonOverride::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ButtonStyle {
    /// Côté du bouton (8 à `height`).
    pub size: u32,
    pub background: Rgba8,
    pub border_color: Rgba8,
    /// Largeur de bordure (0 à 4).
    pub border_width: u32,
    /// Rayon des coins (0 à 32) ; un rayon ≥ size/2 donne un disque.
    pub radius: u32,
    pub icon_color: Rgba8,
    pub hover_background: Rgba8,
    pub hover_border_color: Rgba8,
    pub hover_icon_color: Rgba8,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        let none = Rgba8::new(0, 0, 0, 0);
        Self {
            size: 22,
            background: none,
            border_color: none,
            border_width: 0,
            radius: 5,
            icon_color: Rgba8::new(0xbd, 0xbd, 0xbd, 0xff),
            hover_background: Rgba8::new(0xff, 0xff, 0xff, 0x1a),
            hover_border_color: none,
            hover_icon_color: Rgba8::new(0xff, 0xff, 0xff, 0xff),
        }
    }
}

/// Surcharge d'un bouton : une clé absente laisse la couche du dessous.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ButtonOverride {
    pub visible: bool,
    pub size: Option<u32>,
    pub background: Option<Rgba8>,
    pub border_color: Option<Rgba8>,
    pub border_width: Option<u32>,
    pub radius: Option<u32>,
    pub icon_color: Option<Rgba8>,
    pub hover_background: Option<Rgba8>,
    pub hover_border_color: Option<Rgba8>,
    pub hover_icon_color: Option<Rgba8>,
}

impl Default for ButtonOverride {
    fn default() -> Self {
        Self {
            visible: true,
            size: None,
            background: None,
            border_color: None,
            border_width: None,
            radius: None,
            icon_color: None,
            hover_background: None,
            hover_border_color: None,
            hover_icon_color: None,
        }
    }
}

impl ButtonOverride {
    /// Pastille ronde de 14 px, bordée, dont l'icône n'apparaît qu'au survol.
    fn window_control(color: Rgba8) -> Self {
        Self {
            size: Some(14),
            radius: Some(7),
            border_width: Some(1),
            border_color: Some(Rgba8::new(0, 0, 0, 0x99)),
            background: Some(color),
            hover_background: Some(color),
            icon_color: Some(Rgba8::new(0, 0, 0, 0)),
            hover_icon_color: Some(Rgba8::new(0x11, 0x11, 0x11, 0xff)),
            ..Self::default()
        }
    }

    pub fn resolve(&self, base: &ButtonStyle) -> ButtonStyle {
        ButtonStyle {
            size: self.size.unwrap_or(base.size),
            background: self.background.unwrap_or(base.background),
            border_color: self.border_color.unwrap_or(base.border_color),
            border_width: self.border_width.unwrap_or(base.border_width),
            radius: self.radius.unwrap_or(base.radius),
            icon_color: self.icon_color.unwrap_or(base.icon_color),
            hover_background: self.hover_background.unwrap_or(base.hover_background),
            hover_border_color: self.hover_border_color.unwrap_or(base.hover_border_color),
            hover_icon_color: self.hover_icon_color.unwrap_or(base.hover_icon_color),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TitleButtons {
    pub settings: ButtonOverride,
    pub color: ButtonOverride,
    pub folder: ButtonOverride,
    pub minimize: ButtonOverride,
    pub maximize: ButtonOverride,
    pub close: ButtonOverride,
}

impl TitleButtons {
    pub fn get(&self, button: TitleButton) -> &ButtonOverride {
        match button {
            TitleButton::Settings => &self.settings,
            TitleButton::Color => &self.color,
            TitleButton::Folder => &self.folder,
            TitleButton::Minimize => &self.minimize,
            TitleButton::Maximize => &self.maximize,
            TitleButton::Close => &self.close,
        }
    }
}

impl TitleBar {
    /// Style résolu d'un bouton : commun, puis préréglage, puis l'utilisateur.
    pub fn button_style(&self, button: TitleButton) -> ButtonStyle {
        let preset = button.preset().resolve(&self.button);
        self.buttons.get(button).resolve(&preset)
    }

    pub fn button_visible(&self, button: TitleButton) -> bool {
        self.buttons.get(button).visible
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        bounded("titlebar.height", self.height, 16..=64)?;
        bounded("titlebar.padding", self.padding, 0..=32)?;
        bounded("titlebar.spacing", self.spacing, 0..=32)?;
        bounded("titlebar.resize_border", self.resize_border, 0..=16)?;
        if !(0.0..=1.0).contains(&self.gloss) {
            return Err(ConfigError::Invalid {
                field: "titlebar.gloss",
                reason: format!("{} n'est pas entre 0.0 et 1.0", self.gloss),
            });
        }
        validate_button("titlebar.button", &self.button, self.height)?;
        for button in TitleButton::ALL {
            validate_button(button.prefix(), &self.button_style(button), self.height)?;
        }
        Ok(())
    }
}

fn bounded(
    field: &'static str,
    value: u32,
    range: std::ops::RangeInclusive<u32>,
) -> Result<(), ConfigError> {
    if range.contains(&value) {
        Ok(())
    } else {
        Err(ConfigError::Invalid {
            field,
            reason: format!(
                "{value} n'est pas entre {} et {}",
                range.start(),
                range.end()
            ),
        })
    }
}

/// Bornes d'un style de bouton, nommées avec le préfixe de sa table.
fn validate_button(
    prefix: &'static str,
    style: &ButtonStyle,
    height: u32,
) -> Result<(), ConfigError> {
    bounded(button_field(prefix, "size"), style.size, 8..=height)?;
    bounded(button_field(prefix, "radius"), style.radius, 0..=32)?;
    bounded(
        button_field(prefix, "border_width"),
        style.border_width,
        0..=4,
    )
}

/// Nom complet d'une clé de bouton, en `&'static str` comme l'exige `ConfigError`.
fn button_field(prefix: &'static str, key: &str) -> &'static str {
    match (prefix, key) {
        ("titlebar.button", "size") => "titlebar.button.size",
        ("titlebar.button", "radius") => "titlebar.button.radius",
        ("titlebar.button", "border_width") => "titlebar.button.border_width",
        ("titlebar.buttons.settings", "size") => "titlebar.buttons.settings.size",
        ("titlebar.buttons.settings", "radius") => "titlebar.buttons.settings.radius",
        ("titlebar.buttons.settings", "border_width") => "titlebar.buttons.settings.border_width",
        ("titlebar.buttons.color", "size") => "titlebar.buttons.color.size",
        ("titlebar.buttons.color", "radius") => "titlebar.buttons.color.radius",
        ("titlebar.buttons.color", "border_width") => "titlebar.buttons.color.border_width",
        ("titlebar.buttons.folder", "size") => "titlebar.buttons.folder.size",
        ("titlebar.buttons.folder", "radius") => "titlebar.buttons.folder.radius",
        ("titlebar.buttons.folder", "border_width") => "titlebar.buttons.folder.border_width",
        ("titlebar.buttons.minimize", "size") => "titlebar.buttons.minimize.size",
        ("titlebar.buttons.minimize", "radius") => "titlebar.buttons.minimize.radius",
        ("titlebar.buttons.minimize", "border_width") => "titlebar.buttons.minimize.border_width",
        ("titlebar.buttons.maximize", "size") => "titlebar.buttons.maximize.size",
        ("titlebar.buttons.maximize", "radius") => "titlebar.buttons.maximize.radius",
        ("titlebar.buttons.maximize", "border_width") => "titlebar.buttons.maximize.border_width",
        ("titlebar.buttons.close", "size") => "titlebar.buttons.close.size",
        ("titlebar.buttons.close", "radius") => "titlebar.buttons.close.radius",
        ("titlebar.buttons.close", "border_width") => "titlebar.buttons.close.border_width",
        _ => "titlebar.buttons",
    }
}

#[cfg(test)]
#[path = "titlebar_tests.rs"]
mod tests;
