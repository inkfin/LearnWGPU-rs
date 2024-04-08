use crate::particles::ParticleState;

const WORKGROUP_SIZE: (u32, u32, u32) = (64, 64, 64);

pub struct ComputeState {
    pub shader: wgpu::ShaderModule,

    pub pipeline_layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::ComputePipeline,
}

impl ComputeState {
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout_cache: &super::render::BindGroupLayoutCache,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shader/compute_particle.wgsl").into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout_cache.particle_compute_bind_group_layout],
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
        }
    }

    pub fn compute_pass_particle(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        particle_state: &ParticleState,
    ) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

        compute_pass.set_pipeline(&self.pipeline);

        use super::particles::ComputeParticle;
        compute_pass.compute_particle(WORKGROUP_SIZE, &particle_state.compute_bind_group);
    }
}
