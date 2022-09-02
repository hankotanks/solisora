mod mesh;
mod camera;
mod state;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder
};

pub(crate) async fn run(mut simulation: crate::simulation::Simulation) {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Initialize
    let mut mesh = mesh::Mesh::new(&simulation);
    let mut state = state::State::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                // Advance simulation, update mesh to reflect new position, and render changes
                simulation.update();
                mesh.handle_simulation_update(&simulation);

                state.update(&mesh);

                match state.render() {
                    Ok(..) => {  },
                    Err(wgpu::SurfaceError::Lost) => state.redraw(),
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
    });
}
