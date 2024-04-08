use wgpu::util::DeviceExt;

use crate::particles::ParticleState;

const WORKGROUP_SIZE: (u32, u32, u32) = (64, 64, 64);

pub struct ComputeState {
    pub shader: wgpu::ShaderModule,

    pub particle_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,

    pub pipeline_layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::ComputePipeline,
}

impl ComputeState {
    pub fn new(
        device: &wgpu::Device,
        particle_state: &ParticleState,
        bind_group_layout_cache: &super::render::BindGroupLayoutCache,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shader/compute_particle.wgsl").into(),
            ),
        });

        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(&particle_state.get_particle_data()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &bind_group_layout_cache.particle_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(particle_buffer.as_entire_buffer_binding()),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &bind_group_layout_cache.get_compute_particles_layouts(),
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
            particle_buffer,
            bind_group,
            pipeline_layout,
            pipeline,
        }
    }

    pub fn compute_pass_particle(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);

        compute_pass.dispatch_workgroups(WORKGROUP_SIZE.0, WORKGROUP_SIZE.1, WORKGROUP_SIZE.2)
    }
}
