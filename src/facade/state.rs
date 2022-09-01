use wgpu::util::DeviceExt;

use winit::window;
use winit::event::WindowEvent;

use super::vertex::{self, Vertex};
use super::vertex::DEFAULT_VERTICES;

pub(super) struct State {
    pub(super) size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    render_pipeline: wgpu::RenderPipeline
}

impl State {
    pub(super) async fn new(window: &window::Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());

        let surface = unsafe { 
            instance.create_surface(window) 
        };

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: { 
                    if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    }
                },
                label: None
            },
            None
        ).await.unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo
        };

        surface.configure(&device, &config);

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(DEFAULT_VERTICES),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[0, 1, 2]),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let index_count = 3u32;

        let shader = device.create_shader_module(
            wgpu::include_wgsl!("shader.wgsl")
        );    

        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[]
            }
        );

        let render_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[
                        vertex::Vertex::description()
                    ]
                },
                fragment: Some(
                    wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[
                            Some(
                                wgpu::ColorTargetState {
                                    format: config.format,
                                    blend: Some(wgpu::BlendState::REPLACE),
                                    write_mask: wgpu::ColorWrites::ALL
                                }
                            )
                        ],
                    }
                ),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, //Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None
            }
        );

        Self {
            size,
            surface,
            device,
            queue,
            config,
            vertex_buffer,
            index_buffer,
            index_count,
            render_pipeline
        }
    }

    pub(super) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub(super) fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    pub(super) fn update(&mut self, simulation: &mut crate::simulation::Simulation) {
        //dbg!(simulation.objects().collect::<Vec<&crate::simulation::Object>>().len());
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        for object in simulation.bodies() {
            let center_index = vertices.len();

            // 1st add the center point
            vertices.push(
                Vertex {
                    position: [
                        object.pos().x(),
                        object.pos().y(),
                        0f32
                    ],
                    color: [ 1f32, 1f32, 1f32 ]
                }
            );

            // AND the 1st point on the circumference of the circle
            vertices.push(
                Vertex {
                    position: [
                        object.radius() + object.pos().x(),
                        object.pos().y(),
                        0f32
                    ],
                    color: [ 1f32, 1f32, 1f32 ]
                }
            );

            // Add in each slice, one by one
            for i in (19625..628000).step_by(19625) {
                let i = i as f32 * 0.00001f32;

                vertices.push(
                    Vertex {
                        position: [
                            i.cos() * object.radius() + object.pos().x(),
                            i.sin() * object.radius() + object.pos().y(),
                            0f32
                        ],
                        color: [ 1f32, 1f32, 1f32 ]
                    }
                );

                //dbg!(vertices.len());

                indices.append(
                    &mut vec![
                        vertices.len() as u16 - 2, 
                        vertices.len() as u16 - 1, 
                        center_index as u16
                    ]
                )
            }

            // Finally, connect the final edge point to the 1st
            // This avoids duplicating the point that was added before the loop
            indices.append(
                &mut vec![
                    vertices.len() as u16 - 1, 
                    center_index as u16 + 1, 
                    center_index as u16
                ]
            );
        }

        self.vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices.as_slice()),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        self.index_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indices.as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        self.index_count = indices.len() as u32;
    }

    pub(super) fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: None,
            }
        );

        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[
                        Some(
                            wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: true
                                },
                            }
                        )
                    ],
                    depth_stencil_attachment: None
                }
            );

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(
                0, 
                self.vertex_buffer.slice(..)
            );

            render_pass.set_index_buffer(
                self.index_buffer.slice(..), 
                wgpu::IndexFormat::Uint16
            );

            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }
    
        self.queue.submit(
            std::iter::once(encoder.finish())
        );

        output.present();
    
        Ok(())
    }
}