//! Quads texturés par l'atlas : les glyphes de la grille et du chrome.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

use super::{Globals, alpha_target, globals_layout};
use crate::atlas::AtlasRegion;
use crate::color::Rgba;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct GlyphInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    /// `u0, v0, u1, v1` dans l'atlas.
    pub uv: [f32; 4],
    pub color: [f32; 4],
    /// Bit 0 : glyphe couleur.
    pub flags: u32,
    pub _pad: [u32; 3],
}

impl GlyphInstance {
    pub fn new(x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], color: Rgba, is_color: bool) -> Self {
        Self {
            pos: [x, y],
            size: [w, h],
            uv,
            color: color.to_array(),
            flags: u32::from(is_color),
            _pad: [0; 3],
        }
    }
}

pub struct AtlasTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: u32,
}

impl AtlasTexture {
    pub fn new(device: &wgpu::Device, size: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rustty-atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            size,
        }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    /// Écrit `rgba` (`region.width × region.height × 4` octets) dans l'atlas.
    pub fn upload(&self, queue: &wgpu::Queue, region: AtlasRegion, rgba: &[u8]) {
        debug_assert_eq!(rgba.len() as u32, region.width * region.height * 4);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(region.width * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Remet tout l'atlas à zéro (après un `AtlasPacker::clear`).
    pub fn clear(&self, queue: &wgpu::Queue) {
        let zeros = vec![0u8; (self.size * self.size * 4) as usize];
        self.upload(
            queue,
            AtlasRegion {
                x: 0,
                y: 0,
                width: self.size,
                height: self.size,
            },
            &zeros,
        );
    }
}

pub struct GlyphPipeline {
    pipeline: wgpu::RenderPipeline,
    globals_layout: wgpu::BindGroupLayout,
    atlas_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

pub struct GlyphBatch {
    instances: wgpu::Buffer,
    count: u32,
    globals_group: wgpu::BindGroup,
    atlas_group: wgpu::BindGroup,
    _globals: wgpu::Buffer,
}

impl GlyphBatch {
    pub fn len(&self) -> usize {
        self.count as usize
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl GlyphPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rustty-glyph"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/glyph.wgsl").into()),
        });
        let globals_layout = globals_layout(device);
        let atlas_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rustty-atlas"),
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
            label: Some("rustty-atlas"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rustty-glyph"),
            bind_group_layouts: &[Some(&globals_layout), Some(&atlas_layout)],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rustty-glyph"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GlyphInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Uint32],
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
            atlas_layout,
            sampler,
        }
    }

    pub fn prepare(
        &self,
        device: &wgpu::Device,
        atlas: &AtlasTexture,
        instances: &[GlyphInstance],
        viewport: (u32, u32),
    ) -> Option<GlyphBatch> {
        if instances.is_empty() {
            return None;
        }
        let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-glyph-instances"),
            contents: bytemuck::cast_slice(instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-glyph-globals"),
            contents: bytemuck::bytes_of(&Globals::new(viewport)),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let globals_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustty-glyph-globals"),
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        let atlas_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustty-glyph-atlas"),
            layout: &self.atlas_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        Some(GlyphBatch {
            instances: instances_buffer,
            count: instances.len() as u32,
            globals_group,
            atlas_group,
            _globals: globals,
        })
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, batch: &'a GlyphBatch) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &batch.globals_group, &[]);
        pass.set_bind_group(1, &batch.atlas_group, &[]);
        pass.set_vertex_buffer(0, batch.instances.slice(..));
        pass.draw(0..6, 0..batch.count);
    }
}
