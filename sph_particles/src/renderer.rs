pub(crate) mod compute;
pub(crate) mod compute_depth_filter;
pub(crate) mod gpu_cache;
pub(crate) mod render;

use std::sync::Arc;

use compute::{ComputeState, ComputeUniforms};
pub(crate) use gpu_cache::BindGroupLayoutCache;
use render::RenderState;

use crate::{
    camera::{self, Camera},
    particle_system::ParticleState,
};

pub struct Renderer {
    pub render_state: RenderState,
    pub compute_state: ComputeState,
}

impl Renderer {
    pub async fn new(
        device: &wgpu::Device,
        camera: &Camera,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        let compute_state = ComputeState::new(device, bind_group_layout_cache).await;
        let render_state =
            RenderState::new(device, camera, surface_config, bind_group_layout_cache).await;
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
            .compute_sph(device, queue, particle_state, dt);

        self.render_state
            .render_depth(particle_state, device, queue, view);

        // self.compute_state.compute_filter

        // self.render_state.render_final
        self.render_state.render_water(device, queue, view);

        // let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //     label: Some("staging_buffer"),
        //     size: depth_texture.texture.size().width as u64
        //         * depth_texture.texture.size().height as u64,
        //     usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        //     mapped_at_creation: false,
        // });

        // let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        //     label: Some("Texture Copy Encoder"),
        // });

        // encoder.copy_texture_to_buffer(
        //     wgpu::ImageCopyTexture {
        //         texture: &self.depth_texture.texture,
        //         mip_level: 0,
        //         origin: wgpu::Origin3d::ZERO,
        //         aspect: wgpu::TextureAspect::All,
        //     },
        //     wgpu::ImageCopyBuffer {
        //         buffer: &self.staging_buffer,
        //         layout: wgpu::ImageDataLayout {
        //             offset: 0,
        //             bytes_per_row: Some(4 * self.depth_texture.texture.size().width),
        //             rows_per_image: Some(self.depth_texture.texture.size().height),
        //         },
        //     },
        //     self.depth_texture.texture.size(),
        // );

        // queue.submit(Some(encoder.finish()));

        // staging_buffer
        //     .slice(..)
        //     .map_async(wgpu::MapMode::Read, move |result| {
        //         if result.is_ok() {
        //             let data = staging_buffer.slice(..).get_mapped_range();
        //             let data_array: &[f32] = bytemuck::cast_slice(&data);

        //             let (min_val, max_val) = data_array
        //                 .iter()
        //                 .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), &val| {
        //                     (min.min(val), max.max(val))
        //                 });

        //             println!("depth texture min: {}, max: {}", min_val, max_val);
        //             staging_buffer.unmap();
        //         } else {
        //             eprintln!("Failed to map the buffer");
        //         }
        //     });
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) {
        self.render_state
            .resize(device, surface_config, bind_group_layout_cache);
    }

    pub fn update(
        &mut self,
        camera: &Camera,
        surface_config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
    ) {
        self.render_state
            .update_uniforms(camera, surface_config, queue);
    }
}
