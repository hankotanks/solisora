use cgmath::{
    Point2,
    Point3,
    Matrix4,
    SquareMatrix
};

use winit::event;
use winit::dpi::PhysicalSize;

use super::mesh;

pub(super) struct Camera {
    pub(super) pos: Point2<f32>,
    zoom: f32,
    aspect: f32,
    following: Option<usize>
}

impl Camera {
    const FOV: f32 = 45.0;
    const ZNEAR: f32 = 0.1;
    const ZFAR: f32 = 100.0;

    const MATRIX_CORRECTION_FOR_WGPU: Matrix4<f32> = Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    );

    pub(super) fn new(aspect: f32) -> Self {
        Self {
            pos: Point2::new(0.0f32, 0.0f32),
            zoom: 1f32,
            aspect,
            following: None
        }
    }

    pub(super) fn pan(&mut self, pos: Point2<f32>) {
        // only allow panning if the camera isn't tracking a planet
        if self.following.is_none() {
            self.pos = pos;
        }
    }

    pub(super) fn pan_to_target(&mut self, mesh: &mesh::Mesh) {
        if let Some(target_index) = self.following {
            if let Some(point) =  mesh.origin(target_index) {
                self.pos = point;
            }
        }
    }

    pub(super) fn zoom(&mut self, delta: f32) {
        self.zoom = (self.zoom + delta).clamp(0.2f32, 5f32);
    }

    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let pos = Point3::new(self.pos.x, self.pos.y, self.zoom);
        let target = Point3::new(self.pos.x, self.pos.y, 0.0);

        let view = Matrix4::look_at_rh(pos, target, cgmath::Vector3::unit_y());

        let projection = cgmath::perspective(
            cgmath::Deg(Self::FOV),
            self.aspect,
            Self::ZNEAR,
            Self::ZFAR
        );

        Self::MATRIX_CORRECTION_FOR_WGPU * projection * view
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct CameraUniform {
    projection: [[f32; 4]; 4]
}

impl CameraUniform {
    pub(super) fn new() -> Self {
        Self {
            projection: Matrix4::identity().into()
        }
    }

    pub(super) fn update_projection(&mut self, camera: &Camera) {
        self.projection = camera.build_view_projection_matrix().into();
    }
}

pub(super) struct CameraController {
    mouse_position: Option<winit::dpi::PhysicalPosition<f64>>,
    mouse_position_offset: Point2<f32>,
    size: PhysicalSize<u32>,
    toggle: bool
}

impl CameraController {
    pub(super) fn new(size: &PhysicalSize<u32>) -> Self {
        Self {
            mouse_position: None,
            mouse_position_offset: (0f32, 0f32).into(),
            size: *size,
            toggle: false
        }
    }

    pub(super) fn handle_mouse_events(&mut self, camera: &mut Camera, event: &event::WindowEvent) {
        use event::WindowEvent::*;
        match event {
            MouseWheel { 
                delta: 
                    event::MouseScrollDelta::LineDelta(.., mut line_delta), 
                    .. 
            } => {
                // Zoom controls on MouseWheel
                line_delta *= -0.1f32;

                camera.zoom(line_delta);                            
            },
            CursorMoved { position, .. } => {
                // Update PhysicalPosition of the cursor within the window
                self.mouse_position = Some(*position);
            },
            MouseInput { 
                state: event::ElementState::Pressed, 
                button: event::MouseButton::Left, 
                ..
            } => {
                // When the user starts to drag
                self.toggle = true;

                self.mouse_position_offset = self.physical_position_to_clip_space();
                self.mouse_position_offset.x += camera.pos.x;
                self.mouse_position_offset.y -= camera.pos.y;
            },
            MouseInput {
                state: event::ElementState::Released,
                button: event::MouseButton::Left,
                ..
            } => {
                // Releasing the drag
                self.toggle = false;
            },
            _ => {  }
        }

        if self.toggle {
            camera.pan(
                {
                    let mut p = self.physical_position_to_clip_space();

                    p.x -= self.mouse_position_offset.x;
                    p.x *= -1f32;
                    p.y -= self.mouse_position_offset.y;
                    p
                }
            );
        }
    }

    pub(super) fn handle_resize(&mut self, size: &PhysicalSize<u32>) {
        self.size = *size;
    }

    fn physical_position_to_clip_space(&self) -> Point2<f32> {
        if let Some(pos) = self.mouse_position {
            return Point2::new(
                (pos.x as f32 / self.size.width as f32) * 2f32 - 1f32,
                (pos.y as f32 / self.size.height as f32) * 2f32 - 1f32
            );
        }
            
        panic!()
    }
}