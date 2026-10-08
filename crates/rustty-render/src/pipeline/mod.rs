//! Les deux pipelines wgpu : quads colorés et quads texturés par l'atlas.

pub mod quad;

use bytemuck::{Pod, Zeroable};

/// Uniforme partagé : la taille du viewport en pixels.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct Globals {
    pub viewport: [f32; 2],
    pub _pad: [f32; 2],
}

impl Globals {
    pub fn new(viewport: (u32, u32)) -> Self {
        Self {
            viewport: [viewport.0.max(1) as f32, viewport.1.max(1) as f32],
            _pad: [0.0; 2],
        }
    }
}

pub(crate) fn globals_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("rustty-globals"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub(crate) fn alpha_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    }
}
