//! Copie réduite d'une image à sa taille d'affichage : un placement ne garde
//! jamais plus de pixels qu'il n'en montre (marge ×2 pour le zoom).

use std::sync::Arc;

use image::RgbaImage;
use image::imageops::{self, FilterType};

use super::ImageData;

/// Marge appliquée à la taille affichée en pixels, pour rester net au zoom.
pub const DISPLAY_MARGIN: f32 = 2.0;

/// Taille maximale en pixels conservée pour un affichage de
/// `cells` (largeur, hauteur en cellules) avec des cellules de `cell` pixels.
pub fn display_pixels(cells: (f32, f32), cell: (u32, u32)) -> (u32, u32) {
    let side = |cells: f32, px: u32| ((cells * px as f32 * DISPLAY_MARGIN).ceil() as u32).max(1);
    (side(cells.0, cell.0), side(cells.1, cell.1))
}

/// Rend l'image réduite pour tenir dans `max` pixels, chaque côté
/// indépendamment ; jamais agrandie. Sans réduction, la même image est rendue.
pub fn fit_display(image: &Arc<ImageData>, max: (u32, u32)) -> Arc<ImageData> {
    let (w, h) = (
        image.width.min(max.0.max(1)),
        image.height.min(max.1.max(1)),
    );
    if (w, h) == (image.width, image.height) {
        return Arc::clone(image);
    }
    let Some(full) = RgbaImage::from_raw(image.width, image.height, image.rgba.clone()) else {
        return Arc::clone(image);
    };
    let small = imageops::resize(&full, w, h, FilterType::Triangle);
    Arc::new(ImageData::derived(image.source, w, h, small.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform(w: u32, h: u32) -> Arc<ImageData> {
        let rgba = [200u8, 30, 30, 255].repeat(w as usize * h as usize);
        Arc::new(ImageData::new(w, h, rgba))
    }

    #[test]
    fn display_pixels_doubles_the_shown_size() {
        assert_eq!(display_pixels((10.0, 5.0), (10, 20)), (200, 200));
        assert_eq!(display_pixels((0.01, 0.01), (10, 20)), (1, 1));
    }

    #[test]
    fn large_image_is_reduced_with_its_pixels() {
        let big = uniform(1000, 800);
        let small = fit_display(&big, (200, 100));
        assert_eq!((small.width, small.height), (200, 100));
        assert_eq!(small.rgba.len(), 200 * 100 * 4);
        assert!(
            small
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [200, 30, 30, 255])
        );
        assert_ne!(small.id, big.id, "nouvelle clé de cache côté rendu");
        assert_eq!(small.source, big.id, "rattachée à l'image d'origine");
    }

    #[test]
    fn small_image_is_kept_as_is() {
        let img = uniform(10, 20);
        let same = fit_display(&img, (200, 200));
        assert!(Arc::ptr_eq(&img, &same));
        assert_eq!(img.source, img.id);
    }

    #[test]
    fn each_side_is_reduced_independently_never_enlarged() {
        let img = uniform(100, 10);
        let out = fit_display(&img, (50, 400));
        assert_eq!((out.width, out.height), (50, 10));
    }
}
