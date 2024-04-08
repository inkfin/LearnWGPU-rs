use std::ops::Range;

use cgmath::{prelude::*, Vector3};

use crate::vertex_data::{BindGroupIndex, ShaderVertexData, VertexDataLocation};

pub const PARTICLE_MAX_SIZE: usize = 1048576; // 2^20

pub struct ParticleState {
    pub particle_list: Vec<Particle>,
}

impl ParticleState {
    pub fn new(size: usize) -> Self {
        // --------------------------------------
        // Init particles
        let particle_list = vec![Particle::default(); size.min(PARTICLE_MAX_SIZE)];
        // ---------------------------------------
        Self { particle_list }
    }

    pub fn get_particle_data(&self) -> Vec<ParticleRaw> {
        self.particle_list
            .iter()
            .map(Particle::to_raw)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub force: Vector3<f32>,
    pub density: f32,
    pub support_radius: f32,
    pub particle_radius: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    position: [f32; 3],
    velocity: [f32; 3],
    force: [f32; 3],
    density: f32,
    support_radius: f32,
    particle_radius: f32,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            force: Vector3::new(0.0, 0.0, 0.0),
            density: 0.0,
            support_radius: 0.0,
            particle_radius: 0.0,
        }
    }
}

impl ShaderVertexData for Particle {
    type RawType = ParticleRaw;
    fn to_raw(&self) -> ParticleRaw {
        ParticleRaw {
            position: self.position.into(),
            velocity: self.velocity.into(),
            force: self.force.into(),
            density: self.density,
            support_radius: self.support_radius,
            particle_radius: self.particle_radius,
        }
    }

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ParticleRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: VertexDataLocation::Position as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: VertexDataLocation::Velocity as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: VertexDataLocation::Force as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 9]>() as wgpu::BufferAddress,
                    shader_location: VertexDataLocation::Density as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 10]>() as wgpu::BufferAddress,
                    shader_location: VertexDataLocation::SupportRadius as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: VertexDataLocation::ParticleRadius as u32,
                },
            ],
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
        _particle: &'b Particle,
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
