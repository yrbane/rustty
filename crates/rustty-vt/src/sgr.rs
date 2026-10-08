//! Select Graphic Rendition : met à jour le style courant. Gère les deux
//! écritures des couleurs étendues (`38;5;n` et `38:5:n`, idem `2`).

use vte::Params;

use crate::cell::{Attrs, Style};
use crate::color::Color;

pub(crate) fn apply_sgr(style: &mut Style, params: &Params) {
    let groups: Vec<&[u16]> = params.iter().collect();
    if groups.is_empty() {
        *style = Style::default();
        return;
    }
    let mut i = 0;
    while i < groups.len() {
        let g = groups[i];
        let code = g.first().copied().unwrap_or(0);
        match code {
            0 => *style = Style::default(),
            1 => style.attrs.insert(Attrs::BOLD),
            2 => style.attrs.insert(Attrs::DIM),
            3 => style.attrs.insert(Attrs::ITALIC),
            4 => match g.get(1) {
                Some(0) => style.attrs.remove(Attrs::UNDERLINE),
                _ => style.attrs.insert(Attrs::UNDERLINE),
            },
            5 | 6 => style.attrs.insert(Attrs::BLINK),
            7 => style.attrs.insert(Attrs::INVERSE),
            8 => style.attrs.insert(Attrs::HIDDEN),
            9 => style.attrs.insert(Attrs::STRIKETHROUGH),
            22 => style.attrs.remove(Attrs::BOLD | Attrs::DIM),
            23 => style.attrs.remove(Attrs::ITALIC),
            24 => style.attrs.remove(Attrs::UNDERLINE),
            25 => style.attrs.remove(Attrs::BLINK),
            27 => style.attrs.remove(Attrs::INVERSE),
            28 => style.attrs.remove(Attrs::HIDDEN),
            29 => style.attrs.remove(Attrs::STRIKETHROUGH),
            30..=37 => style.fg = Color::Indexed(as_u8(code - 30)),
            39 => style.fg = Color::Default,
            40..=47 => style.bg = Color::Indexed(as_u8(code - 40)),
            49 => style.bg = Color::Default,
            90..=97 => style.fg = Color::Indexed(as_u8(code - 90 + 8)),
            100..=107 => style.bg = Color::Indexed(as_u8(code - 100 + 8)),
            38 | 48 => {
                let (color, consumed) = if g.len() > 1 {
                    // Forme `38:2::r:g:b` ou `38:5:n` : tout est dans le groupe.
                    (extended_color(&g[1..], true), 0)
                } else {
                    // Forme `38;2;r;g;b` : les groupes suivants portent la suite.
                    let rest: Vec<u16> = groups[i + 1..]
                        .iter()
                        .map(|s| s.first().copied().unwrap_or(0))
                        .collect();
                    // Même tronquée, la séquence consomme ses sous-paramètres :
                    // ils ne doivent pas être relus comme des codes SGR.
                    (extended_color(&rest, false), extended_len(&rest))
                };
                if let Some(c) = color {
                    if code == 38 {
                        style.fg = c;
                    } else {
                        style.bg = c;
                    }
                }
                i += consumed;
            }
            _ => {}
        }
        i += 1;
    }
}

fn as_u8(v: u16) -> u8 {
    u8::try_from(v.min(255)).unwrap_or(255)
}

/// Longueur consommée par une couleur étendue en forme `;` : `5;n` = 2, `2;r;g;b` = 4.
fn extended_len(items: &[u16]) -> usize {
    match items.first() {
        Some(5) => 2,
        Some(2) => 4,
        _ => 0,
    }
}

/// `items` commence au sélecteur (2 ou 5). En forme `:`, `2` peut être suivi
/// d'un identifiant d'espace colorimétrique vide : `2::r:g:b`.
fn extended_color(items: &[u16], colon_form: bool) -> Option<Color> {
    match items.first()? {
        5 => items.get(1).map(|&n| Color::Indexed(as_u8(n))),
        2 => {
            let rgb = if colon_form && items.len() >= 5 {
                &items[2..5]
            } else {
                items.get(1..4)?
            };
            Some(Color::Rgb(as_u8(rgb[0]), as_u8(rgb[1]), as_u8(rgb[2])))
        }
        _ => None,
    }
}
