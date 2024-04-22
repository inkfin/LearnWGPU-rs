pub(crate) mod bind_group_layout_cache;
pub(crate) mod compute_pass_copy_depth;
pub(crate) mod compute_pass_depth_filter;
pub(crate) mod compute_pass_particle;
pub(crate) mod render_pass_depth;
pub(crate) mod render_pass_water;

use std::sync::Arc;

pub(crate) use bind_group_layout_cache::BindGroupLayoutCache;

use compute_pass_copy_depth::CopyDepthPass;
use compute_pass_particle::ComputeParticlePass;
use render_pass_depth::RenderDepthPass;
use render_pass_water::RenderQuadPass;

const RENDER_TARGET: i32 = 2;

const RENDER_PARTICLE_DEPTH: i32 = 0;
// TODO: implement this
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
    pub compute_particle_pass: ComputeParticlePass,
    pub copy_depth_pass: CopyDepthPass,
}

impl Renderer {
    pub async fn new(
        device: &wgpu::Device,
        camera: &Camera,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        let compute_particle_pass = ComputeParticlePass::new(device, bind_group_layout_cache).await;

        let copy_depth_pass =
            CopyDepthPass::new(device, surface_config, bind_group_layout_cache).await;

        let render_depth_pass =
            RenderDepthPass::new(device, camera, surface_config, bind_group_layout_cache).await;

        let render_quad_pass =
            RenderQuadPass::new(device, camera, surface_config, bind_group_layout_cache).await;

        Self {
            render_depth_pass,
            render_quad_pass,
            compute_particle_pass,
            copy_depth_pass,
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
        futures::executor::block_on(self.compute_particle_pass.sort_particle_data(
            device,
            queue,
            particle_state,
        ));
        self.compute_particle_pass
            .compute_sph(device, queue, particle_state, dt);

        self.render_depth_pass
            .render(particle_state, device, queue, view);

        self.copy_depth_pass.compute(
            &self.render_depth_pass.particle_depth_texture_bind_group,
            device,
            queue,
        );

        let (depth_read_bg_0, depth_read_bg_1, depth_write_bg_0, depth_write_bg_1) = (
            &self.copy_depth_pass.sampled_depth_texture_read_bind_group_0,
            &self.copy_depth_pass.sampled_depth_texture_read_bind_group_1,
            &self
                .copy_depth_pass
                .sampled_depth_texture_write_bind_group_0,
            &self
                .copy_depth_pass
                .sampled_depth_texture_write_bind_group_1,
        );

        // self.compute_state.compute_filter

        // self.render_state.render_final
        self.render_quad_pass
            .render(depth_read_bg_0, device, queue, view);
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
        self.copy_depth_pass
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
