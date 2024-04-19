//! This module contains materials needed for rendering

pub struct Materials {
    pub skybox: wgpu::Texture,
    pub albedo: wgpu::Texture,
    pub roughness: wgpu::Texture,
}
