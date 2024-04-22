//! This module apply bileteral filter to depth texture
use crate::texture::Texture;
use wgpu::{
    util::{DeviceExt, StagingBelt},
    BindGroupEntry, BindGroupLayoutEntry,
};
// TODO: FINISH this module

use crate::resources::load_shader;

use super::BindGroupLayoutCache;

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsData {
    pub sigma1: f32,
    pub sigma2: f32,
    pub indexes_size: i32,
    pub filter_interval: i32,
    pub width: i32,
    pub height: i32,
}

pub struct ComputeDepthFilterPass {
    // depth filters as texture
    texture_weight: wgpu::Texture,
    sampler_weight: wgpu::Sampler,
    texture_weight_bind_group: wgpu::BindGroup,
    texture_weight_bind_group_layout: wgpu::BindGroupLayout,

    // uniforms
    uniforms_data: UniformsData,
    uniforms_buffer: wgpu::Buffer,

    // storage buffers
    buffer_kernel_indices_5x5: wgpu::Buffer,
    buffer_kernel_indices_9x9: wgpu::Buffer,
    buffer_kernel_indices_5x5_bind_group: wgpu::BindGroup,
    buffer_kernel_indices_9x9_bind_group: wgpu::BindGroup,

    buffer_bind_group_layout: wgpu::BindGroupLayout,

    compute_pipeline: wgpu::ComputePipeline,
}

impl ComputeDepthFilterPass {
    pub fn filter(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        depth_texture_read_bind_groups: [&wgpu::BindGroup; 2],
        depth_texture_write_bind_groups: [&wgpu::BindGroup; 2],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let (x, y, z) = (
            (config.width as f32 / 16.0).ceil() as u32,
            (config.height as f32 / 16.0).ceil() as u32,
            1,
        );

        let mut staging_belt = wgpu::util::StagingBelt::new(0x100);

        self.uniforms_data.indexes_size = 25;
        for i in 0..5 {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Depth Filter Encoder"),
            });

            // update uniforms
            self.uniforms_data.filter_interval = 2i32.pow(i);
            self.update_uniforms(&mut staging_belt, &mut encoder, device);
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Depth Filter Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &self.buffer_kernel_indices_5x5_bind_group, &[]);
                compute_pass.set_bind_group(1, &self.texture_weight_bind_group, &[]);
                compute_pass.set_bind_group(
                    2,
                    depth_texture_read_bind_groups[(i % 2) as usize],
                    &[],
                );
                compute_pass.set_bind_group(
                    3,
                    depth_texture_write_bind_groups[((i + 1) % 2) as usize],
                    &[],
                );
                compute_pass.dispatch_workgroups(x, y, z);
            }
            queue.submit(Some(encoder.finish()));

            // don't forget to recall staging belt
            staging_belt.recall();
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Depth Filter Encoder"),
        });

        self.uniforms_data.indexes_size = 81;
        self.uniforms_data.filter_interval = 1;
        self.update_uniforms(&mut staging_belt, &mut encoder, device);
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Depth Filter Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.buffer_kernel_indices_9x9_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.texture_weight_bind_group, &[]);
            compute_pass.set_bind_group(2, depth_texture_read_bind_groups[1], &[]);
            compute_pass.set_bind_group(3, depth_texture_write_bind_groups[0], &[]);
            compute_pass.dispatch_workgroups(x, y, z);
        }
        queue.submit(Some(encoder.finish()));

        // don't forget to recall staging belt
        staging_belt.recall();
    }

    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        let uniforms_data = UniformsData {
            sigma1: 8.0,
            sigma2: 0.025,
            indexes_size: 25,
            filter_interval: 1,
            width: surface_config.width as i32,
            height: surface_config.height as i32,
        };
        let (
            texture_weight,
            sampler_weight,
            texture_weight_bind_group,
            texture_weight_bind_group_layout,
        ) = init_weight_texture(uniforms_data.sigma1, uniforms_data.sigma2, device, queue);

        let (
            uniforms_data,
            uniforms_buffer,
            buffer_kernel_indices_5x5,
            buffer_kernel_indices_9x9,
            buffer_kernel_indices_5x5_bind_group,
            buffer_kernel_indices_9x9_bind_group,
            buffer_bind_group_layout,
        ) = setup_uniform_kernel_buffers(uniforms_data, device);

        let compute_pipeline = build_shader(
            vec![
                &buffer_bind_group_layout,
                &texture_weight_bind_group_layout,
                &bind_group_layout_cache.sampled_depth_texture_read_bind_group_layout,
                &bind_group_layout_cache.sampled_depth_texture_write_bind_group_layout,
            ],
            device,
        )
        .await;

        Self {
            texture_weight,
            sampler_weight,
            texture_weight_bind_group,
            texture_weight_bind_group_layout,
            uniforms_data,
            uniforms_buffer,
            buffer_kernel_indices_5x5,
            buffer_kernel_indices_9x9,
            buffer_kernel_indices_5x5_bind_group,
            buffer_kernel_indices_9x9_bind_group,
            buffer_bind_group_layout,
            compute_pipeline,
        }
    }

    fn update_uniforms(
        &self,
        staging_belt: &mut StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
    ) {
        staging_belt
            .write_buffer(
                encoder,
                &self.uniforms_buffer,
                0,
                wgpu::BufferSize::new(std::mem::size_of::<UniformsData>() as wgpu::BufferAddress)
                    .unwrap(),
                device,
            )
            .copy_from_slice(bytemuck::cast_slice(&[self.uniforms_data]));
        staging_belt.finish();
    }

    pub fn resize(&mut self, device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) {
        self.uniforms_data.width = surface_config.width as i32;
        self.uniforms_data.height = surface_config.height as i32;
    }
}

