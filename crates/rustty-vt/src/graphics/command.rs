//! Analyse d'une commande graphique kitty (`G<clés>;<base64>`).

/// Action demandée (`a=`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `a=t` : transmettre sans afficher (défaut, comme kitty).
    Transmit,
    /// `a=T` : transmettre puis afficher.
    TransmitAndPut,
    /// `a=p` : afficher une image déjà transmise.
    Put,
    /// `a=d` : supprimer.
    Delete,
}

/// Format des pixels transmis (`f=`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// `f=100`.
    Png,
    /// `f=32` (défaut).
    Rgba,
    /// `f=24`.
    Rgb,
}

/// Commande graphique analysée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsCommand {
    pub action: Action,
    pub format: Format,
    pub id: Option<u32>,
    /// `s=` : largeur en pixels.
    pub width: Option<u32>,
    /// `v=` : hauteur en pixels.
    pub height: Option<u32>,
    /// `c=` : colonnes d'affichage.
    pub cols: Option<u16>,
    /// `r=` : lignes d'affichage.
    pub rows: Option<u16>,
    /// `m=1` : d'autres morceaux suivent.
    pub more: bool,
    /// `q=` : niveau de silence (0, 1 ou 2).
    pub quiet: u8,
    /// `d=` : cible de suppression (`'a'` par défaut).
    pub delete: char,
    /// `t=` : support de transmission (`'d'` direct par défaut).
    pub medium: char,
    /// `o=z` : charge compressée.
    pub compressed: bool,
    /// Vrai si `a=` ou `f=` porte une valeur inconnue : `Chunks::push`
    /// répond alors `Invalid` (choix documenté : l'analyse ne rejette pas).
    pub invalid: bool,
}

impl Default for GraphicsCommand {
    fn default() -> Self {
        Self {
            action: Action::Transmit,
            format: Format::Rgba,
            id: None,
            width: None,
            height: None,
            cols: None,
            rows: None,
            more: false,
            quiet: 0,
            delete: 'a',
            medium: 'd',
            compressed: false,
            invalid: false,
        }
    }
}

/// Analyse une chaîne APC ; `None` si elle ne commence pas par `G`.
/// Rend la commande et la charge base64 brute (après le premier `;`).
pub fn parse(apc: &[u8]) -> Option<(GraphicsCommand, &[u8])> {
    let rest = apc.strip_prefix(b"G")?;
    let (control, payload) = match rest.iter().position(|&b| b == b';') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, &rest[rest.len()..]),
    };
    let mut cmd = GraphicsCommand::default();
    for pair in control.split(|&b| b == b',') {
        let Some(eq) = pair.iter().position(|&b| b == b'=') else {
            continue;
        };
        if eq != 1 {
            continue; // les clés sont d'un seul caractère
        }
        let value = std::str::from_utf8(&pair[2..]).unwrap_or("");
        apply(&mut cmd, pair[0], value);
    }
    Some((cmd, payload))
}

fn apply(cmd: &mut GraphicsCommand, key: u8, value: &str) {
    let ch = value.chars().next();
    match key {
        b'a' => match ch {
            Some('t') => cmd.action = Action::Transmit,
            Some('T') => cmd.action = Action::TransmitAndPut,
            Some('p') => cmd.action = Action::Put,
            Some('d') => cmd.action = Action::Delete,
            _ => cmd.invalid = true,
        },
        b'f' => match value {
            "100" => cmd.format = Format::Png,
            "32" => cmd.format = Format::Rgba,
            "24" => cmd.format = Format::Rgb,
            _ => cmd.invalid = true,
        },
        b'i' => cmd.id = value.parse().ok(),
        b's' => cmd.width = value.parse().ok(),
        b'v' => cmd.height = value.parse().ok(),
        b'c' => cmd.cols = value.parse().ok(),
        b'r' => cmd.rows = value.parse().ok(),
        b'm' => cmd.more = value == "1",
        b'q' => cmd.quiet = value.parse().ok().filter(|q| *q <= 2).unwrap_or(0),
        b'd' => cmd.delete = ch.unwrap_or('a'),
        b't' => cmd.medium = ch.unwrap_or('d'),
        b'o' => cmd.compressed = value == "z",
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keys_and_payload() {
        let (cmd, payload) = parse(b"Ga=T,f=100,i=7,c=10;QUJD").unwrap();
        assert_eq!(cmd.action, Action::TransmitAndPut);
        assert_eq!(cmd.format, Format::Png);
        assert_eq!(cmd.id, Some(7));
        assert_eq!(cmd.cols, Some(10));
        assert_eq!(payload, b"QUJD");
    }

    #[test]
    fn defaults_are_transmit_rgba_direct() {
        let (cmd, payload) = parse(b"G;").unwrap();
        assert_eq!(cmd.action, Action::Transmit);
        assert_eq!(cmd.format, Format::Rgba);
        assert_eq!(cmd.medium, 'd');
        assert_eq!(cmd.delete, 'a');
        assert!(payload.is_empty());
    }

    #[test]
    fn non_graphics_apc_is_ignored() {
        assert!(parse(b"Xabc").is_none());
    }

    #[test]
    fn unknown_keys_are_ignored_and_bad_numbers_reject() {
        let (cmd, _) = parse(b"Gz=1,i=x;").unwrap();
        assert_eq!(cmd.id, None);
        assert!(!cmd.invalid);
    }

    #[test]
    fn unknown_format_is_flagged_invalid() {
        assert!(parse(b"Gf=99;").unwrap().0.invalid);
    }
}
