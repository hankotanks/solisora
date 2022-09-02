use cgmath::{Point2, Point3, SquareMatrix, Vector2};
use cgmath::Matrix4;

pub(super) struct Camera {
    pub(super) pos: Point2<f32>,
    zoom: f32,
    aspect: f32
}

impl Camera {
    const FOVY: f32 = 45.0;
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
            aspect
        }
    }

    pub(super) fn pan(&mut self, pos: Point2<f32>) {
        self.pos = pos;
    }

    pub(super) fn zoom(&mut self, delta: f32) {
        self.zoom = (self.zoom + delta).clamp(0.2f32, 5f32);
    }

    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let pos = Point3::new(self.pos.x, self.pos.y, self.zoom);
        let target = Point3::new(self.pos.x, self.pos.y, 0.0);

        let view = Matrix4::look_at_rh(pos, target, cgmath::Vector3::unit_y());

        let projection = cgmath::perspective(
            cgmath::Deg(Self::FOVY),
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