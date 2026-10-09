//! La surface de la fenêtre : format de rendu (vue non-sRGB pour écrire la
//! palette telle quelle), mode de composition alpha (opacité), acquisition
//! d'image résistante aux pertes de surface.

use std::sync::Arc;

use anyhow::Context as _;
use rustty_render::GpuContext;
use wgpu::{CompositeAlphaMode, TextureFormat};
use winit::window::Window;

pub fn pick_format(formats: &[TextureFormat]) -> Option<(TextureFormat, TextureFormat)> {
    for wanted in [TextureFormat::Bgra8Unorm, TextureFormat::Rgba8Unorm] {
        if formats.contains(&wanted) {
            return Some((wanted, wanted));
        }
    }
    let first = *formats.first()?;
    Some((first, first.remove_srgb_suffix()))
}

pub fn pick_alpha_mode(modes: &[CompositeAlphaMode]) -> CompositeAlphaMode {
    [
        CompositeAlphaMode::PreMultiplied,
        CompositeAlphaMode::PostMultiplied,
        CompositeAlphaMode::Inherit,
        CompositeAlphaMode::Auto,
    ]
    .into_iter()
    .find(|m| modes.contains(m))
    .or_else(|| modes.first().copied())
    .unwrap_or(CompositeAlphaMode::Opaque)
}

/// Instance, surface et contexte GPU compatibles avec la fenêtre.
pub fn init_gpu(window: Arc<Window>) -> anyhow::Result<(GpuContext, wgpu::Surface<'static>)> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
    descriptor.backends = wgpu::Backends::all();
    let instance = wgpu::Instance::new(descriptor);
    let surface = instance
        .create_surface(window)
        .context("création de la surface de rendu")?;
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
        ..Default::default()
    }))
    .context("aucun adaptateur graphique compatible avec la fenêtre")?;
    let ctx = GpuContext::with_adapter(instance, adapter)
        .context("création du périphérique graphique")?;
    Ok((ctx, surface))
}

/// Ce qu'il faut faire d'une tentative d'acquisition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquireKind {
    Frame,
    /// Surface reconfigurée : redessiner aussitôt.
    Retry,
    /// Rien à dessiner pour cette image.
    Skip,
}

pub enum Acquire {
    Frame(wgpu::SurfaceTexture),
    Retry,
    Skip,
}

pub fn classify(current: &wgpu::CurrentSurfaceTexture) -> AcquireKind {
    use wgpu::CurrentSurfaceTexture as C;
    match current {
        C::Success(_) | C::Suboptimal(_) => AcquireKind::Frame,
        C::Outdated | C::Lost => AcquireKind::Retry,
        C::Timeout | C::Occluded | C::Validation => AcquireKind::Skip,
    }
}

/// Borne les relances : seule la première tentative consécutive perdue
/// redemande un dessin ; ensuite on saute jusqu'à une image réussie, pour
/// qu'une surface durablement perdue ne fasse pas tourner le processeur.
pub fn throttle(kind: AcquireKind, retries: &mut u32) -> AcquireKind {
    match kind {
        AcquireKind::Retry => {
            *retries += 1;
            if *retries > 1 {
                AcquireKind::Skip
            } else {
                AcquireKind::Retry
            }
        }
        AcquireKind::Frame => {
            *retries = 0;
            kind
        }
        AcquireKind::Skip => kind,
    }
}

pub struct Surface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    view_format: TextureFormat,
    retries: u32,
}

impl Surface {
    pub fn new(
        surface: wgpu::Surface<'static>,
        ctx: &GpuContext,
        width: u32,
        height: u32,
    ) -> anyhow::Result<Self> {
        let caps = surface.get_capabilities(&ctx.adapter);
        let (format, view_format) =
            pick_format(&caps.formats).context("la surface n'offre aucun format")?;
        let mut config = surface
            .get_default_config(&ctx.adapter, width.max(1), height.max(1))
            .context("surface incompatible avec l'adaptateur")?;
        config.format = format;
        config.alpha_mode = pick_alpha_mode(&caps.alpha_modes);
        config.view_formats = if view_format == format {
            Vec::new()
        } else {
            vec![view_format]
        };
        surface.configure(&ctx.device, &config);
        Ok(Self {
            surface,
            config,
            view_format,
            retries: 0,
        })
    }

