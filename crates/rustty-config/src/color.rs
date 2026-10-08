//! Couleur concrète de la configuration, écrite `#rrggbb`.

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("couleur invalide « {0} » : attendu #rrggbb")]
pub struct ColorParseError(pub String);

impl FromStr for Rgb {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ColorParseError(s.to_string());
        let hex = s.strip_prefix('#').ok_or_else(err)?;
        if hex.len() != 6 || !hex.is_ascii() {
            return Err(err());
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| err());
        Ok(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_six_digit_hex_in_any_case() {
        assert_eq!(
            "#1e1e2e".parse::<Rgb>().unwrap(),
            Rgb::new(0x1e, 0x1e, 0x2e)
        );
        assert_eq!("#FFAA00".parse::<Rgb>().unwrap(), Rgb::new(255, 170, 0));
    }

    #[test]
    fn rejects_other_shapes_with_the_offending_text() {
        for bad in [
            "1e1e2e", "#fff", "#12345", "#1234567", "#gg0000", "", "rouge",
        ] {
            let err = bad.parse::<Rgb>().unwrap_err();
            assert!(err.to_string().contains(bad), "{err}");
        }
    }

    #[test]
    fn displays_as_lowercase_hex() {
        assert_eq!(Rgb::new(255, 170, 0).to_string(), "#ffaa00");
    }

    #[test]
    fn deserializes_from_a_toml_string() {
        #[derive(serde::Deserialize, Debug)]
        struct Doc {
            c: Rgb,
        }
        let d: Doc = toml::from_str("c = \"#89b4fa\"").unwrap();
        assert_eq!(d.c, Rgb::new(0x89, 0xb4, 0xfa));
        let err = toml::from_str::<Doc>("c = \"#zz\"").unwrap_err();
        assert!(err.to_string().contains("#zz"));
    }
}
