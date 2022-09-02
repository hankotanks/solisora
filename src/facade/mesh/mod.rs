mod planetoid;

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Vertex {
    pub(super) position: [f32; 3],
    pub(super) color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub(super) fn description<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

trait Meshable {
    fn vertices(&self) -> Vec<Vertex>;
    fn indices(&self) -> Vec<u16>;
}

pub(super) struct Mesh {
    objects: Vec<Box<dyn Meshable>>
}

impl Mesh {
    pub(super) fn new() -> Self {
        Self {
            objects: Vec::new()
        }
    }

    pub(super) fn build_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let mut vertices: Vec<Vertex> = Vec::new();

        for object in self.objects.iter() {
            vertices.append(&mut object.vertices());
        }

        device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX
            }
        )
    }

    pub(super) fn build_index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let mut indices: Vec<u16> = Vec::new();

        let mut offset = 0u16;
        for object in self.objects.iter() {
            indices.append(
                &mut object.indices().iter().map(|i| i + offset).collect::<Vec<u16>>()
            );

            offset += object.vertices().len() as u16;
        }

        device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indices.as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            }
        )
    }

    pub(super) fn count(&self) -> u32 {
        let mut c = 0u32;
        for object in self.objects.iter() {
            c += object.indices().len() as u32;
        }

        c
    }

    pub(super) fn update_from_simulation(&mut self, simulation: &crate::simulation::Simulation) {
        self.objects.clear();

        for object in simulation.bodies() {
            self.objects.push(
                Box::new(
                    planetoid::Planetoid::new(object.pos(), object.radius())
                )
            );
        }
    }
}