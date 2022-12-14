use std::hash::Hash;

use rand::Rng;
use wgpu::util::DeviceExt;
use rand_seeder::SipHasher;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Vertex {
    pub(super) position: [f32; 3],
    pub(super) color: [f32; 3]
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = { 
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3] 
    };

    pub(super) fn description<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Default)]
pub(super) struct Mesh {
    pub(super) vertices: Vec<Vertex>,
    pub(super) indices: Vec<u16>,
}

impl Mesh {
    pub(super) fn from_planet(planet: &crate::sim::planet::Planet) -> Self {
        Self {
            vertices: {
                let color = {
                    let mut h = SipHasher::new();
                    planet.rad.to_string().hash(&mut h);
                    if let Some(o) = &planet.orbit {
                        o.dist.to_string().hash(&mut h);
                        o.speed.to_string().hash(&mut h);
                        o.ccw.hash(&mut h);
                    }

                    let mut h = h.into_rng();

                    [
                        h.gen_range(0f32..1f32),
                        h.gen_range(0f32..1f32),
                        h.gen_range(0f32..1f32)
                    ]
                };

                let mut vertices = Vec::new();

                // 1st add the center point
                vertices.push(
                    Vertex {
                        position: [
                            planet.pos.x,
                            planet.pos.y,
                            0f32
                        ],
                        color
                    }
                );

                // AND the 1st point on the circumference of the circle
                vertices.push(
                    Vertex {
                        position: [
                            planet.rad + planet.pos.x,
                            planet.pos.y,
                            0f32 
                        ],
                        color
                    }
                );

                // Add in each slice, one by one
                for i in (19625..628000).step_by(19625) {
                    let i = i as f32 * 0.00001f32;

                    vertices.push(
                        Vertex {
                            position: [
                                i.cos() * planet.rad + planet.pos.x,
                                i.sin() * planet.rad + planet.pos.y,
                                0f32
                            ],
                            color
                        }
                    );
                }

                vertices
            },
            indices: { 
                vec![
                     1,  2,  0,  2,  3,  0,  3,  4,  0,  4,  5,  0, 
                     5,  6,  0,  6,  7,  0,  7,  8,  0,  8,  9,  0, 
                     9, 10,  0, 10, 11,  0, 11, 12,  0, 12, 13,  0, 
                    13, 14,  0, 14, 15,  0, 15, 16,  0, 16, 17,  0, 
                    17, 18,  0, 18, 19,  0, 19, 20,  0, 20, 21,  0, 
                    21, 22,  0, 22, 23,  0, 23, 24,  0, 24, 25,  0, 
                    25, 26,  0, 26, 27,  0, 27, 28,  0, 28, 29,  0, 
                    29, 30,  0, 30, 31,  0, 31, 32,  0, 32,  1,  0
                ]
            }
        }
    }

    pub(super) fn from_ship(ship: &crate::sim::ship::Ship) -> Self {
        Self {
            vertices: {
                use crate::sim::ship::ShipGoal;
                use crate::sim::ship::ShipJob::*;

                let size = 0.05f32;
                let color = match ship.job {
                    Miner => [1f32, 0.2f32, 0.8f32],
                    Trader { cargo: false } => [0f32, 0.6f32, 1f32],
                    Trader { cargo: true } => [0f32, 1f32, 0.6f32],
                    Pirate { .. } if matches!(ship.goal, ShipGoal::Wander) || matches!(ship.goal, ShipGoal::Scan) => [1f32, 0.1f32, 0f32],
                    Pirate { .. } => [1f32, 0f32, 0f32]
                };
        
                let top_pos = [ ship.pos.x, ship.pos.y, 0f32 ];
                
                let min = ship.angle - 0.2617994;
                let min_pos = [ 
                    size * min.sin() + ship.pos.x, 
                    size * min.cos() + ship.pos.y, 
                    0f32 
                ];

                let max = ship.angle + 0.2617994;
                let max_pos = [ 
                    size * max.sin() + ship.pos.x, 
                    size * max.cos() + ship.pos.y, 
                    0f32 
                ];

        
                vec![
                    Vertex { position: top_pos, color },
                    Vertex { position: min_pos, color: [ 0f32, 0f32, 0f32 ] },
                    Vertex { position: max_pos, color: [ 0f32, 0f32, 0f32 ] },
                ]
            },
            indices: vec![0, 1, 2]
        }
    }
}

impl Mesh {
    pub(super) fn build_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(self.vertices.as_slice()),
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
}