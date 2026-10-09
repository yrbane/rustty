//! Zoom de police : un point par cran, borné, retour à la taille configurée.

pub const MIN_FONT_SIZE: f32 = 4.0;
pub const MAX_FONT_SIZE: f32 = 72.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontChange {
    Increase,
    Decrease,
    Reset,
}

/// Le sens du zoom pour un événement de molette : un seul cran par
/// événement, quelle que soit son amplitude (chaque cran recharge les polices).
pub fn wheel_change(lines: i32) -> Option<FontChange> {
    match lines.signum() {
        1 => Some(FontChange::Increase),
        -1 => Some(FontChange::Decrease),
        _ => None,
    }
}

/// La nouvelle taille en points ; `base` est celle de la configuration.
pub fn next_size(current: f32, base: f32, change: FontChange) -> f32 {
    // Une taille configurée hors bornes ne doit jamais aller à contre-sens.
    match change {
        FontChange::Increase => (current + 1.0).min(MAX_FONT_SIZE).max(current),
        FontChange::Decrease => (current - 1.0).max(MIN_FONT_SIZE).min(current),
        FontChange::Reset => base.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_wheel_event_is_one_zoom_step() {
        assert_eq!(wheel_change(3), Some(FontChange::Increase));
        assert_eq!(wheel_change(-1), Some(FontChange::Decrease));
        assert_eq!(wheel_change(0), None);
    }

    #[test]
    fn steps_of_one_point() {
        assert_eq!(next_size(11.0, 11.0, FontChange::Increase), 12.0);
        assert_eq!(next_size(11.0, 11.0, FontChange::Decrease), 10.0);
    }

    #[test]
    fn font_size_is_clamped() {
        assert_eq!(
            next_size(MAX_FONT_SIZE, 11.0, FontChange::Increase),
            MAX_FONT_SIZE
        );
        assert_eq!(
            next_size(MIN_FONT_SIZE, 11.0, FontChange::Decrease),
            MIN_FONT_SIZE
        );
        assert_eq!(next_size(4.5, 11.0, FontChange::Decrease), MIN_FONT_SIZE);
        let mut size = 11.0;
        for _ in 0..200 {
            size = next_size(size, 11.0, FontChange::Increase);
        }
        assert_eq!(size, MAX_FONT_SIZE);
        assert_eq!(
            next_size(size, 11.0, FontChange::Decrease),
            MAX_FONT_SIZE - 1.0,
            "un cran suffit pour redescendre"
        );
    }

    #[test]
    fn zoom_never_moves_against_its_direction() {
        assert_eq!(next_size(80.0, 80.0, FontChange::Increase), 80.0);
        assert_eq!(next_size(2.0, 2.0, FontChange::Decrease), 2.0);
    }

    #[test]
    fn reset_returns_to_the_base() {
        assert_eq!(next_size(30.0, 11.0, FontChange::Reset), 11.0);
        assert_eq!(
            next_size(30.0, 200.0, FontChange::Reset),
            MAX_FONT_SIZE,
            "la base elle-même est bornée"
        );
    }
}
