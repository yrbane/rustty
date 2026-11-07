//! Objets GPU persistants partagés par les pipelines : un tampon d'instances
//! qui grandit sans jamais rétrécir, et l'uniforme du viewport avec son bind
//! group, réécrit sur place quand la fenêtre change de taille. En régime
//! établi, une image ne crée donc aucun objet GPU.

use bytemuck::Pod;
use wgpu::util::DeviceExt as _;

use super::{Globals, globals_layout};

/// Taille de départ d'un tampon d'instances (en octets) : assez pour une
/// petite image, doublée ensuite à la demande.
const INITIAL_CAPACITY: u64 = 4096;

/// Tampon de vertex instancié, rempli par `write_buffer` et agrandi par
/// doublement quand une image ne tient plus.
pub struct InstanceBuffer {
    buffer: wgpu::Buffer,
    capacity: u64,
    count: u32,
    label: &'static str,
}

impl InstanceBuffer {
    pub fn new(device: &wgpu::Device, label: &'static str) -> Self {
        Self {
            buffer: Self::allocate(device, label, INITIAL_CAPACITY),
            capacity: INITIAL_CAPACITY,
            count: 0,
            label,
        }
    }

    fn allocate(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Remplace le contenu par `data` ; rend vrai si le tampon a dû être
    /// recréé (plus grand).
    pub fn write<T: Pod>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[T],
    ) -> bool {
        self.count = data.len() as u32;
        if data.is_empty() {
            return false;
        }
        let needed = std::mem::size_of_val(data) as u64;
        let grown = needed > self.capacity;
        if grown {
            self.capacity = needed.max(self.capacity * 2);
            self.buffer = Self::allocate(device, self.label, self.capacity);
        }
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
        grown
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(..)
    }
}

/// L'uniforme du viewport et son bind group, partagés par tous les pipelines
/// (même layout). Le tampon est réécrit quand le viewport change ; le bind
/// group ne l'est jamais.
pub struct GlobalsBinding {
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
    viewport: (u32, u32),
}

impl GlobalsBinding {
    pub fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        viewport: (u32, u32),
    ) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustty-globals"),
            contents: bytemuck::bytes_of(&Globals::new(viewport)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustty-globals"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Self {
            buffer,
            group,
            viewport,
        }
    }

    /// Le layout que tout pipeline consommant cet uniforme doit déclarer.
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        globals_layout(device)
    }

    /// Met l'uniforme au viewport courant, seulement s'il a changé.
    pub fn ensure(&mut self, queue: &wgpu::Queue, viewport: (u32, u32)) {
        if viewport != self.viewport {
            queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&Globals::new(viewport)));
            self.viewport = viewport;
        }
    }

    pub fn group(&self) -> &wgpu::BindGroup {
        &self.group
    }
}
