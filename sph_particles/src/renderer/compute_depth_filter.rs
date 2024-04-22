//! This module apply bileteral filter to depth texture
use crate::texture::Texture;
use wgpu::{
    util::{DeviceExt, StagingBelt},
    BindGroupEntry, BindGroupLayoutEntry,
};
// TODO: FINISH this module

use crate::resources::load_shader;

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsData {
    pub sigma1: f32,
    pub sigma2: f32,
    pub indexes_size: i32,
    pub filter_interval: i32,
}

pub struct DepthFilter {
    // depth filters as texture
    texture_depth_filter: wgpu::Texture,
    sampler_depth_filter: wgpu::Sampler,
    depth_filter_bind_group: wgpu::BindGroup,
    depth_filter_bind_group_layout: wgpu::BindGroupLayout,

    // uniforms
    uniforms_data: UniformsData,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
    uniforms_bind_group_layout: wgpu::BindGroupLayout,

    // storage buffers
    buffer_kernel_indices_5x5: wgpu::Buffer,
    buffer_kernel_indices_9x9: wgpu::Buffer,
    buffer_kernel_indices_bind_group: wgpu::BindGroup,
    buffer_kernel_indices_bind_group_layout: wgpu::BindGroupLayout,

    // depth texture bind group layout
    in_texture_bind_group_layout: wgpu::BindGroupLayout,
    out_texture_bind_group_layout: wgpu::BindGroupLayout,

    compute_pipeline: wgpu::ComputePipeline,
}

impl DepthFilter {
    pub fn filter(
        &mut self,
        depth_texture_in: &Texture,
        depth_texture_out: &Texture,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        assert!(
            depth_texture_in.texture.size() == depth_texture_out.texture.size(),
            "Input and output textures must have the same size"
        );

        let in_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Depth Filter In Texture Bind Group"),
            layout: &self.in_texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&depth_texture_in.view),
            }],
        });

        let out_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Depth Filter Out Texture Bind Group"),
            layout: &self.out_texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&depth_texture_out.view),
            }],
        });

        let mut staging_belt = wgpu::util::StagingBelt::new(0x100);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Depth Filter Encoder"),
        });

        self.uniforms_data.indexes_size = 25;
        for i in 0..5 {
            // update uniforms
            self.uniforms_data.filter_interval = 2f32.powi(i) as i32;

            self.update_uniforms(&mut staging_belt, &mut encoder, device);

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Depth Filter Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.buffer_kernel_indices_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.depth_filter_bind_group, &[]);
            compute_pass.set_bind_group(3, &in_texture_bind_group, &[]);
            compute_pass.set_bind_group(4, &out_texture_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                depth_texture_in.texture.size().width / 16,
                depth_texture_in.texture.size().height / 16,
                1,
            );
        }

        queue.submit(std::iter::once(encoder.finish()));

        // don't forget to recall staging belt
        staging_belt.recall();
    }

    pub async fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let uniforms_data = UniformsData {
            sigma1: 8.0,
            sigma2: 0.025,
            indexes_size: 25,
            filter_interval: 1,
        };
        let (
            texture_depth_filter,
            sampler_depth_filter,
            depth_filter_bind_group,
            depth_filter_bind_group_layout,
        ) = init_weight_texture(uniforms_data.sigma1, uniforms_data.sigma2, device, queue);

        let (uniforms_data, uniforms_buffer, uniforms_bind_group, uniforms_bind_group_layout) =
            setup_uniform_buffer(uniforms_data, device);

        let (
            buffer_kernel_indices_5x5,
            buffer_kernel_indices_9x9,
            buffer_kernel_indices_bind_group,
            buffer_kernel_indices_bind_group_layout,
        ) = setup_kernel_buffers(device);

        let (in_texture_bind_group_layout, out_texture_bind_group_layout) =
            init_texture_bind_groups(device);

        let compute_pipeline = build_shader(
            vec![
                &uniforms_bind_group_layout,
                &buffer_kernel_indices_bind_group_layout,
                &depth_filter_bind_group_layout,
                &in_texture_bind_group_layout,
                &out_texture_bind_group_layout,
            ],
            device,
        )
        .await;

        Self {
            texture_depth_filter,
            sampler_depth_filter,
            depth_filter_bind_group,
            depth_filter_bind_group_layout,
            uniforms_data,
            uniforms_buffer,
            uniforms_bind_group,
            uniforms_bind_group_layout,
            buffer_kernel_indices_5x5,
            buffer_kernel_indices_9x9,
            buffer_kernel_indices_bind_group,
            buffer_kernel_indices_bind_group_layout,
            in_texture_bind_group_layout,
            out_texture_bind_group_layout,
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

    let texture_depth_filter = device.create_texture_with_data(
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

    let sampler_depth_filter = device.create_sampler(&wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let depth_filter_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Depth Filter Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

    let depth_filter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Depth Filter Bind Group"),
        layout: &depth_filter_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture_depth_filter.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler_depth_filter),
            },
        ],
    });

    (
        texture_depth_filter,
        sampler_depth_filter,
        depth_filter_bind_group,
        depth_filter_bind_group_layout,
    )
}

fn setup_uniform_buffer(
    uniforms_data: UniformsData,
    device: &wgpu::Device,
) -> (
    UniformsData,
    wgpu::Buffer,
    wgpu::BindGroup,
    wgpu::BindGroupLayout,
) {
    // uniform buffer
    let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniforms_data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let uniforms_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &uniforms_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: uniforms_buffer.as_entire_binding(),
        }],
    });

    (
        uniforms_data,
        uniforms_buffer,
        uniforms_bind_group,
        uniforms_bind_group_layout,
    )
}

fn setup_kernel_buffers(
    device: &wgpu::Device,
) -> (
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::BindGroup,
    wgpu::BindGroupLayout,
) {
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

    let buffer_kernel_indices_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Kernel Indices Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
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

    let buffer_kernel_indices_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Kernel Indices Bind Group"),
        layout: &buffer_kernel_indices_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: buffer_kernel_indices_5x5.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: buffer_kernel_indices_9x9.as_entire_binding(),
            },
        ],
    });

    (
        buffer_kernel_indices_5x5,
        buffer_kernel_indices_9x9,
        buffer_kernel_indices_bind_group,
        buffer_kernel_indices_bind_group_layout,
    )
}

fn init_texture_bind_groups(
    device: &wgpu::Device,
) -> (wgpu::BindGroupLayout, wgpu::BindGroupLayout) {
    let in_texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Depth Filter In Texture Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            }],
        });

    let out_texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Depth Filter Out Texture Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            }],
        });

    (in_texture_bind_group_layout, out_texture_bind_group_layout)
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
