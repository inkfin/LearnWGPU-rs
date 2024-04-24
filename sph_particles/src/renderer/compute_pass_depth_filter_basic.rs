//! This module apply bileteral filter to depth texture
use crate::texture::Texture;
use wgpu::{
    util::{DeviceExt, StagingBelt},
    BindGroupEntry, BindGroupLayoutEntry,
};
// TODO: FINISH this module

use crate::resources::load_shader;

use super::BindGroupLayoutCache;

pub struct ComputeDepthFilterBasicPass {
    compute_pipeline: wgpu::ComputePipeline,
}

impl ComputeDepthFilterBasicPass {
    pub fn filter(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        camera_bind_group: &wgpu::BindGroup,
        depth_texture_read_bind_group: &wgpu::BindGroup,
        depth_texture_write_bind_group: &wgpu::BindGroup,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let (x, y, z) = (
            (config.width as f32 / 16.0).ceil() as u32,
            (config.height as f32 / 16.0).ceil() as u32,
            1,
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Depth Filter Basic Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Depth Filter Basic Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, camera_bind_group, &[]);
            compute_pass.set_bind_group(1, depth_texture_read_bind_group, &[]);
            compute_pass.set_bind_group(2, depth_texture_write_bind_group, &[]);
            compute_pass.dispatch_workgroups(x, y, z);
        }
        queue.submit(Some(encoder.finish()));
    }

    pub async fn new(
        device: &wgpu::Device,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        let compute_pipeline = build_shader(
            vec![
                &bind_group_layout_cache.camera_bind_group_layout,
                &bind_group_layout_cache.sampled_depth_texture_read_bind_group_layout,
                &bind_group_layout_cache.sampled_depth_texture_write_bind_group_layout,
            ],
            device,
        )
        .await;

        Self {
            compute_pipeline,
        }
    }
}

async fn build_shader(
    bind_group_layouts: Vec<&wgpu::BindGroupLayout>,
    device: &wgpu::Device,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(
        load_shader("compute_depth_filter_basic.wgsl")
            .await
            .unwrap(),
    );

    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Pipeline Layout"),
        bind_group_layouts: &bind_group_layouts[..],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &shader,
        entry_point: "main",
    });

    compute_pipeline
}
