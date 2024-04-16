use wgpu::util::DeviceExt;

use crate::{
    particles::ParticleState,
    uniforms::{ComputeStage, Uniforms},
};

use super::resources::load_shader;

const WORKGROUP_SIZE: (u32, u32, u32) = (4096, 1024, 1); // total 4194304

pub struct ComputeState {
    #[allow(dead_code)]
    pub shader: wgpu::ShaderModule,

    #[allow(dead_code)]
    pub pipeline_layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::ComputePipeline,

    pub uniforms_data: Uniforms,
    pub uniforms_buffer: wgpu::Buffer,
    pub uniforms_bind_group: wgpu::BindGroup,
}

impl ComputeState {
    pub async fn new(
        device: &wgpu::Device,
        bind_group_layout_cache: &super::render::BindGroupLayoutCache,
    ) -> Self {
        let shader =
            device.create_shader_module(load_shader("compute_particle_2d.wgsl").await.unwrap());

        let uniforms_data = Uniforms::new();

        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[uniforms_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniforms Bind Group"),
            layout: &bind_group_layout_cache.uniforms_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[
                &bind_group_layout_cache.uniforms_bind_group_layout,
                &bind_group_layout_cache.particle_compute_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "cs_main",
        });

        Self {
            shader,
            pipeline_layout,
            pipeline,
            uniforms_data,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub fn compute(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout_cache: &super::render::BindGroupLayoutCache,
        particle_state: &mut ParticleState,
        dt: f32,
    ) {
        let mut current_stage = ComputeStage::ComputeDensities;

        loop {
            self.uniforms_data.compute_stage = current_stage as u32;
            self.uniforms_data.dt = dt;
            queue.write_buffer(
                &self.uniforms_buffer,
                0,
                bytemuck::cast_slice(&[self.uniforms_data]),
            );

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Encoder"),
            });
            {
                let mut compute_pass =
                    encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

                compute_pass.set_pipeline(&self.pipeline);

                use super::particles::ComputeParticle;
                compute_pass.compute_particle(
                    WORKGROUP_SIZE,
                    &self.uniforms_bind_group,
                    &particle_state.particle_compute_bind_group,
                );
            }

            queue.submit(Some(encoder.finish()));

            // swap compute buffers after rendering
            particle_state.swap_compute_buffers(device, bind_group_layout_cache);

            // change state
            match current_stage {
                ComputeStage::ComputeDensities => {
                    current_stage = ComputeStage::ComputeNonPressureForces;
                }
                ComputeStage::ComputeNonPressureForces => {
                    current_stage = ComputeStage::ComputePressureForces;
                }
                ComputeStage::ComputePressureForces => {
                    current_stage = ComputeStage::Advect;
                }
                ComputeStage::Advect => {
                    break;
                }
            }
        }
    }
}