fn init_weight_texture(
    sigma1: f32,
    sigma2: f32,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (
    wgpu::Texture,
    wgpu::Sampler,
    wgpu::BindGroup,
    wgpu::BindGroupLayout,
) {
    let size = 128;
    let mut weights = vec![0.0; size * size];
    for i in 0..size {
        for j in 0..size {
            let d1 = (j as f32 / size as f32) * 3.0 * sigma1;
            let d2 = (i as f32 / size as f32) * 3.0 * sigma2;
            weights[i * size + j] = calculate_weight(sigma1, sigma2, d1, d2);
        }
    }

    let texture_weight = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("Depth Filter Texture"),
            size: wgpu::Extent3d {
                width: size as u32,
                height: size as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::default(),
        bytemuck::cast_slice(&weights),
    );

    let sampler_weight = device.create_sampler(&wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let texture_weight_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Weight Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                // wgpu::BindGroupLayoutEntry {
                //     binding: 1,
                //     visibility: wgpu::ShaderStages::COMPUTE,
                //     ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                //     count: None,
                // },
            ],
        });

    let texture_weight_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Texture Weight Bind Group"),
        layout: &texture_weight_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_weight.create_view(
                    &wgpu::TextureViewDescriptor {
                        format: wgpu::TextureFormat::R32Float.into(),
                        ..Default::default()
                    },
                )),
            },
            // wgpu::BindGroupEntry {
            //     binding: 1,
            //     resource: wgpu::BindingResource::Sampler(&sampler_weight),
            // },
        ],
    });

    (
        texture_weight,
        sampler_weight,
        texture_weight_bind_group,
        texture_weight_bind_group_layout,
    )
}

fn setup_uniform_kernel_buffers(
    uniforms_data: UniformsData,
    device: &wgpu::Device,
) -> (
    UniformsData,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::BindGroup,
    wgpu::BindGroup,
    wgpu::BindGroupLayout,
) {
    // uniform buffer
    let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniforms_data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // ssb
    let indices_5x5 = generate_indices(2);
    let buffer_kernel_indices_5x5 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Kernel Indices 5x5 Buffer"),
        contents: bytemuck::cast_slice(&indices_5x5),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let indices_9x9 = generate_indices(4);
    let buffer_kernel_indices_9x9 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Kernel Indices 9x9 Buffer"),
        contents: bytemuck::cast_slice(&indices_9x9),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Kernel Indices Bind Group Layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let buffer_kernel_indices_bind_group_5x5 =
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Kernel Indices 5x5 Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniforms_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: buffer_kernel_indices_5x5.as_entire_binding(),
                },
            ],
        });

    let buffer_kernel_indices_bind_group_9x9 =
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Kernel Indices 9x9 Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniforms_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: buffer_kernel_indices_9x9.as_entire_binding(),
                },
            ],
        });

    (
        uniforms_data,
        uniforms_buffer,
        buffer_kernel_indices_5x5,
        buffer_kernel_indices_9x9,
        buffer_kernel_indices_bind_group_5x5,
        buffer_kernel_indices_bind_group_9x9,
        bind_group_layout,
    )
}

async fn build_shader(
    bind_group_layouts: Vec<&wgpu::BindGroupLayout>,
    device: &wgpu::Device,
) -> wgpu::ComputePipeline {
    let shader =
        device.create_shader_module(load_shader("compute_depth_filter.wgsl").await.unwrap());

    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Pipeline Layout"),
        bind_group_layouts: &bind_group_layouts[..],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &shader,
        entry_point: "main",
    });

    compute_pipeline
}

fn calculate_weight(sigma1: f32, sigma2: f32, d1: f32, d2: f32) -> f32 {
    (-d1.powi(2) / (2.0 * sigma1.powi(2)) - d2.powi(2) / (2.0 * sigma2.powi(2))).exp()
}

fn generate_indices(half_kernel_size: usize) -> Vec<[i32; 2]> {
    let mut indices = Vec::new();
    for i in -(half_kernel_size as i32)..=half_kernel_size as i32 {
        for j in -(half_kernel_size as i32)..=half_kernel_size as i32 {
            indices.push([i, j]);
        }
    }
    indices
}
