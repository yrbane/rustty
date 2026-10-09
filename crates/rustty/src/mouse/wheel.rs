//! Accumulation des crans de molette fractionnaires (pavé tactile, défilement
//! au pixel) en lignes entières, sans en perdre.

#[derive(Debug, Default)]
pub struct WheelAccumulator {
    remainder: f64,
}

impl WheelAccumulator {
    /// Ajoute `delta` (en lignes, fractionnaire) et rend les lignes entières
    /// disponibles ; le reste attend le prochain cran.
    pub fn lines(&mut self, delta: f64) -> i32 {
        self.remainder += delta;
        let whole = self.remainder.trunc();
        self.remainder -= whole;
        whole as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_deltas_add_up_to_whole_lines() {
        let mut w = WheelAccumulator::default();
        assert_eq!(
            (w.lines(0.3), w.lines(0.3), w.lines(0.3), w.lines(0.3)),
            (0, 0, 0, 1)
        );
    }

    #[test]
    fn large_deltas_keep_their_remainder() {
        let mut w = WheelAccumulator::default();
        assert_eq!(w.lines(-2.6), -2);
        assert_eq!(w.lines(-0.5), -1, "-0.6 restant + -0.5 = -1.1");
        assert_eq!(w.lines(3.0), 2, "-0.1 restant + 3.0 = 2.9");
    }

    #[test]
    fn direction_changes_cancel_out() {
        let mut w = WheelAccumulator::default();
        assert_eq!((w.lines(0.5), w.lines(-0.5), w.lines(0.0)), (0, 0, 0));
    }
}
