//! Rectangles pleins instanciés.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

use super::{Globals, alpha_target, globals_layout};
use crate::color::Rgba;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct QuadInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
}

impl QuadInstance {
    pub fn new(x: f32, y: f32, w: f32, h: f32, color: Rgba) -> Self {
        Self {
            pos: [x, y],
            size: [w, h],
            color: color.to_array(),
        }
    }
}

pub struct QuadPipeline {
    pipeline: wgpu::RenderPipeline,
    globals_layout: wgpu::BindGroupLayout,
}

pub struct QuadBatch {
    instances: wgpu::Buffer,
    count: u32,
    bind_group: wgpu::BindGroup,
    _globals: wgpu::Buffer,
}

impl QuadBatch {
    pub fn len(&self) -> usize {
        self.count as usize
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl QuadPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rustty-quad"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/quad.wgsl").into()),
        });
        let globals_layout = globals_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rustty-quad"),
            bind_group_layouts: &[Some(&globals_layout)],
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
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
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
        }
    }

    pub fn prepare(
        &self,
        device: &wgpu::Device,
        instances: &[QuadInstance],
        viewport: (u32, u32),
    ) -> Option<QuadBatch> {
        if instances.is_empty() {
            return None;
        }
        let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-quad-instances"),
            contents: bytemuck::cast_slice(instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-quad-globals"),
            contents: bytemuck::bytes_of(&Globals::new(viewport)),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustty-quad-globals"),
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        Some(QuadBatch {
            instances: instances_buffer,
            count: instances.len() as u32,
            bind_group,
            _globals: globals,
        })
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, batch: &'a QuadBatch) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &batch.bind_group, &[]);
        pass.set_vertex_buffer(0, batch.instances.slice(..));
        pass.draw(0..6, 0..batch.count);
    }
}
