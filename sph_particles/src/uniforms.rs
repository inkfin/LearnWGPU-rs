use cgmath::prelude::*;

use crate::camera::Camera;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub dt: f32,
}

impl Uniforms {
    pub fn new() -> Self {
        Self { dt: 0.0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    eye_pos: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            eye_pos: [0.0; 4],
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
        self.eye_pos = [camera.eye.x, camera.eye.y, camera.eye.z, 1.0];
    }
}
