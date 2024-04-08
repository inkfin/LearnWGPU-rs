use std::ops::Range;

use cgmath::{prelude::*, Vector3};
use wgpu::util::DeviceExt;

use crate::{
    render::BindGroupLayoutCache,
    vertex_data::{BindGroupIndex, ShaderVertexData, VertexDataLocation},
};

pub const PARTICLE_MAX_SIZE: usize = 1048576; // 2^20

pub struct ParticleState {
    pub particle_list: Vec<Particle>,

    pub particle_buffer: wgpu::Buffer,
    pub render_bind_group: wgpu::BindGroup,
    pub compute_bind_group: wgpu::BindGroup,
}

impl ParticleState {
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout_cache: &BindGroupLayoutCache,
        size: usize,
    ) -> Self {
        // --------------------------------------
        // Init particles
        // let particle_list = vec![Particle::default(); size.min(PARTICLE_MAX_SIZE)];
        let particle_list = vec![
            Particle {
                position: Vector3::new(-1.0, 0.0, 0.0),
                ..Default::default()
            },
            Particle {
                position: Vector3::new(0.0, 0.0, 0.0),
                ..Default::default()
            },
            Particle {
                position: Vector3::new(1.0, 1.0, 0.0),
                ..Default::default()
            },
        ];
        // ---------------------------------------

        let particle_data = particle_list
            .iter()
            .map(Particle::to_raw)
            .collect::<Vec<_>>();

        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(&particle_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Particle Bind Group"),
            layout: &bind_group_layout_cache.particle_compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            }],
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Particle Bind Group"),
            layout: &bind_group_layout_cache.particle_render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            }],
        });

        Self {
            particle_list,
            particle_buffer,
            compute_bind_group,
            render_bind_group,
        }
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
    density: f32, // padding for 16 bytes
    velocity: [f32; 3],
    support_radius: f32,
    force: [f32; 3],
    particle_radius: f32,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            force: Vector3::new(0.0, 0.0, 0.0),
            density: 1.0,
            support_radius: 0.5,
            particle_radius: 0.1,
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
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: VertexDataLocation::Density as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: VertexDataLocation::Velocity as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: VertexDataLocation::SupportRadius as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: VertexDataLocation::Force as u32,
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
        workgroup_size: (u32, u32, u32),
        particle_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_bind_group(0, particle_bind_group, &[]);
        self.insert_debug_marker("compute particle");
        self.dispatch_workgroups(workgroup_size.0, workgroup_size.1, workgroup_size.2);
    }
}

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
