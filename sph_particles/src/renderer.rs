pub(crate) mod compute;
pub(crate) mod gpu_cache;
pub(crate) mod render;

use std::sync::Arc;

use compute::{ComputeState, ComputeUniforms};
pub(crate) use gpu_cache::BindGroupLayoutCache;
use render::RenderState;

use crate::particle_system::ParticleState;

pub struct Renderer {
    pub render_state: RenderState,
    pub compute_state: ComputeState,
}

impl Renderer {
    pub async fn new(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        let compute_state = ComputeState::new(device, bind_group_layout_cache).await;
        let render_state = RenderState::new(device, surface_config, bind_group_layout_cache).await;
        Self {
            render_state,
            compute_state,
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
        futures::executor::block_on(self.compute_state.sort_particle_data(
            device,
            queue,
            particle_state,
        ));
        self.compute_state
            .compute(device, queue, particle_state, dt);

        self.render_state
            .render(device, queue, view, particle_state);
    }
}
