use cgmath::{Point2, Point3, SquareMatrix};
use cgmath::Matrix4;

pub(super) struct Camera {
    pub(super) eye: Point2<f32>,
    pub(super) aspect: f32
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

    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let eye = Point3::new(self.eye.x, self.eye.y, 1.0);
        let target = Point3::new(self.eye.x, self.eye.y, 0.0);

        let view = Matrix4::look_at_rh(eye, target, cgmath::Vector3::unit_y());

        Self::MATRIX_CORRECTION_FOR_WGPU * view
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