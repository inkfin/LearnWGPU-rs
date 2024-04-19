use std::ops::Range;

/// draw pass
pub trait DrawParticle<'a> {
    fn draw_particle_instanced(
        &mut self,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        particle_size: u32,
        particle_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a> DrawParticle<'a> for wgpu::RenderPass<'a> {
    fn draw_particle_instanced(
        &mut self,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        particle_size: u32,
        particle_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, particle_bind_group, &[]);
        // Hack: use attributeless rendering, * 6 because we have 6 vertices
        self.draw(0..(particle_size * 6), instances);
    }
}

/// compute pass
pub trait ComputeParticle<'a> {
    fn compute_particle(
        &mut self,
        workgroup_size: (u32, u32, u32),
        particle_bind_group_0: &'a wgpu::BindGroup,
        particle_bind_group_1: &'a wgpu::BindGroup,
        uniforms_bind_group: &'a wgpu::BindGroup,
        world_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> ComputeParticle<'b> for wgpu::ComputePass<'a>
where
    'b: 'a,
{
    fn compute_particle(
        &mut self,
        workgroup_size: (u32, u32, u32),
        particle_bind_group_0: &'a wgpu::BindGroup,
        particle_bind_group_1: &'a wgpu::BindGroup,
        uniforms_bind_group: &'a wgpu::BindGroup,
        world_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_bind_group(0, particle_bind_group_0, &[]);
        self.set_bind_group(1, particle_bind_group_1, &[]);
        self.set_bind_group(2, uniforms_bind_group, &[]);
        self.set_bind_group(3, world_bind_group, &[]);
        self.insert_debug_marker("compute particle");
        self.dispatch_workgroups(workgroup_size.0, workgroup_size.1, workgroup_size.2);
    }
}