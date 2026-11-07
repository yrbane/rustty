//! Rectangles instanciés : pleins par défaut, ou stylés (coins arrondis,
//! bordure, dégradé vertical) pour le chrome de la fenêtre.

use super::alpha_target;
use super::common::{GlobalsBinding, InstanceBuffer};
use crate::color::Rgba;
use bytemuck::{Pod, Zeroable};

/// Aspect d'un rectangle stylé. Aux valeurs neutres (`flat`), le rendu est
/// celui d'un rectangle plein.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadStyle {
    /// Couleur en haut.
    pub fill: Rgba,
    /// Couleur en bas (dégradé vertical linéaire).
    pub fill_bottom: Rgba,
    /// Rayon des coins en pixels.
    pub radius: f32,
    /// Largeur de la bordure intérieure en pixels, 0 pour aucune.
    pub border_width: f32,
    pub border: Rgba,
}

impl QuadStyle {
    pub const fn flat(color: Rgba) -> Self {
        Self {
            fill: color,
            fill_bottom: color,
            radius: 0.0,
            border_width: 0.0,
            border: Rgba::new(0.0, 0.0, 0.0, 0.0),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct QuadInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub color_bottom: [f32; 4],
    pub border_color: [f32; 4],
    pub radius: f32,
    pub border_width: f32,
    pub _pad: [f32; 2],
}

impl QuadInstance {
    pub fn new(x: f32, y: f32, w: f32, h: f32, color: Rgba) -> Self {
        Self::styled(x, y, w, h, QuadStyle::flat(color))
    }

    pub fn styled(x: f32, y: f32, w: f32, h: f32, style: QuadStyle) -> Self {
        Self {
            pos: [x, y],
            size: [w, h],
            color: style.fill.to_array(),
            color_bottom: style.fill_bottom.to_array(),
            border_color: style.border.to_array(),
            radius: style.radius,
            border_width: style.border_width,
            _pad: [0.0; 2],
        }
    }
}

pub struct QuadPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl QuadPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        globals_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rustty-quad"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/quad.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rustty-quad"),
            bind_group_layouts: &[Some(globals_layout)],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rustty-quad"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<QuadInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32, 6 => Float32],
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
        Self { pipeline }
    }

    /// Dessine `instances` avec l'uniforme `globals` ; rien si le lot est vide.
    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        globals: &'a GlobalsBinding,
        instances: &'a InstanceBuffer,
    ) {
        if instances.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, globals.group(), &[]);
        pass.set_vertex_buffer(0, instances.slice());
        pass.draw(0..6, 0..instances.count());
    }
}
