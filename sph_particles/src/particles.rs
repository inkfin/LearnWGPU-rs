use std::ops::Range;

use cgmath::{prelude::*, Vector3};

use crate::{
    model::{Mesh, ModelVertex},
    uniforms::{self, BindGroupIndex},
};

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub density: f32,
    pub pressure: f32,
    pub force: Vector3<f32>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawParticle {
    position: [f32; 3],
    velocity: [f32; 3],
    density: f32,
    pressure: f32,
    force: [f32; 3],
    _padding: f32,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            density: 0.0,
            pressure: 0.0,
            force: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

impl Particle {
    pub fn to_raw(&self) -> RawParticle {
        RawParticle {
            position: self.position.into(),
            velocity: self.velocity.into(),
            density: self.density,
            pressure: self.pressure,
            force: self.force.into(),
            _padding: 0.0,
        }
    }
}

pub trait ComputeParticle<'a> {
    fn compute_particle(
        &mut self,
        particle: &'a Particle,
        workgroup_size: (u32, u32, u32),
        particle_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> ComputeParticle<'b> for wgpu::ComputePass<'a>
where
    'b: 'a,
{
    fn compute_particle(
        &mut self,
        particle: &'b Particle,
        workgroup_size: (u32, u32, u32),
        particle_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_bind_group(
            BindGroupIndex::ParticleBuffer as u32,
            particle_bind_group,
            &[],
        );
        self.insert_debug_marker("compute particle");
        self.dispatch_workgroups(workgroup_size.0, workgroup_size.1, workgroup_size.2);
    }
}

pub trait DrawParticle<'a> {
    fn draw_particle_instanced(
        &mut self,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        particle_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a> DrawParticle<'a> for wgpu::RenderPass<'a> {
    fn draw_particle_instanced(
        &mut self,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        particle_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_bind_group(
            BindGroupIndex::CameraUniforms as u32,
            camera_bind_group,
            &[],
        );
        self.set_bind_group(
            BindGroupIndex::ParticleBuffer as u32,
            particle_bind_group,
            &[],
        );
        self.draw_indexed(0..6, 0, instances);
    }
}
