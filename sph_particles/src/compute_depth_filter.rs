//! This module apply bileteral filter to depth texture
use wgpu::{util::DeviceExt, BindGroupEntry, BindGroupLayoutEntry};
// TODO: FINISH this module

use crate::{compute, resources::load_shader};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsData {
    pub sigma1: f32,
    pub sigma2: f32,
    pub indexes_size: i32,
    pub filter_interval: i32,
}

pub struct DepthFilter {
    pub sigma1: f32,
    pub sigma2: f32,
    texture_depth_filter: Option<wgpu::Texture>,
    uniforms_data: Option<UniformsData>,
    uniforms_buffer: Option<wgpu::Buffer>,
    buffer_kernel_indices_5x5: Option<wgpu::Buffer>,
    buffer_kernel_indices_9x9: Option<wgpu::Buffer>,
    compute_pipeline: Option<wgpu::ComputePipeline>,
    uniforms_bind_group: Option<wgpu::BindGroup>,
    uniforms_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl DepthFilter {
    pub fn new() -> Self {
        Self {
            sigma1: 0.0,
            sigma2: 0.0,
            texture_depth_filter: None,
            buffer_kernel_indices_5x5: None,
            buffer_kernel_indices_9x9: None,
            compute_pipeline: None,
            uniforms_bind_group: None,
            uniforms_bind_group_layout: None,
            uniforms_data: None,
            uniforms_buffer: None,
        }
    }

    #[allow(dead_code)]
    pub async fn init(
        &mut self,
        sigma1: f32,
        sigma2: f32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.set_sigmas(sigma1, sigma2);
        self.init_textures(device, queue);
        self.build_shader(device).await;
        self.upload_buffers(device);
    }

    async fn build_shader(&mut self, device: &wgpu::Device) {
        let shader =
            device.create_shader_module(load_shader("compute_depth_filter.wgsl").await.unwrap());

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[self.uniforms_bind_group_layout.as_ref().unwrap()],
                push_constant_ranges: &[],
            });

        self.compute_pipeline = device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline"),
                layout: Some(&compute_pipeline_layout),
                module: &shader,
                entry_point: "main",
            })
            .into();
    }

    pub fn set_sigmas(&mut self, sigma1: f32, sigma2: f32) {
        self.sigma1 = sigma1;
        self.sigma2 = sigma2;
    }

    fn upload_buffers(&mut self, device: &wgpu::Device) {
        let indices_5x5 = self.generate_indices(2);
        self.buffer_kernel_indices_5x5 = device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Kernel Indices 5x5 Buffer"),
                contents: bytemuck::cast_slice(&indices_5x5),
                usage: wgpu::BufferUsages::STORAGE,
            })
            .into();

        let indices_9x9 = self.generate_indices(4);
        self.buffer_kernel_indices_9x9 = device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Kernel Indices 9x9 Buffer"),
                contents: bytemuck::cast_slice(&indices_9x9),
                usage: wgpu::BufferUsages::STORAGE,
            })
            .into();

        self.uniforms_bind_group_layout = device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
            .into();
    }

    fn init_textures(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let size = 128;
        let mut weights = vec![0.0; size * size];
        for i in 0..size {
            for j in 0..size {
                let d1 = (j as f32 / size as f32) * 3.0 * self.sigma1;
                let d2 = (i as f32 / size as f32) * 3.0 * self.sigma2;
                weights[i * size + j] = self.calculate_weight(d1, d2);
            }
        }

        self.texture_depth_filter = device
            .create_texture_with_data(
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
                wgpu::util::TextureDataOrder::LayerMajor,
                bytemuck::cast_slice(&weights),
            )
            .into();

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
    }

    fn calculate_weight(&self, d1: f32, d2: f32) -> f32 {
        (-d1.powi(2) / (2.0 * self.sigma1.powi(2)) - d2.powi(2) / (2.0 * self.sigma2.powi(2))).exp()
    }

    fn generate_indices(&self, half_kernel_size: usize) -> Vec<[i32; 2]> {
        let mut indices = Vec::new();
        for i in -(half_kernel_size as i32)..=half_kernel_size as i32 {
            for j in -(half_kernel_size as i32)..=half_kernel_size as i32 {
                indices.push([i, j]);
            }
        }
        indices
    }
}
