mod mesh;
mod camera;
mod state;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, dpi::PhysicalPosition,
};

pub(crate) async fn run(mut simulation: crate::simulation::Simulation) {
    env_logger::init();

    let mut mouse_position: Option<winit::dpi::PhysicalPosition<f64>> = None;
    let mut mouse_position_offset = cgmath::Point2::new(0.0f32, 0.0f32);
    let mut mouse_panning_toggle = false;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Initialize mesh
    let mut mesh = mesh::Mesh::new(&simulation);
    
    let mut state = state::State::new(&window).await;
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                // Advance simulation, update mesh to reflect new position, and render changes
                simulation.update();
                mesh.update_from_simulation(&simulation);
                state.update(&mesh);

                match state.render() {
                    Ok(..) => {  },
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e)
                }
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            },
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => if !state.input(event) {
                match event {
                    // Handle close behavior
                    WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,

                    // Zoom controls on MouseWheel
                    WindowEvent::MouseWheel { delta, .. } => {
                        if let MouseScrollDelta::LineDelta(.., mut line_delta) = delta {
                            line_delta *= -0.1f32;

                            // TODO: Maybe Camera should be pulled out of State and used alongside it
                            state.camera.zoom(line_delta);                            
                        }
                    },

                    // Panning using click and drag 
                    WindowEvent::CursorMoved { 
                        position,
                        .. 
                    } => {
                        mouse_position = Some(*position);
                    }
                    WindowEvent::MouseInput { 
                        state: ElementState::Pressed, 
                        button: MouseButton::Left, 
                        ..
                    } => {
                        mouse_panning_toggle = true;
                        mouse_position_offset.x = state.camera.pos.x + (mouse_position.unwrap().x as f32 / state.size.width as f32) * 2f32 - 1f32;
                        mouse_position_offset.y = (mouse_position.unwrap().y as f32 / state.size.height as f32) * 2f32 - 1f32 - state.camera.pos.y;
                    },
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: MouseButton::Left,
                        ..
                    } => {
                        mouse_panning_toggle = false;
                    },

                    // Resizing
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size)
                    },
                    
                    // Adjust inner size
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => { 
                        state.resize(**new_inner_size) 
                    },
                    _ => {}
                }
            },
            _ => {}
        }
        
        if mouse_panning_toggle {
            let mut relative_position = cgmath::Point2::new(
                (mouse_position.unwrap().x as f32 / state.size.width as f32) * 2f32 - 1f32,
                (mouse_position.unwrap().y as f32 / state.size.height as f32) * 2f32 - 1f32
            );

            relative_position.x -= mouse_position_offset.x;
            relative_position.x *= -1f32;

            relative_position.y -= mouse_position_offset.y;

            state.camera.pan(relative_position);
        }
    });
}
