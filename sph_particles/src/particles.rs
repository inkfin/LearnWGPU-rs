use std::ops::Range;

use cgmath::Vector3;
use log::info;
use wgpu::util::DeviceExt;

use crate::{grid_2d::Grid2D, render::BindGroupLayoutCache};

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub pressure: f32,
    pub density: f32,
    pub ptype: u32, // 0: fluid, 1: boundary
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    position: [f32; 3],
    density: f32, // padding for 16 bytes
    velocity: [f32; 3],
    pressure: f32,
    ptype: u32,
    _pad: [f32; 3],
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            pressure: 0.0,
            density: 1000.0,
            ptype: 0,
        }
    }
}

impl Particle {
    fn to_raw(&self) -> ParticleRaw {
        ParticleRaw {
            position: self.position.into(),
            velocity: self.velocity.into(),
            pressure: self.pressure,
            density: self.density,
            ptype: self.ptype,
            _pad: [0.0, 0.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorldData {
    pub boundary_upper: [f32; 3],
    pub particle_radius: f32,
    pub boundary_lower: [f32; 3],
    pub support_radius: f32,
}

pub struct ParticleState {
    pub particle_radius: f32,
    pub support_radius: f32,
    pub grid_2d: Grid2D,

    // particle data shared with shaders
    pub cell_extens: Vec<Vector3<u32>>,
    pub cell_id_offsets: Vec<u32>,

    pub particle_list: Vec<Particle>,

    // wgpu state
    pub particle_render_bind_group: wgpu::BindGroup,
    pub particle_compute_bind_group_0: wgpu::BindGroup,
    pub particle_compute_bind_group_1: wgpu::BindGroup,

    // world data buffers
    pub world_buffer: wgpu::Buffer,
    pub world_bind_group: wgpu::BindGroup,
}

impl ParticleState {
    pub fn new(device: &wgpu::Device, bind_group_layout_cache: &BindGroupLayoutCache) -> Self {
        let particle_radius = 0.1;
        let support_radius = 0.4;

        let grid_2d = Grid2D::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(25, 25, 0),
            Vector3::new(support_radius, support_radius, 0.0),
        );

        let cell_extens = vec![];
        let cell_id_offsets = vec![];

        // --------------------------------------
        // Init particles

        // fluids
        let mut particle_list = vec![];

        particle_list.append(&mut get_particles_2d(
            (1.0, 2.0),
            (8.0, 5.0),
            true,
            1000.0,
            Some(Vector3::new(2.0, -2.0, 0.0)),
            particle_radius * 2.0,
        ));

        particle_list.append(&mut get_particles_2d(
            (4.0, 4.0),
            (9.0, 9.0),
            true,
            1000.0,
            Some(Vector3::new(-4.0, -2.0, 0.0)),
            particle_radius * 2.0,
        ));

        // generate walls
        let particle_diameter = particle_radius * 2.0;
        let padding = (support_radius / particle_diameter).ceil() * particle_diameter * 5.0;
        // bottom wall
        particle_list.append(&mut get_particles_2d(
            (
                grid_2d.boundary_lower.x - padding,
                grid_2d.boundary_lower.y - padding,
            ),
            (
                grid_2d.boundary_upper.x + padding + particle_diameter,
                grid_2d.boundary_lower.y,
            ),
            false,
            1000.0,
            None,
            particle_diameter,
        ));
        // left wall
        particle_list.append(&mut get_particles_2d(
            (grid_2d.boundary_lower.x - padding, grid_2d.boundary_lower.y),
            (grid_2d.boundary_lower.x, grid_2d.boundary_upper.y),
            false,
            1000.0,
            None,
            particle_diameter,
        ));
        // right wall
        particle_list.append(&mut get_particles_2d(
            (
                grid_2d.boundary_upper.x + particle_diameter,
                grid_2d.boundary_lower.y,
            ),
            (
                grid_2d.boundary_upper.x + padding + particle_diameter,
                grid_2d.boundary_upper.y,
            ),
            false,
            1000.0,
            None,
            particle_diameter,
        ));

        info!("particle list len: {}", particle_list.len());

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

        let particle_compute_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Particle Bind Group 0"),
            layout: &bind_group_layout_cache.particle_compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffers[0].as_entire_binding(),
            }],
        });

        let particle_compute_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Particle Bind Group 1"),
            layout: &bind_group_layout_cache.particle_compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffers[1].as_entire_binding(),
            }],
        });

        let particle_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Particle Bind Group"),
            layout: &bind_group_layout_cache.particle_render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffers[0].as_entire_binding(),
            }],
        });

        let world_data = WorldData {
            boundary_upper: grid_2d.boundary_upper.into(),
            boundary_lower: grid_2d.boundary_lower.into(),
            particle_radius,
            support_radius,
        };

        let world_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("World Buffer"),
            contents: bytemuck::cast_slice(&[world_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let world_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("World Bind Group"),
            layout: &bind_group_layout_cache.world_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: world_buffer.as_entire_binding(),
            }],
        });

        Self {
            particle_list,
            particle_compute_bind_group_0,
            particle_compute_bind_group_1,
            particle_render_bind_group,
            particle_radius,
            support_radius,
            grid_2d,
            cell_extens,
            cell_id_offsets,
            world_buffer,
            world_bind_group,
        }
    }
}

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
    bottom_left: (f32, f32),
    top_right: (f32, f32),
    is_fluid: bool,
    density: f32,
    velocity: Option<Vector3<f32>>,
    diameter: f32,
) -> Vec<Particle> {
    let mut particles = Vec::new();

    let num_x = ((top_right.0 - bottom_left.0) / diameter).ceil() as u32;
    let num_y = ((top_right.1 - bottom_left.1) / diameter).ceil() as u32;
    for i in 0..num_x {
        for j in 0..num_y {
            let x_coord = bottom_left.0 + i as f32 * diameter;
            let y_coord = bottom_left.1 + j as f32 * diameter;
            let p = Particle {
                position: Vector3::new(x_coord, y_coord, 0.0),
                ptype: if is_fluid { 0 } else { 1 },
                density,
                velocity: velocity.unwrap_or(Vector3::new(0.0, 0.0, 0.0)),
                ..Default::default()
            };

            particles.push(p);
        }
    }

    particles
}
