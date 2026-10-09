//! `rustty img` : lit une image, la réduit si besoin et l'émet en séquences
//! graphiques kitty (`a=T`, PNG, base64 par morceaux de 4096 caractères).

use std::io::{Cursor, Write};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use image::{DynamicImage, ImageFormat, imageops::FilterType};

use crate::img_fetch::read_source;

/// Côté maximal envoyé au terminal.
const MAX_SIDE: u32 = 2048;
/// Taille d'un morceau base64 (limite du protocole).
const CHUNK: usize = 4096;

/// Lit `source`, décode et écrit les séquences ; rien n'est écrit en cas d'erreur.
pub fn run(source: &str, out: &mut impl Write) -> Result<(), String> {
    let bytes = read_source(source)?;
    let img =
        image::load_from_memory(&bytes).map_err(|e| format!("image illisible ({source}) : {e}"))?;
    let data = encode_for_terminal(img)?;
    out.write_all(&data)
        .and_then(|()| out.flush())
        .map_err(|e| format!("écriture impossible : {e}"))
}

/// Réduit à `MAX_SIDE`, encode en PNG et découpe en séquences APC, `\n` final compris.
pub fn encode_for_terminal(img: DynamicImage) -> Result<Vec<u8>, String> {
    let img = if img.width().max(img.height()) > MAX_SIDE {
        img.resize(MAX_SIDE, MAX_SIDE, FilterType::Lanczos3)
    } else {
        img
    };
    let mut png = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| format!("encodage PNG impossible : {e}"))?;
    let b64 = STANDARD.encode(png);
    let chunks: Vec<&[u8]> = b64.as_bytes().chunks(CHUNK).collect();
    let last = chunks.len() - 1;
    let mut out = Vec::with_capacity(b64.len() + chunks.len() * 24 + 1);
    for (i, chunk) in chunks.iter().enumerate() {
        let header = match (i, last) {
            (0, 0) => "a=T,f=100,q=2",
            (0, _) => "a=T,f=100,q=2,m=1",
            (i, last) if i == last => "m=0",
            _ => "m=1",
        };
        out.extend_from_slice(b"\x1b_G");
        out.extend_from_slice(header.as_bytes());
        out.push(b';');
        out.extend_from_slice(chunk);
        out.extend_from_slice(b"\x1b\\");
    }
    out.push(b'\n');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn plain(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, Rgba([200, 30, 30, 255])))
    }

    fn noisy(w: u32, h: u32) -> DynamicImage {
        let mut seed = 12345u32;
        DynamicImage::ImageRgba8(RgbaImage::from_fn(w, h, |_, _| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let b = seed.to_be_bytes();
            Rgba([b[0], b[1], b[2], 255])
        }))
    }

    /// Les séquences APC (sans `\n` final) : (en-tête, charge).
    fn sequences(out: &[u8]) -> Vec<(String, String)> {
        let text = std::str::from_utf8(out).unwrap().trim_end_matches('\n');
        text.split("\x1b\\")
            .filter(|s| !s.is_empty())
            .map(|s| {
                let body = s.strip_prefix("\x1b_G").expect("préfixe APC");
                let (h, d) = body.split_once(';').expect("séparateur");
                (h.to_string(), d.to_string())
            })
            .collect()
    }

    #[test]
    fn small_image_is_one_chunk() {
        let seqs = sequences(&encode_for_terminal(plain(4, 4)).unwrap());
        assert_eq!(seqs.len(), 1);
        assert_eq!(seqs[0].0, "a=T,f=100,q=2");
    }

    #[test]
    fn large_payload_is_split_into_chunks_with_m_flags() {
        let seqs = sequences(&encode_for_terminal(noisy(600, 600)).unwrap());
        assert!(seqs.len() > 2);
        assert_eq!(seqs[0].0, "a=T,f=100,q=2,m=1");
        for (h, d) in &seqs[1..seqs.len() - 1] {
            assert_eq!(h, "m=1");
            assert_eq!(d.len(), CHUNK);
        }
        let (h, d) = seqs.last().unwrap();
        assert_eq!(h, "m=0");
        assert!(d.len() <= CHUNK);
    }

    #[test]
    fn huge_image_is_downscaled() {
        let seqs = sequences(&encode_for_terminal(plain(3000, 10)).unwrap());
        let b64: String = seqs.iter().map(|(_, d)| d.as_str()).collect();
        let png = STANDARD.decode(b64).unwrap();
        let img = image::load_from_memory(&png).unwrap();
        assert_eq!(img.width(), 2048);
        assert!(img.height() >= 1);
    }

    #[test]
    fn output_ends_with_a_newline() {
        assert!(
            encode_for_terminal(plain(2, 2))
                .unwrap()
                .ends_with(b"\x1b\\\n")
        );
    }

    #[test]
    fn unreadable_source_emits_nothing() {
        let mut out = Vec::new();
        assert!(run("/nonexistent/rustty.png", &mut out).is_err());
        let text = std::env::temp_dir().join("rustty-img-not-an-image.txt");
        std::fs::write(&text, "pas une image").unwrap();
        assert!(run(text.to_str().unwrap(), &mut out).is_err());
        let _ = std::fs::remove_file(text);
        assert!(out.is_empty());
    }

    #[test]
    fn a_png_file_round_trips_through_run() {
        let path = std::env::temp_dir().join("rustty-img-run-test.png");
        plain(8, 8).save(&path).unwrap();
        let mut out = Vec::new();
        run(path.to_str().unwrap(), &mut out).unwrap();
        let _ = std::fs::remove_file(path);
        assert!(out.starts_with(b"\x1b_Ga=T,f=100,q=2;"));
    }
}
