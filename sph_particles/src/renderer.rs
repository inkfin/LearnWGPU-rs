pub(crate) mod bind_group_layout_cache;
pub(crate) mod compute_pass_depth_filter;
pub(crate) mod compute_pass_particle;
pub(crate) mod render_pass_depth;
pub(crate) mod render_pass_water;

use std::sync::Arc;

pub(crate) use bind_group_layout_cache::BindGroupLayoutCache;

use compute_pass_particle::ComputeParticlePass;
use render_pass_depth::RenderDepthPass;
use render_pass_water::RenderQuadPass;

const RENDER_TARGET: i32 = 2;

const RENDER_PARTICLE_DEPTH: i32 = 0;
const RENDER_ALPHA: i32 = 1;
const RENDER_WATER: i32 = 2;

const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.01,
    g: 0.01,
    b: 0.01,
    a: 1.0,
};

use crate::{
    camera::{self, Camera},
    particle_system::ParticleState,
};

pub struct Renderer {
    pub render_depth_pass: RenderDepthPass,
    pub render_quad_pass: RenderQuadPass,
    pub compute_pass: ComputeParticlePass,
}

impl Renderer {
    pub async fn new(
        device: &wgpu::Device,
        camera: &Camera,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        let compute_state = ComputeParticlePass::new(device, bind_group_layout_cache).await;
        let render_depth_pass =
            RenderDepthPass::new(device, camera, surface_config, bind_group_layout_cache).await;

        let render_quad_pass =
            RenderQuadPass::new(device, camera, surface_config, bind_group_layout_cache).await;

        Self {
            render_depth_pass,
            render_quad_pass,
            compute_pass: compute_state,
        }
    }

    pub fn render(
        &mut self,
        dt: f32,
        particle_state: &mut ParticleState,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
    ) {
        futures::executor::block_on(self.compute_pass.sort_particle_data(
            device,
            queue,
            particle_state,
        ));
        self.compute_pass
            .compute_sph(device, queue, particle_state, dt);

        self.render_depth_pass
            .render(particle_state, device, queue, view);

        // self.compute_state.compute_filter

        // self.render_state.render_final
        self.render_quad_pass.render(
            &self.render_depth_pass.particle_depth_texture_bind_group,
            device,
            queue,
            view,
        );
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) {
        self.render_depth_pass
            .resize(device, surface_config, bind_group_layout_cache);
        self.render_quad_pass
            .resize(device, surface_config, bind_group_layout_cache);
    }

    pub fn update(
        &mut self,
        camera: &Camera,
        surface_config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
    ) {
        self.render_depth_pass
            .update_uniforms(camera, surface_config, queue);
        self.render_quad_pass
            .update_uniforms(camera, surface_config, queue)
    }
}
