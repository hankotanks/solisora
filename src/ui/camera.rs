use cgmath::{
    Point2,
    Point3,
    Matrix4, SquareMatrix
};

pub(super) struct Camera {
    pub(super) pos: Point2<f32>,
    pub(super) zoom: f32,
    pub(super) aspect: f32
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

