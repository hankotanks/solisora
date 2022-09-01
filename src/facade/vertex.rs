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

pub(super) const DEFAULT_VERTICES: &[Vertex] = &[
    Vertex { position: [ 0.0,  0.5, 0.0], color: [0f32, 0f32, 0f32] },
    Vertex { position: [-0.5, -0.5, 0.0], color: [0f32, 0f32, 0f32] },
    Vertex { position: [ 0.5, -0.5, 0.0], color: [0f32, 0f32, 0f32] }
];