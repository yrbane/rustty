//! Quads texturés par l'atlas : les glyphes de la grille et du chrome.

use super::common::{
    GlobalsBinding, InstanceBuffer, InstancedPipelineSpec, instanced_pipeline, sampler, shader,
    texture_layout,
};
use crate::atlas::AtlasRegion;
use crate::color::Rgba;
use bytemuck::{Pod, Zeroable};

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
    atlas_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl GlyphPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        globals_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = shader(
            device,
            "rustty-glyph",
            include_str!("../shaders/glyph.wgsl"),
        );
        let atlas_layout = texture_layout(device, "rustty-atlas");
        let sampler = sampler(device, "rustty-atlas", wgpu::FilterMode::Nearest);
        let pipeline = instanced_pipeline(
            device,
            &InstancedPipelineSpec {
                label: "rustty-glyph",
                shader: &shader,
                bind_layouts: &[globals_layout, &atlas_layout],
                stride: std::mem::size_of::<GlyphInstance>() as u64,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Uint32],
                format,
            },
        );
        Self {
            pipeline,
            atlas_layout,
            sampler,
        }
    }

    /// Le bind group d'un atlas : à créer une fois par texture d'atlas, pas
    /// par image.
    pub fn atlas_group(&self, device: &wgpu::Device, atlas: &AtlasTexture) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        })
    }

    /// Dessine `instances` texturées par l'atlas de `atlas_group`.
    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        globals: &'a GlobalsBinding,
        atlas_group: &'a wgpu::BindGroup,
        instances: &'a InstanceBuffer,
    ) {
        if instances.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, globals.group(), &[]);
        pass.set_bind_group(1, atlas_group, &[]);
        pass.set_vertex_buffer(0, instances.slice());
        pass.draw(0..6, 0..instances.count());
    }
}
