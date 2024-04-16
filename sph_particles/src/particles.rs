use std::ops::Range;

use cgmath::Vector3;
use wgpu::util::DeviceExt;

use crate::{render::BindGroupLayoutCache, vertex_data::ShaderVertexData};

const DEFAULT_SUPPORT_RADIUS: f32 = 0.5;
const DEFAULT_PARTICLE_RADIUS: f32 = 0.1;

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub pressure: f32,
    pub density: f32,
    pub mass: f32,
    pub support_radius: f32,
    pub ptype: u32, // 0: fluid, 1: boundary
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    position: [f32; 3],
    density: f32, // padding for 16 bytes
    velocity: [f32; 3],
    support_radius: f32,
    mass: f32,
    pressure: f32,
    ptype: u32,
    _pad: [f32; 1],
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            pressure: 0.0,
            density: 1.0,
            support_radius: DEFAULT_SUPPORT_RADIUS,
            mass: 0.0,
            ptype: 0,
        }
    }
}

/// Defines the particle structure @location() in particle storage buffer
pub enum ParticleDataShaderLocation {
    Position = 0,
    Density = 1,
    Velocity = 2,
    SupportRadius = 3,
    Mass = 4,
    Pressure = 5,
    PType = 6,
}

impl ShaderVertexData for Particle {
    type RawType = ParticleRaw;
    fn to_raw(&self) -> ParticleRaw {
        ParticleRaw {
            position: self.position.into(),
            velocity: self.velocity.into(),
            pressure: self.pressure,
            density: self.density,
            support_radius: self.support_radius,
            mass: self.mass,
            ptype: self.ptype,
            _pad: [0.0],
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
                    shader_location: ParticleDataShaderLocation::Position as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: ParticleDataShaderLocation::Density as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: ParticleDataShaderLocation::Velocity as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: ParticleDataShaderLocation::SupportRadius as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: ParticleDataShaderLocation::Mass as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: mem::size_of::<[f32; 9]>() as wgpu::BufferAddress,
                    shader_location: ParticleDataShaderLocation::Pressure as u32,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: mem::size_of::<[f32; 10]>() as wgpu::BufferAddress,
                    shader_location: ParticleDataShaderLocation::PType as u32,
                },
            ],
        }
    }
}

pub struct ParticleState {
    pub particle_list: Vec<Particle>,

    // wgpu state
    pub particle_buffers: Vec<wgpu::Buffer>,
    pub particle_render_bind_group: wgpu::BindGroup,
    pub particle_compute_bind_group: wgpu::BindGroup,
}

impl ParticleState {
    pub fn new(device: &wgpu::Device, bind_group_layout_cache: &BindGroupLayoutCache) -> Self {
        // --------------------------------------
        // Init particles

        tracing::info!(
            "ParticleRaw Size: {}",
            std::mem::size_of::<ParticleRaw>() as wgpu::BufferAddress
        );
        // let particle_list = vec![
        //     Particle {
        //         position: Vector3::new(4.0, 5.0, 0.0),
        //         particle_radius: 0.3,
        //         ptype: 1,
        //         ..Default::default()
        //     },
        //     Particle {
        //         position: Vector3::new(5.0, 5.0, 0.0),
        //         ptype: 1,
        //         ..Default::default()
        //     },
        //     Particle {
        //         position: Vector3::new(6.0, 5.0, 0.0),
        //         ptype: 1,
        //         ..Default::default()
        //     },
        //     Particle {
        //         position: Vector3::new(5.0, 6.0, 0.0),
        //         ptype: 1,
        //         ..Default::default()
        //     },
        // ];

        // fluids
        let mut particle_list = get_particles_2d(
            (1.0, 0.8),
            (6.0, 6.8),
            true,
            1000.0,
            None,
            DEFAULT_SUPPORT_RADIUS,
        );

        // walls
        particle_list.append(&mut get_particles_2d(
            (0.0, 0.0),
            (10.0, 0.4),
            false,
            1000.0,
            None,
            DEFAULT_SUPPORT_RADIUS,
        ));

        particle_list.append(&mut get_particles_2d(
            (0.0, 0.4),
            (0.4, 10.0),
            false,
            1000.0,
            None,
            DEFAULT_SUPPORT_RADIUS,
        ));

        particle_list.append(&mut get_particles_2d(
            (9.6, 0.4),
            (10.0, 10.0),
            false,
            1000.0,
            None,
            DEFAULT_SUPPORT_RADIUS,
        ));

        // ---------------------------------------

        let particle_data = particle_list
            .iter()
            .map(Particle::to_raw)
            .collect::<Vec<_>>();

        let particle_buffers = vec![
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Buffer"),
                contents: bytemuck::cast_slice(&particle_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }),
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Buffer"),
                contents: bytemuck::cast_slice(&particle_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }),
        ];

        let particle_compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Particle Bind Group"),
            layout: &bind_group_layout_cache.particle_compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0, // @binding(0) read
                    resource: particle_buffers[0].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1, // @binding(1) write
                    resource: particle_buffers[1].as_entire_binding(),
                },
            ],
        });

        let particle_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Particle Bind Group"),
            layout: &bind_group_layout_cache.particle_render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffers[0].as_entire_binding(),
            }],
        });

        Self {
            particle_list,
            particle_buffers,
            particle_compute_bind_group,
            particle_render_bind_group,
        }
    }

    pub fn swap_compute_buffers(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) {
        self.particle_buffers.swap(0, 1);
        self.particle_compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Particle Bind Group"),
            layout: &bind_group_layout_cache.particle_compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0, // @binding(0) read
                    resource: self.particle_buffers[0].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1, // @binding(1) write
                    resource: self.particle_buffers[1].as_entire_binding(),
                },
            ],
        });
        self.particle_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Particle Bind Group"),
            layout: &bind_group_layout_cache.particle_render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.particle_buffers[0].as_entire_binding(),
            }],
        });
    }
}

pub trait ComputeParticle<'a> {
    fn compute_particle(
        &mut self,
        workgroup_size: (u32, u32, u32),
        uniforms_bind_group: &'a wgpu::BindGroup,
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
        uniforms_bind_group: &'a wgpu::BindGroup,
        particle_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_bind_group(0, uniforms_bind_group, &[]);
        self.set_bind_group(1, particle_bind_group, &[]);
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

/// fill in particles in the given range
fn get_particles_2d(
    lower_bound: (f32, f32),
    upper_bound: (f32, f32),
    is_fluid: bool,
    density: f32,
    velocity: Option<Vector3<f32>>,
    support_radius: f32,
) -> Vec<Particle> {
    let mut particles = Vec::new();

    let mut x_coord = lower_bound.0;
    let mut y_coord = lower_bound.1;
    while x_coord <= upper_bound.0 {
        while y_coord <= upper_bound.1 {
            let p = Particle {
                position: Vector3::new(x_coord, y_coord, 0.0),
                ptype: if is_fluid { 0 } else { 1 },
                density,
                velocity: velocity.unwrap_or(Vector3::new(0.0, 0.0, 0.0)),
                support_radius,
                ..Default::default()
            };

            particles.push(p);

            y_coord += DEFAULT_PARTICLE_RADIUS * 2.0;
        }
        x_coord += DEFAULT_PARTICLE_RADIUS * 2.0;
        y_coord = lower_bound.1;
    }

    particles
}
