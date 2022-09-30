mod mesh;
mod camera;

use winit::{
    event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window,
    window::WindowBuilder
};
use wgpu::util::DeviceExt;

use mesh::{
    Mesh,
    Vertex
};

use camera::{
    Camera,
    CameraUniform
};


use cgmath::MetricSpace;
use crate::sim::{Sim, ship::ShipGoal};

pub(crate) async fn run(mut sim: crate::sim::Sim) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(&window).await;
    event_loop.run(move |event, _, control_flow| {
        match event {
            event::Event::RedrawRequested(window_id) if window_id == window.id() => {
                sim.update();

                let mesh = build_mesh(&sim);
                state.update(&mesh);

                match state.render() {
                    Ok(..) => {  },
                    Err(wgpu::SurfaceError::Lost) => state.redraw(),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e)
                }
            },
            event::Event::MainEventsCleared => {
                window.request_redraw();
            },
            event::Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => if !state.input(event) {
                match event {
                    // Handle close behavior
                    WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                        input:
                        event::KeyboardInput {
                                state: event::ElementState::Pressed,
                                virtual_keycode: Some(event::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size)
                    },
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => { 
                        state.resize(**new_inner_size) 
                    },
                    _ => {}
                }
            },
            _ => {  }
        }
    });
}

fn build_mesh(sim: &Sim) -> Mesh {
    fn combine_meshes(m1: &mut Mesh, mut m2: Mesh, scale: f32) {
        let offset = m1.vertices.len();
        m2.vertices.iter_mut().for_each(|v| { 
            v.position[0] *= scale; 
            v.position[1] *= scale; } );
        m1.vertices.append(&mut m2.vertices);
        m2.indices.iter_mut().for_each(|i| *i += offset as u16);
        m1.indices.append(&mut m2.indices);
    }

    let mut m = Mesh::default();
    let scale = (sim.system_rad.powf(2f32) * 2f32).sqrt().recip();

    for planet in sim.system.iter() {
        combine_meshes(
            &mut m,
            Mesh::from_planet(planet),
            scale
        );
    }

    for ship in sim.ships.iter() {
        combine_meshes(
            &mut m,
            Mesh::from_ship(ship),
            scale
        );

        if let ShipGoal::Hunt { prey, .. } = ship.goal {
            if ship.pos.distance(sim.ships[prey].pos) < sim.config.raid_range {
                let prey_mesh = Mesh::from_ship(&sim.ships[prey]);
                let ship_mesh = Mesh::from_ship(ship);

                combine_meshes(
                    &mut m,
                    Mesh {
                        vertices: vec![
                            ship_mesh.vertices[0],
                            prey_mesh.vertices[1],
                            prey_mesh.vertices[2]
                        ],
                        indices: vec![0, 1, 2]
                    },
                    scale
                );
            }
        }
    }

    m
}

struct State {
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline
}

impl State {
    async fn new(window: &window::Window) -> Self {
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
                contents: &[],
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &[],
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let index_count = 0u32;

        let camera = Camera {
            pos: (0f32, 0f32).into(),
            zoom: 1f32,
            aspect: config.width as f32 / config.height as f32
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_projection(&camera);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: None
        });
        
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: None
        });

        let shader = device.create_shader_module(
            wgpu::include_wgsl!("shader.wgsl")
        );    

        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &camera_bind_group_layout
                ],
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
                        Vertex::description()
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
                    front_face: wgpu::FrontFace::Cw,
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
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            render_pipeline
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn redraw(&mut self) {
        self.resize(self.size);
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        let mut processed: bool = true;
        use WindowEvent::*;
        match event {
            MouseWheel { 
                delta: 
                    event::MouseScrollDelta::LineDelta(.., line_delta), 
                    .. 
            } => {
                let zoom = self.camera.zoom + line_delta * -0.1f32;
                let zoom = zoom.clamp(0.5f32, 2f32);
                self.camera.zoom = zoom;  
            },
            _ => { processed = false }
        }
        
        processed
    }

    fn update(&mut self, mesh: &Mesh) {
        self.index_count = mesh.indices.len() as u32;

        self.vertex_buffer = mesh.build_vertex_buffer(&self.device);
        self.index_buffer = mesh.build_index_buffer(&self.device);

        self.camera_uniform.update_projection(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform])
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

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