//! Images décodées et bandes d'image rattachées aux lignes.

use std::fmt;
use std::io::Cursor;

use image::ImageDecoder;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{Format, GraphicsError};

/// Côté maximal accepté pour une image (pixels).
pub const MAX_SIDE: u32 = 8192;

/// Plafond d'allocation du décodeur (octets) : 8192 × 8192 × 4 = 256 Mio.
const MAX_ALLOC: u64 = 256 * 1024 * 1024;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Image décodée en RGBA 8 bits.
pub struct ImageData {
    /// Identifiant unique au processus (clé de cache côté rendu).
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl ImageData {
    /// `rgba.len()` doit valoir `width * height * 4`.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        debug_assert_eq!(rgba.len(), width as usize * height as usize * 4);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            rgba,
        }
    }
}

impl fmt::Debug for ImageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ImageData#{}({}x{})", self.id, self.width, self.height)
    }
}

/// Décode une charge utile en RGBA.
pub fn decode(
    format: Format,
    width: Option<u32>,
    height: Option<u32>,
    bytes: &[u8],
) -> Result<ImageData, GraphicsError> {
    match format {
        Format::Png => {
            let decoder =
                image::ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png)
                    .into_decoder()
                    .map_err(|e| GraphicsError::Invalid(format!("PNG illisible : {e}")))?;
            // Dimensions lues dans l'en-tête, avant toute allocation de pixels.
            let (w, h) = decoder.dimensions();
            if w > MAX_SIDE || h > MAX_SIDE {
                return Err(GraphicsError::TooBig);
            }
            let mut limits = image::Limits::default();
            limits.max_alloc = Some(MAX_ALLOC);
            let mut decoder = decoder;
            decoder
                .set_limits(limits)
                .map_err(|e| GraphicsError::Invalid(format!("PNG refusé : {e}")))?;
            let img = image::DynamicImage::from_decoder(decoder)
                .map_err(|e| GraphicsError::Invalid(format!("PNG illisible : {e}")))?
                .to_rgba8();
            if w == 0 || h == 0 {
                return Err(GraphicsError::Invalid("image vide".into()));
            }
            Ok(ImageData::new(w, h, img.into_raw()))
        }
        Format::Rgb | Format::Rgba => {
            let (Some(w), Some(h)) = (width, height) else {
                return Err(GraphicsError::Invalid(
                    "dimensions s= et v= requises".into(),
                ));
            };
            if w > MAX_SIDE || h > MAX_SIDE {
                return Err(GraphicsError::TooBig);
            }
            if w == 0 || h == 0 {
                return Err(GraphicsError::Invalid("image vide".into()));
            }
            let bpp = if format == Format::Rgb { 3 } else { 4 };
            if bytes.len() != w as usize * h as usize * bpp {
                return Err(GraphicsError::Invalid(
                    "longueur de charge incohérente avec les dimensions".into(),
                ));
            }
            let rgba = if bpp == 4 {
                bytes.to_vec()
            } else {
                bytes
                    .chunks_exact(3)
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect()
            };
            Ok(ImageData::new(w, h, rgba))
        }
    }
}

/// Une image posée : taille en cellules et fraction affichée de l'image.
#[derive(Debug)]
pub struct Placement {
    pub image: Arc<ImageData>,
    pub cols: u16,
    pub rows: u16,
    pub width_cells: f32,
    pub height_cells: f32,
}

/// Tranche d'un placement (rangée `row`, à partir de la colonne `col`) rattachée à une ligne.
#[derive(Clone)]
pub struct ImageStrip {
    pub placement: Arc<Placement>,
    pub col: u16,
    pub row: u16,
}

impl PartialEq for ImageStrip {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.placement, &other.placement)
            && self.col == other.col
            && self.row == other.row
    }
}

impl Eq for ImageStrip {}

impl fmt::Debug for ImageStrip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ImageStrip {{ image: {}, col: {}, row: {}, cols: {} }}",
            self.placement.image.id, self.col, self.row, self.placement.cols
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, RgbaImage};
    use std::io::Cursor;

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn png_declaring_huge_width_is_too_big_without_decoding() {
        let mut bytes = Vec::new();
        RgbaImage::new(1, 1)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        // IHDR : longueur 4 + type 4 + données 13 ; la largeur débute à l'octet 16.
        bytes[16..20].copy_from_slice(&9000u32.to_be_bytes());
        let crc = crc32(&bytes[12..29]);
        bytes[29..33].copy_from_slice(&crc.to_be_bytes());
        assert_eq!(
            decode(Format::Png, None, None, &bytes).unwrap_err(),
            GraphicsError::TooBig
        );
    }

    #[test]
    fn zero_sized_raw_images_are_invalid() {
        for f in [Format::Rgb, Format::Rgba] {
            let e = decode(f, Some(0), Some(0), &[]).unwrap_err();
            assert!(matches!(e, GraphicsError::Invalid(_)));
            let e = decode(f, Some(0), Some(3), &[]).unwrap_err();
            assert!(matches!(e, GraphicsError::Invalid(_)));
        }
    }

    #[test]
    fn png_is_decoded_to_rgba() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([0, 255, 0, 128]));
        let mut bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        let d = decode(Format::Png, None, None, &bytes).unwrap();
        assert_eq!((d.width, d.height), (2, 1));
        assert_eq!(d.rgba, vec![255, 0, 0, 255, 0, 255, 0, 128]);
    }

    #[test]
    fn raw_rgb_is_expanded() {
        let d = decode(Format::Rgb, Some(2), Some(1), &[1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(d.rgba, vec![1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn raw_rgba_is_kept() {
        let d = decode(Format::Rgba, Some(1), Some(1), &[1, 2, 3, 4]).unwrap();
        assert_eq!(d.rgba, vec![1, 2, 3, 4]);
    }

    #[test]
    fn raw_with_wrong_length_is_invalid() {
        let e = decode(Format::Rgba, Some(2), Some(2), &[0; 15]).unwrap_err();
        assert!(matches!(e, GraphicsError::Invalid(_)));
        let e = decode(Format::Rgb, None, Some(2), &[0; 12]).unwrap_err();
        assert!(matches!(e, GraphicsError::Invalid(_)));
    }

    #[test]
    fn oversized_side_is_too_big() {
        let e = decode(Format::Rgba, Some(MAX_SIDE + 1), Some(1), &[]).unwrap_err();
        assert_eq!(e, GraphicsError::TooBig);
    }

    #[test]
    fn ids_are_unique() {
        let a = ImageData::new(1, 1, vec![0; 4]);
        let b = ImageData::new(1, 1, vec![0; 4]);
        assert_ne!(a.id, b.id);
    }
}
