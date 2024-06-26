use cgmath::Vector3;
use log::info;
use wgpu::util::DeviceExt;

use super::grid::Grid;
use crate::renderer::BindGroupLayoutCache;
#[allow(unused_imports)]
use super::utils::{get_particles_3d, get_particles_2d};

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub pressure: f32,
    pub density: f32,
    pub ptype: u32, // 0: fluid, 1: boundary
    pub cell_id: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    position: [f32; 3],
    density: f32, // padding for 16 bytes
    velocity: [f32; 3],
    pressure: f32,
    ptype: u32,
    pub cell_id: u32,
    _pad: [f32; 2],
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            pressure: 0.0,
            density: 1000.0,
            ptype: 0,
            cell_id: 0,
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
            cell_id: self.cell_id,
            _pad: [0.0, 0.0],
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
    pub grid_2d: Grid,

    // particle data shared with shaders
    pub cell_extens: Vec<Vector3<u32>>,
    pub cell_id_offsets: Vec<u32>,

    // staging buffer for reading data back
    pub particle_data: Vec<ParticleRaw>,
    pub particle_buffers: [wgpu::Buffer; 2], // double buffer
    pub staging_buffer: wgpu::Buffer,

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

        let grid = Grid::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(25, 25, 25),
            Vector3::new(support_radius, support_radius, support_radius),
        );

        let cell_extens = vec![];
        let cell_id_offsets = vec![];

        // --------------------------------------
        // Init particles

        // fluids
        let mut particle_list = vec![];

        particle_list.append(&mut get_particles_3d(
            (1.0, 2.0, 1.0),
            (8.0, 5.0, 4.0),
            true,
            1000.0,
            Some(Vector3::new(2.0, -2.0, 0.0)),
            particle_radius * 2.0,
        ));

        particle_list.append(&mut get_particles_3d(
            (4.0, 4.0, 4.0),
            (9.0, 9.0, 9.0),
            true,
            1000.0,
            Some(Vector3::new(-4.0, -2.0, 0.0)),
            particle_radius * 2.0,
        ));

        // generate walls
        // let particle_diameter = particle_radius * 2.0;
        // let padding = (support_radius / particle_diameter).ceil() * particle_diameter * 5.0;
        // // bottom wall
        // particle_list.append(&mut get_particles_2d(
        //     (
        //         grid_2d.boundary_lower.x - padding,
        //         grid_2d.boundary_lower.y - padding,
        //     ),
        //     (
        //         grid_2d.boundary_upper.x + padding + particle_diameter,
        //         grid_2d.boundary_lower.y,
        //     ),
        //     false,
        //     1000.0,
        //     None,
        //     particle_diameter,
        // ));
        // // left wall
        // particle_list.append(&mut get_particles_2d(
        //     (grid_2d.boundary_lower.x - padding, grid_2d.boundary_lower.y),
        //     (grid_2d.boundary_lower.x, grid_2d.boundary_upper.y),
        //     false,
        //     1000.0,
        //     None,
        //     particle_diameter,
        // ));
        // // right wall
        // particle_list.append(&mut get_particles_2d(
        //     (
        //         grid_2d.boundary_upper.x + particle_diameter,
        //         grid_2d.boundary_lower.y,
        //     ),
        //     (
        //         grid_2d.boundary_upper.x + padding + particle_diameter,
        //         grid_2d.boundary_upper.y,
        //     ),
        //     false,
        //     1000.0,
        //     None,
        //     particle_diameter,
        // ));

        info!("particle list len: {}", particle_list.len());

        // ---------------------------------------

        let particle_data = particle_list
            .iter()
            .map(Particle::to_raw)
            .collect::<Vec<_>>();

        let particle_buffers = [
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Buffer"),
                contents: bytemuck::cast_slice(&particle_data),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            }),
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Buffer"),
                contents: bytemuck::cast_slice(&particle_data),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            }),
        ];

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Staging Buffer"),
            size: (std::mem::size_of::<ParticleRaw>() * particle_data.len()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
            boundary_upper: grid.boundary_upper.into(),
            boundary_lower: grid.boundary_lower.into(),
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
            particle_data,
            particle_buffers,
            staging_buffer,
            particle_compute_bind_group_0,
            particle_compute_bind_group_1,
            particle_render_bind_group,
            particle_radius,
            support_radius,
            grid_2d: grid,
            cell_extens,
            cell_id_offsets,
            world_buffer,
            world_bind_group,
        }
    }

    /// upload particle data to index 0
    pub fn upload_particle_data_to_gpu(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.particle_buffers[0],
            0,
            bytemuck::cast_slice(&self.particle_data),
        );
    }

    /// dump particle data from index buffer
    pub async fn dump_particle_data_from_gpu(
        &mut self,
        particle_buffer_idx: usize,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        command_encoder.copy_buffer_to_buffer(
            &self.particle_buffers[particle_buffer_idx % 2],
            0,
            &self.staging_buffer,
            0,
            (std::mem::size_of::<ParticleRaw>() * self.particle_data.len()) as wgpu::BufferAddress,
        );
        queue.submit(Some(command_encoder.finish()));
        let buffer_slice = self.staging_buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
        device.poll(wgpu::Maintain::Wait);
        receiver.receive().await.unwrap().unwrap();
        self.particle_data
            .copy_from_slice(bytemuck::cast_slice(&buffer_slice.get_mapped_range()[..]));
        self.staging_buffer.unmap();
    }
}