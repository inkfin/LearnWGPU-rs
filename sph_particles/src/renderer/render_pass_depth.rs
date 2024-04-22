use crate::{
    camera::{Camera, CameraUniform},
    particle_system::ParticleState,
    texture,
};
use wgpu::{util::DeviceExt, SurfaceConfiguration};

use crate::resources::load_shader;

use super::bind_group_layout_cache::BindGroupLayoutCache;

/// The `BindGroupLayout` is where to define one group(index) data
/// The index is set in order of get_bind_group_layouts() vectors
pub struct RenderDepthPass {
    pub _particle_depth_shader: wgpu::ShaderModule,
    pub _particle_thickness_shader: wgpu::ShaderModule,

    /// The render pipeline defines the layout of data that the GPU will receive
    pub particle_depth_render_pipeline: wgpu::RenderPipeline,
    pub particle_thickness_render_pipeline: wgpu::RenderPipeline,

    /// depth texture for surface processing
    pub particle_depth_texture: texture::Texture,
    pub particle_depth_texture_bind_group: wgpu::BindGroup,

    // thickness render pipeline setup
    pub particle_thickness_texture: texture::Texture,
    pub particle_thickness_texture_bind_group: wgpu::BindGroup,

    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
}

impl RenderDepthPass {
    pub async fn new(
        device: &wgpu::Device,
        camera: &Camera,
        config: &SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        let camera_uniform = CameraUniform::new();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout_cache.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // depth bind group
        let particle_depth_texture =
            texture::Texture::create_depth_texture(device, config, "depth_texture");

        let particle_depth_texture_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout_cache.particle_depth_texture_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&particle_depth_texture.view),
                }],
                label: Some("particle_depth_bind_group"),
            });

        // thickness setup
        let particle_thickness_texture =
            texture::Texture::create_rgba_texture(device, config, "thickness_texture");

        let particle_thickness_texture_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout_cache.particle_thickness_texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &particle_thickness_texture.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            &particle_thickness_texture.sampler,
                        ),
                    },
                ],
                label: Some("particle_thickness_bind_group"),
            });

        // init shader
        let particle_depth_shader = device
            .create_shader_module(load_shader("render_particle_depth_3d.wgsl").await.unwrap());

        let particle_thickness_shader = device.create_shader_module(
            load_shader("render_particle_thickness_3d.wgsl")
                .await
                .unwrap(),
        );

        // pipeline setups
        let particle_depth_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout Depth"),
                bind_group_layouts: &[
                    &bind_group_layout_cache.camera_bind_group_layout,
                    &bind_group_layout_cache.particle_render_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let particle_depth_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Depth Render Pipeline Particle"),
                layout: Some(&particle_depth_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &particle_depth_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &particle_depth_shader,
                    entry_point: "fs_main",
                    targets: &[if super::RENDER_TARGET == super::RENDER_PARTICLE_DEPTH {
                        Some(wgpu::ColorTargetState {
                            format: config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })
                    } else {
                        None
                    }],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0u64,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        // render particle thickness based on color
        let blend_state = wgpu::BlendState {
            color: wgpu::BlendComponent {
                operation: wgpu::BlendOperation::Add,
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
            },
            alpha: wgpu::BlendComponent {
                operation: wgpu::BlendOperation::Add,
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
            },
        };
        let particle_thickness_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout Thickness"),
                bind_group_layouts: &[
                    &bind_group_layout_cache.camera_bind_group_layout,
                    &bind_group_layout_cache.particle_render_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let particle_thickness_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Thickness Render Pipeline Particle"),
                layout: Some(&particle_thickness_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &particle_thickness_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &particle_thickness_shader,
                    entry_point: "fs_main",
                    targets: &[if super::RENDER_TARGET == super::RENDER_ALPHA {
                        Some(wgpu::ColorTargetState {
                            format: config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })
                    } else {
                        None
                    }],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0u64,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        Self {
            _particle_depth_shader: particle_depth_shader,
            _particle_thickness_shader: particle_thickness_shader,
            particle_depth_render_pipeline,
            particle_thickness_render_pipeline,
            particle_depth_texture,
            particle_depth_texture_bind_group,
            particle_thickness_texture,
            particle_thickness_texture_bind_group,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
        }
    }

    pub fn update_uniforms(
        &mut self,
        camera: &Camera,
        config: &SurfaceConfiguration,
        queue: &wgpu::Queue,
    ) {
        self.camera_uniform.update_view_proj(camera);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn render(
        &self,
        particle_state: &ParticleState,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Depth Encoder"),
        });

        {
            let color_attachments = if super::RENDER_TARGET == super::RENDER_PARTICLE_DEPTH {
                [Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(super::CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })]
            } else {
                [None]
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.particle_depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.particle_depth_render_pipeline);

            use crate::particle_system::DrawParticle;
            render_pass.draw_particle_instanced(
                0..1,
                &self.camera_bind_group,
                particle_state.particle_data.len() as u32,
                &particle_state.particle_render_bind_group,
            );
        }

        // submit will accept anything that implements IntoIter
        queue.submit(Some(encoder.finish()));
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        surface_config: &SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) {
        self.particle_depth_texture = texture::Texture::create_depth_texture(
            device,
            surface_config,
            "particle_depth_texture",
        );
        self.particle_depth_texture_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout_cache.particle_depth_texture_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.particle_depth_texture.view),
                }],
                label: Some("particle_depth_bind_group"),
            });
    }
}
