//! Couleurs de cellule telles que vues par l'émulation : la résolution en RGB
//! réel (palette, couleurs par défaut) appartient au renderer.

/// Couleur d'avant-plan ou d'arrière-plan d'une cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Color {
    /// Couleur par défaut du terminal (configurable).
    #[default]
    Default,
    /// Index 0..=255 dans la palette.
    Indexed(u8),
    /// Couleur vraie.
    Rgb(u8, u8, u8),
}
