use wgpu::{util::DeviceExt, SurfaceConfiguration};

use crate::texture;

use super::BindGroupLayoutCache;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeUniforms {
    pub width: u32,
    pub height: u32,
}

pub struct CopyDepthPass {
    // copy the depth texture and store it in normal texture
    pub sampled_depth_texture_0: texture::Texture,
    pub sampled_depth_texture_1: texture::Texture,

    pub sampled_depth_texture_write_bind_group_0: wgpu::BindGroup,
    pub sampled_depth_texture_write_bind_group_1: wgpu::BindGroup,

    pub sampled_depth_texture_read_bind_group_0: wgpu::BindGroup,
    pub sampled_depth_texture_read_bind_group_1: wgpu::BindGroup,

    // uniform buffer
    uniforms_data: ComputeUniforms,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,

    compute_pipeline: wgpu::ComputePipeline,
}

impl CopyDepthPass {
    pub async fn new(
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        use crate::resources::load_shader;
        let shader = device.create_shader_module(load_shader("copy_depth.wgsl").await.unwrap());

        let (
            sampled_depth_texture_0,
            sampled_depth_texture_1,
            sampled_depth_texture_write_bind_group_0,
            sampled_depth_texture_write_bind_group_1,
            sampled_depth_texture_read_bind_group_0,
            sampled_depth_texture_read_bind_group_1,
        ) = create_sampled_depth_textures(
            device,
            config,
            bind_group_layout_cache,
            "sampled_depth_texture",
        );

        let uniforms_data = ComputeUniforms {
            width: config.width,
            height: config.height,
        };

        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[uniforms_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniforms Bind Group"),
            layout: &bind_group_layout_cache.compute_uniforms_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Copy Pass Compute Pipeline Layout"),
                bind_group_layouts: &[
                    &bind_group_layout_cache.compute_uniforms_bind_group_layout,
                    &bind_group_layout_cache.particle_depth_texture_bind_group_layout,
                    &bind_group_layout_cache.sampled_depth_texture_write_bind_group_layout,
                    &bind_group_layout_cache.sampled_depth_texture_write_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        use crate::model::Vertex;
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Copy Pass Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        Self {
            sampled_depth_texture_0,
            sampled_depth_texture_1,
            sampled_depth_texture_write_bind_group_0,
            sampled_depth_texture_write_bind_group_1,
            sampled_depth_texture_read_bind_group_0,
            sampled_depth_texture_read_bind_group_1,
            uniforms_data,
            uniforms_buffer,
            uniforms_bind_group,
            compute_pipeline,
        }
    }

    pub fn compute(
        &self,
        particle_depth_texture_bind_group: &wgpu::BindGroup,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let width = self.sampled_depth_texture_0.texture.size().width;
        let height = self.sampled_depth_texture_0.texture.size().height;

        // update uniforms
        let uniforms_data = ComputeUniforms { width, height };
        queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::cast_slice(&[uniforms_data]),
        );

        let (x, y, z) = (
            (width as f32 / 16.0).ceil() as u32,
            (height as f32 / 16.0).ceil() as u32,
            1,
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder"),
        });

        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
            compute_pass.set_bind_group(1, particle_depth_texture_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.sampled_depth_texture_write_bind_group_0, &[]);
            compute_pass.set_bind_group(3, &self.sampled_depth_texture_write_bind_group_1, &[]);
            compute_pass.dispatch_workgroups(x, y, z);
        }

        // submit will accept anything that implements IntoIter
        queue.submit(vec![encoder.finish()]);
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.uniforms_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms_data]),
        );
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) {
        self.uniforms_data = ComputeUniforms {
            width: surface_config.width,
            height: surface_config.height,
        };

        (
            self.sampled_depth_texture_0,
            self.sampled_depth_texture_1,
            self.sampled_depth_texture_write_bind_group_0,
            self.sampled_depth_texture_write_bind_group_1,
            self.sampled_depth_texture_read_bind_group_0,
            self.sampled_depth_texture_read_bind_group_1,
        ) = create_sampled_depth_textures(
            device,
            surface_config,
            bind_group_layout_cache,
            "sampled_depth_texture",
        );
    }
}

fn create_sampled_depth_textures(
    device: &wgpu::Device,
    surface_config: &wgpu::SurfaceConfiguration,
    bind_group_layout_cache: &BindGroupLayoutCache,
    label: &str,
) -> (
    texture::Texture,
    texture::Texture,
    wgpu::BindGroup,
    wgpu::BindGroup,
    wgpu::BindGroup,
    wgpu::BindGroup,
) {
    let sampled_depth_texture_0 =
        texture::Texture::create_r32_texture(device, surface_config, "sampled_depth_texture_0");
    let sampled_depth_texture_1 =
        texture::Texture::create_r32_texture(device, surface_config, "sampled_depth_texture_1");
    let sampled_depth_texture_write_bind_group_0 =
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout_cache.sampled_depth_texture_write_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&sampled_depth_texture_0.view),
            }],
            label: Some("sampled_depth_texture_write_bind_group_0"),
        });
    let sampled_depth_texture_write_bind_group_1 =
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout_cache.sampled_depth_texture_write_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&sampled_depth_texture_1.view),
            }],
            label: Some("sampled_depth_texture_write_bind_group_1"),
        });
    let sampled_depth_texture_read_bind_group_0 =
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout_cache.sampled_depth_texture_read_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&sampled_depth_texture_0.view),
            }],
            label: Some("sampled_depth_texture_read_bind_group_0"),
        });
    let sampled_depth_texture_read_bind_group_1 =
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout_cache.sampled_depth_texture_read_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&sampled_depth_texture_1.view),
            }],
            label: Some("sampled_depth_texture_read_bind_group_1"),
        });

    (
        sampled_depth_texture_0,
        sampled_depth_texture_1,
        sampled_depth_texture_write_bind_group_0,
        sampled_depth_texture_write_bind_group_1,
        sampled_depth_texture_read_bind_group_0,
        sampled_depth_texture_read_bind_group_1,
    )
}
