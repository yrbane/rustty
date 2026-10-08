//! Types géométriques partagés : rectangles en pixels, axes, directions,
//! identifiants de fenêtre.

/// Rectangle en pixels, origine en haut à gauche.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn right(&self) -> u32 {
        self.x + self.width
    }

    pub const fn bottom(&self) -> u32 {
        self.y + self.height
    }

    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Vrai si les intérieurs se chevauchent ; des bords communs ne comptent pas.
    pub const fn intersects(&self, other: &Rect) -> bool {
        self.width > 0
            && self.height > 0
            && other.width > 0
            && other.height > 0
            && self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    pub const fn contains_rect(&self, other: &Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }
}

/// Axe d'une division, convention kitty : `Horizontal` = ligne de séparation
/// horizontale, donc panneaux empilés (premier en haut) ; `Vertical` = côte à
/// côte (premier à gauche).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// L'axe de division qui sépare deux voisins dans cette direction.
    pub const fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Vertical,
            Self::Up | Self::Down => Axis::Horizontal,
        }
    }
}

/// Identifiant d'une fenêtre de terminal au sein d'un onglet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_edges_and_area() {
        let r = Rect::new(10, 20, 30, 40);
        assert_eq!((r.right(), r.bottom()), (40, 60));
        assert_eq!(r.area(), 1200);
    }

    #[test]
    fn rect_intersection_excludes_touching_edges() {
        let a = Rect::new(0, 0, 10, 10);
        assert!(a.intersects(&Rect::new(5, 5, 10, 10)));
        assert!(
            !a.intersects(&Rect::new(10, 0, 10, 10)),
            "bord commun : pas de chevauchement"
        );
        assert!(!a.intersects(&Rect::new(0, 10, 10, 10)));
        assert!(!Rect::new(0, 0, 0, 5).intersects(&a), "rectangle vide");
    }

    #[test]
    fn rect_containment() {
        let outer = Rect::new(0, 0, 100, 100);
        assert!(outer.contains_rect(&Rect::new(0, 0, 100, 100)));
        assert!(outer.contains_rect(&Rect::new(10, 10, 20, 20)));
        assert!(!outer.contains_rect(&Rect::new(90, 90, 20, 20)));
    }

    #[test]
    fn direction_maps_to_axis() {
        assert_eq!(Direction::Left.axis(), Axis::Vertical);
        assert_eq!(Direction::Right.axis(), Axis::Vertical);
        assert_eq!(Direction::Up.axis(), Axis::Horizontal);
        assert_eq!(Direction::Down.axis(), Axis::Horizontal);
    }
}