    pub fn resize(&mut self, ctx: &GpuContext, width: u32, height: u32) {
        let (w, h) = (width.max(1), height.max(1));
        if (w, h) != (self.config.width, self.config.height) {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&ctx.device, &self.config);
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn view_format(&self) -> TextureFormat {
        self.view_format
    }

    pub fn premultiplied(&self) -> bool {
        self.config.alpha_mode == CompositeAlphaMode::PreMultiplied
    }

    /// L'image à dessiner, de quoi réessayer tout de suite, ou la sauter.
    pub fn acquire(&mut self, ctx: &GpuContext) -> Acquire {
        let current = self.surface.get_current_texture();
        let raw = classify(&current);
        if raw == AcquireKind::Retry {
            self.surface.configure(&ctx.device, &self.config);
        }
        let kind = throttle(raw, &mut self.retries);
        match current {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => Acquire::Frame(t),
            _ if kind == AcquireKind::Retry => Acquire::Retry,
            _ => Acquire::Skip,
        }
    }

    pub fn view(texture: &wgpu::SurfaceTexture, format: TextureFormat) -> wgpu::TextureView {
        texture.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(format),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::{CompositeAlphaMode as A, TextureFormat as F};

    #[test]
    fn lost_and_outdated_surfaces_are_retried() {
        use wgpu::CurrentSurfaceTexture as C;
        assert_eq!(classify(&C::Outdated), AcquireKind::Retry);
        assert_eq!(classify(&C::Lost), AcquireKind::Retry);
        assert_eq!(classify(&C::Timeout), AcquireKind::Skip);
        assert_eq!(classify(&C::Occluded), AcquireKind::Skip);
        assert_eq!(classify(&C::Validation), AcquireKind::Skip);
    }

    #[test]
    fn only_the_first_consecutive_retry_asks_for_a_redraw() {
        let mut retries = 0;
        assert_eq!(
            throttle(AcquireKind::Retry, &mut retries),
            AcquireKind::Retry
        );
        assert_eq!(
            throttle(AcquireKind::Retry, &mut retries),
            AcquireKind::Skip
        );
        assert_eq!(
            throttle(AcquireKind::Retry, &mut retries),
            AcquireKind::Skip
        );
        assert_eq!(
            throttle(AcquireKind::Frame, &mut retries),
            AcquireKind::Frame
        );
        assert_eq!(
            throttle(AcquireKind::Retry, &mut retries),
            AcquireKind::Retry
        );
        assert_eq!(throttle(AcquireKind::Skip, &mut retries), AcquireKind::Skip);
    }

    #[test]
    fn non_srgb_formats_are_preferred_as_is() {
        assert_eq!(
            pick_format(&[F::Bgra8UnormSrgb, F::Bgra8Unorm]),
            Some((F::Bgra8Unorm, F::Bgra8Unorm))
        );
        assert_eq!(
            pick_format(&[F::Rgba8UnormSrgb, F::Rgba8Unorm]),
            Some((F::Rgba8Unorm, F::Rgba8Unorm))
        );
    }

    #[test]
    fn srgb_only_surfaces_get_a_linear_view() {
        assert_eq!(
            pick_format(&[F::Bgra8UnormSrgb]),
            Some((F::Bgra8UnormSrgb, F::Bgra8Unorm))
        );
        assert_eq!(pick_format(&[]), None);
    }

    #[test]
    fn alpha_mode_prefers_premultiplied() {
        assert_eq!(
            pick_alpha_mode(&[A::Opaque, A::PostMultiplied, A::PreMultiplied]),
            A::PreMultiplied
        );
        assert_eq!(
            pick_alpha_mode(&[A::Opaque, A::PostMultiplied]),
            A::PostMultiplied
        );
        assert_eq!(pick_alpha_mode(&[A::Opaque, A::Inherit]), A::Inherit);
        assert_eq!(pick_alpha_mode(&[A::Opaque]), A::Opaque);
        assert_eq!(pick_alpha_mode(&[]), A::Opaque);
    }
}
