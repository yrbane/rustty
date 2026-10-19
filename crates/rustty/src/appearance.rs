//! Couleurs choisies par la configuration pour les onglets et les barres de
//! split : fixes, dérivées de la palette, ou tirées au sort. Pur.

use rustty_config::{Splits, Tabs};
use rustty_layout::SplitId;
use rustty_render::{Palette, Rgba};

use crate::accent::accent_color;
use crate::tab::{Tab, TermId};

/// La couleur de chaque onglet : celle choisie au nuancier (même sans `random`),
/// sinon l'accent tiré quand `[tabs.colors] random = true`, sinon `None` (style
/// de la configuration).
pub fn tab_accents(tabs_config: &Tabs, palette: &Palette, tabs: &[Tab]) -> Vec<Option<Rgba>> {
    tabs.iter()
        .map(|t| match t.color {
            Some(ansi) => Some(palette.ansi[ansi % palette.ansi.len()]),
            None => tabs_config
                .colors
                .random
                .then(|| accent_color(palette, t.accent)),
        })
        .collect()
}

/// La couleur de la barre de la division `split` de l'onglet `tab`.
pub fn divider_color(splits: &Splits, palette: &Palette, tab: &Tab, split: SplitId) -> Rgba {
    if splits.random_colors {
        accent_color(palette, tab.split_accent(split))
    } else {
        splits.color.map_or(palette.ansi[8], Rgba::from_rgb)
    }
}

/// La couleur du bandeau du panneau `term` : celle de sa division parente, ou
/// celle des divisions de la configuration quand il n'a pas de parent.
pub fn header_color(splits: &Splits, palette: &Palette, tab: &Tab, term: TermId) -> Rgba {
    match tab.parent_split_of(term) {
        Some(split) => divider_color(splits, palette, tab, split),
        None => splits.color.map_or(palette.ansi[8], Rgba::from_rgb),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustty_config::{Colors, Rgb, TabColors};
    use rustty_layout::Axis;

    fn palette() -> Palette {
        Palette::from_config(&Colors::default(), false)
    }

    fn random_config() -> Tabs {
        Tabs {
            colors: TabColors {
                random: true,
                ..TabColors::default()
            },
            ..Tabs::default()
        }
    }

    #[test]
    fn tab_accents_only_in_random_mode() {
        let tabs = vec![Tab::new(TermId(1), 0), Tab::new(TermId(2), 3)];
        assert_eq!(
            tab_accents(&Tabs::default(), &palette(), &tabs),
            vec![None, None]
        );
        let accents = tab_accents(&random_config(), &palette(), &tabs);
        assert_eq!(
            accents,
            vec![
                Some(accent_color(&palette(), 0)),
                Some(accent_color(&palette(), 3))
            ]
        );
    }

    #[test]
    fn chosen_color_shows_even_without_random() {
        let mut tabs = vec![Tab::new(TermId(1), 0), Tab::new(TermId(2), 3)];
        tabs[1].color = Some(4);
        let p = palette();
        assert_eq!(
            tab_accents(&Tabs::default(), &p, &tabs),
            vec![None, Some(p.ansi[4])]
        );
        // Le choix l'emporte aussi sur le tirage au sort.
        assert_eq!(tab_accents(&random_config(), &p, &tabs)[1], Some(p.ansi[4]));
    }

    #[test]
    fn default_color_follows_config() {
        let tabs = vec![Tab::new(TermId(1), 2)];
        assert_eq!(tab_accents(&Tabs::default(), &palette(), &tabs), vec![None]);
    }

    #[test]
    fn header_takes_its_parent_split_color() {
        let mut tab = Tab::new(TermId(1), 0);
        tab.split(Axis::Vertical, TermId(2), 5);
        tab.split(Axis::Horizontal, TermId(3), 8);
        let p = palette();
        let mut splits = Splits {
            random_colors: true,
            ..Splits::default()
        };
        let of = |t| header_color(&splits, &p, &tab, TermId(t));
        assert_eq!(of(1), accent_color(&p, 5));
        assert_eq!(of(2), accent_color(&p, 8));
        assert_eq!(of(3), accent_color(&p, 8));
        splits.random_colors = false;
        assert_eq!(header_color(&splits, &p, &tab, TermId(1)), p.ansi[8]);
    }

    #[test]
    fn header_without_parent_split_uses_configured_color() {
        let tab = Tab::new(TermId(1), 0);
        let p = palette();
        let mut splits = Splits {
            random_colors: true,
            ..Splits::default()
        };
        assert_eq!(header_color(&splits, &p, &tab, TermId(1)), p.ansi[8]);
        splits.color = Some(Rgb::new(1, 2, 3));
        assert_eq!(
            header_color(&splits, &p, &tab, TermId(1)),
            Rgba::from_rgb(Rgb::new(1, 2, 3))
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
