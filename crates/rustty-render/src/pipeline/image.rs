//! Quads texturés par les images du protocole graphique kitty : une texture
//! par image, gardée tant qu'une bande de l'image est dessinée.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use rustty_vt::graphics::ImageData;
use wgpu::util::DeviceExt as _;

use super::{Globals, alpha_target, globals_layout};
use crate::images::ImageDraw;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ImageInstance {
    /// `x, y, w, h` en pixels.
    pub dest: [f32; 4],
    /// `u0, v0, u1, v1` dans l'image.
    pub uv: [f32; 4],
}

/// Textures des images, indexées par `ImageData::id`.
#[derive(Default)]
pub struct ImageTextures {
    entries: HashMap<u64, (wgpu::Texture, wgpu::BindGroup)>,
    /// Images trop grandes pour le GPU, déjà signalées une fois.
    rejected: HashSet<u64>,
}

/// Une texture `width × height` est-elle acceptée par un GPU dont le côté
/// maximal est `max_side` ? Une image vide ne l'est jamais.
pub fn fits_device(width: u32, height: u32, max_side: u32) -> bool {
    width > 0 && height > 0 && width <= max_side && height <= max_side
}

impl ImageTextures {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Oublie les textures des images absentes de `used`.
    pub fn retain(&mut self, used: &HashSet<u64>) {
        self.entries.retain(|id, _| used.contains(id));
        self.rejected.retain(|id| used.contains(id));
    }
}

pub struct ImagePipeline {
    pipeline: wgpu::RenderPipeline,
    globals_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

/// Les bandes d'une image prêtes à dessiner : une instance par bande, avec
/// l'identifiant de l'image dont elle prend la texture.
pub struct ImageBatch {
    instances: wgpu::Buffer,
    ids: Vec<u64>,
    globals_group: wgpu::BindGroup,
    _globals: wgpu::Buffer,
}

impl ImagePipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rustty-image"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/image.wgsl").into()),
        });
        let globals_layout = globals_layout(device);
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rustty-image-texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rustty-image"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rustty-image"),
            bind_group_layouts: &[Some(&globals_layout), Some(&texture_layout)],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rustty-image"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ImageInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(alpha_target(format))],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            globals_layout,
            texture_layout,
            sampler,
        }
    }

    /// Envoie les textures manquantes et prépare les instances de `draws`.
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &mut ImageTextures,
        draws: &[ImageDraw],
        viewport: (u32, u32),
    ) -> Option<ImageBatch> {
        // Une image plus grande que ce que le GPU accepte (GL/GLES descend
        // sous 8192) ferait paniquer wgpu : elle n'est pas dessinée.
        let max_side = device.limits().max_texture_dimension_2d;
        let draws: Vec<&ImageDraw> = draws
            .iter()
            .filter(|d| {
                let fits = fits_device(d.image.width, d.image.height, max_side);
                if !fits && textures.rejected.insert(d.image.id) {
                    eprintln!(
                        "image {}×{} ignorée : le GPU limite les textures à {max_side} px de côté",
                        d.image.width, d.image.height
                    );
                }
                fits
            })
            .collect();
        if draws.is_empty() {
            return None;
        }
        for d in &draws {
            if let Entry::Vacant(slot) = textures.entries.entry(d.image.id) {
                slot.insert(self.upload(device, queue, &d.image));
            }
        }
        let instances: Vec<ImageInstance> = draws
            .iter()
            .map(|d| ImageInstance {
                dest: d.dest,
                uv: d.uv,
            })
            .collect();
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-image-instances"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-image-globals"),
            contents: bytemuck::bytes_of(&Globals::new(viewport)),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let globals_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustty-image-globals"),
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        Some(ImageBatch {
            instances,
            ids: draws.iter().map(|d| d.image.id).collect(),
            globals_group,
            _globals: globals,
        })
    }

    /// Dessine chaque bande avec la texture de son image.
    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        batch: &'a ImageBatch,
        textures: &'a ImageTextures,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &batch.globals_group, &[]);
        pass.set_vertex_buffer(0, batch.instances.slice(..));
        for (i, id) in (0u32..).zip(&batch.ids) {
            let Some((_, group)) = textures.entries.get(id) else {
                continue;
            };
            pass.set_bind_group(1, group, &[]);
            pass.draw(0..6, i..i + 1);
        }
    }

    fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &ImageData,
    ) -> (wgpu::Texture, wgpu::BindGroup) {
        let size = wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rustty-image"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Octets passés tels quels, comme les couleurs de la palette.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width * 4),
                rows_per_image: None,
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustty-image-texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        (texture, group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_within_the_device_limit_fit() {
        assert!(fits_device(4096, 4096, 4096));
        assert!(fits_device(1, 1, 2048));
    }

    #[test]
    fn images_beyond_the_device_limit_do_not_fit() {
        assert!(!fits_device(8192, 10, 4096));
        assert!(!fits_device(10, 8192, 4096));
    }

    #[test]
    fn empty_images_do_not_fit() {
        assert!(!fits_device(0, 10, 4096));
        assert!(!fits_device(10, 0, 4096));
    }
}
