mod meshable;

use wgpu::util::DeviceExt;

use self::meshable::Meshable;

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

pub(super) struct Mesh {
    vertices: Vec<Vec<Vertex>>,
    indices: Vec<u16>,
    count: u32
}

impl Mesh {
    pub(super) fn new(simulation: &crate::simulation::Simulation) -> Self {
        let mut mesh = Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            count: 0u32
        };

        mesh.handle_simulation_update(simulation);
        let mut offset = 0u16;
        for (index, object) in simulation.bodies().enumerate() {
            mesh.indices.append(
                &mut object.indices().iter().map(|&f| {
                    f + offset
                } ).collect::<Vec<u16>>()
            );
            mesh.count += object.indices().len() as u32;

            offset += mesh.vertices[index].len() as u16;
        }

        mesh
    }

    pub(super) fn build_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let mut flattened_vertices = Vec::new();

        for mut vertices in self.vertices.iter().cloned() {
            flattened_vertices.append(&mut vertices);
        }

        device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(flattened_vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX
            }
        )
    }

    pub(super) fn build_index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(self.indices.as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            }
        )
    }

    pub(super) fn count(&self) -> u32 {
        self.count
    }

    pub(super) fn handle_simulation_update(&mut self, simulation: &crate::simulation::Simulation) {
        self.vertices.clear();
        for object in simulation.bodies() {
            self.vertices.push(object.vertices());
        }
    }
}