//! Couleurs choisies par la configuration pour les onglets et les barres de
//! split : fixes, dérivées de la palette, ou tirées au sort. Pur.

use rustty_config::{Splits, Tabs};
use rustty_layout::SplitId;
use rustty_render::{Palette, Rgba};

use crate::accent::accent_color;
use crate::tab::Tab;

/// Les accents des onglets quand `[tabs.colors] random = true`, sinon `None`.
pub fn tab_accents(tabs_config: &Tabs, palette: &Palette, tabs: &[Tab]) -> Option<Vec<Rgba>> {
    tabs_config.colors.random.then(|| {
        tabs.iter()
            .map(|t| accent_color(palette, t.accent))
            .collect()
    })
}

/// La couleur de la barre de la division `split` de l'onglet `tab`.
pub fn divider_color(splits: &Splits, palette: &Palette, tab: &Tab, split: SplitId) -> Rgba {
    if splits.random_colors {
        accent_color(palette, tab.split_accent(split))
    } else {
        splits.color.map_or(palette.ansi[8], Rgba::from_rgb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tab::TermId;
    use rustty_config::{Colors, Rgb, TabColors};
    use rustty_layout::Axis;

    fn palette() -> Palette {
        Palette::from_config(&Colors::default(), false)
    }

    #[test]
    fn tab_accents_only_in_random_mode() {
        let tabs = vec![Tab::new(TermId(1), 0), Tab::new(TermId(2), 3)];
        assert_eq!(tab_accents(&Tabs::default(), &palette(), &tabs), None);
        let random = Tabs {
            colors: TabColors {
                random: true,
                ..TabColors::default()
            },
            ..Tabs::default()
        };
        let accents = tab_accents(&random, &palette(), &tabs).unwrap();
        assert_eq!(
            accents,
            vec![accent_color(&palette(), 0), accent_color(&palette(), 3)]
        );
    }

    #[test]
    fn divider_color_is_fixed_configured_or_random() {
        let mut tab = Tab::new(TermId(1), 0);
        tab.split(Axis::Vertical, TermId(2), 5);
        let window = tab.window_of(TermId(2)).unwrap();
        let split = SplitId(window.0);
        let p = palette();
        let mut splits = Splits::default();
        assert_eq!(divider_color(&splits, &p, &tab, split), p.ansi[8]);
        splits.color = Some(Rgb::new(255, 0, 0));
        assert_eq!(
            divider_color(&splits, &p, &tab, split),
            Rgba::from_rgb(Rgb::new(255, 0, 0))
        );
        splits.random_colors = true;
        assert_eq!(divider_color(&splits, &p, &tab, split), accent_color(&p, 5));
    }
}
