use wgpu::util::DeviceExt;

use crate::{
    particles::{ParticleState, WorldData},
    uniforms::Uniforms,
};

use super::resources::load_shader;

const WORKGROUP_SIZE: (u32, u32, u32) = (512, 256, 1); // total 4194304

pub struct ComputeState {
    #[allow(dead_code)]
    pub shader: wgpu::ShaderModule,

    #[allow(dead_code)]
    pub pipeline_layout: wgpu::PipelineLayout,
    pub compute_density_pipeline: wgpu::ComputePipeline,
    pub compute_non_pressure_pipeline: wgpu::ComputePipeline,
    pub compute_pressure_pipeline: wgpu::ComputePipeline,
    pub advect_pipeline: wgpu::ComputePipeline,
    #[allow(dead_code)]
    pub empty_copy_pipeline: wgpu::ComputePipeline,

    // uniforms data and buffer
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

        //---------------------------------------------------------------------
        // Pipeline Setup

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[
                &bind_group_layout_cache.particle_compute_bind_group_layout,
                &bind_group_layout_cache.particle_compute_bind_group_layout,
                &bind_group_layout_cache.uniforms_bind_group_layout,
                &bind_group_layout_cache.world_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        // shader entries:
        let compute_density_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "compute_density_main",
            });

        let compute_non_pressure_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "compute_non_pressure_main",
            });

        let compute_pressure_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "compute_pressure_main",
            });

        let advect_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "advect_main",
        });

        let empty_copy_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "empty_copy_main",
            });

        Self {
            shader,
            pipeline_layout,
            compute_density_pipeline,
            compute_non_pressure_pipeline,
            compute_pressure_pipeline,
            advect_pipeline,
            empty_copy_pipeline,
            uniforms_data,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub async fn sort_particle_data(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        particle_state: &mut ParticleState,
    ) {
        particle_state
            .dump_particle_data_from_gpu(0, device, queue)
            .await;
        // sort particle data
        particle_state.particle_data.sort_by(|a, b| {
            let a = a.cell_id;
            let b = b.cell_id;
            a.cmp(&b)
        });

        particle_state.upload_particle_data_to_gpu(queue);
    }

    pub fn compute(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        particle_state: &mut ParticleState,
        dt: f32,
    ) {
        // update uniform buffer before dispatching compute
        self.uniforms_data.dt = dt;
        queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms_data]),
        );

        let (mut src_bind_group, mut dst_bind_group) = (
            &particle_state.particle_compute_bind_group_0,
            &particle_state.particle_compute_bind_group_1,
        );

        // add empty copy pipeline if total size is odd
        let pipeline_list = [
            &self.compute_density_pipeline,
            &self.compute_non_pressure_pipeline,
            &self.compute_pressure_pipeline,
            &self.advect_pipeline,
        ];

        for &pipeline in pipeline_list.iter() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Encoder"),
            });

            {
                let mut compute_pass =
                    encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                compute_pass.set_pipeline(pipeline);

                use super::particles::ComputeParticle;
                compute_pass.compute_particle(
                    WORKGROUP_SIZE,
                    src_bind_group,
                    dst_bind_group,
                    &self.uniforms_bind_group,
                    &particle_state.world_bind_group,
                );
            }
            queue.submit(Some(encoder.finish()));

            // swap buffers
            (src_bind_group, dst_bind_group) = (dst_bind_group, src_bind_group);
        }
    }
}
