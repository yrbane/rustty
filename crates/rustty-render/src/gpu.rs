//! Accès au GPU : contexte sans fenêtre pour les tests et le binaire (qui
//! fournit son propre adaptateur), cible hors écran et relecture des pixels.

use crate::color::Rgba;

#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    #[error("aucun adaptateur graphique disponible : {0}")]
    NoAdapter(String),
    #[error("impossible de créer le périphérique graphique : {0}")]
    Device(String),
    #[error("relecture des pixels impossible : {0}")]
    Readback(String),
}

pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    /// Un adaptateur sans surface : le vrai GPU s'il y en a un, sinon le
    /// rendu logiciel (lavapipe, WARP).
    pub fn headless() -> Result<Self, GpuError> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        descriptor.backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(descriptor);
        let hardware = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: None,
            ..Default::default()
        };
        let software = wgpu::RequestAdapterOptions {
            force_fallback_adapter: true,
            ..hardware.clone()
        };
        let adapter = match pollster::block_on(instance.request_adapter(&hardware)) {
            Ok(a) => a,
            Err(first) => {
                pollster::block_on(instance.request_adapter(&software)).map_err(|second| {
                    GpuError::NoAdapter(format!("{first} ; repli logiciel : {second}"))
                })?
            }
        };
        Self::with_adapter(instance, adapter)
    }

    pub fn with_adapter(
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
    ) -> Result<Self, GpuError> {
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("rustty"),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor))
            .map_err(|e| GpuError::Device(e.to_string()))?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
}

/// Format des cibles hors écran : non-sRGB, pour des pixels relus égaux aux
/// couleurs demandées.
pub const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub struct Offscreen {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl Offscreen {
    pub fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rustty-offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            width,
            height,
        }
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Copie la texture dans un tampon lisible et rend les pixels RGBA8,
    /// ligne par ligne, sans le remplissage d'alignement.
    pub fn read_rgba(&self, ctx: &GpuContext) -> Result<Vec<u8>, GpuError> {
        let unpadded = self.width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = unpadded.div_ceil(align) * align;
        let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rustty-readback"),
            size: u64::from(padded) * u64::from(self.height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rustty-readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        ctx.queue.submit(Some(encoder.finish()));
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        ctx.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .map_err(|e| GpuError::Readback(e.to_string()))?;
        rx.recv()
            .map_err(|e| GpuError::Readback(e.to_string()))?
            .map_err(|e| GpuError::Readback(e.to_string()))?;
        let data = slice
            .get_mapped_range()
            .map_err(|e| GpuError::Readback(e.to_string()))?;
        let mut out = Vec::with_capacity((unpadded * self.height) as usize);
        for row in data.chunks(padded as usize) {
            out.extend_from_slice(&row[..unpadded as usize]);
        }
        drop(data);
        buffer.unmap();
        Ok(out)
    }
}

/// Une passe de rendu qui ne fait qu'effacer `view` avec `color`.
pub fn clear(ctx: &GpuContext, view: &wgpu::TextureView, color: Rgba) {
    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rustty-clear"),
        });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rustty-clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(to_wgpu_color(color)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }
    ctx.queue.submit(Some(encoder.finish()));
}

pub(crate) fn to_wgpu_color(c: Rgba) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(c.r),
        g: f64::from(c.g),
        b: f64::from(c.b),
        a: f64::from(c.a),
    }
}
