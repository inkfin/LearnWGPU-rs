use crate::{
    camera::{Camera, CameraUniform},
    particle_system::ParticleState,
    texture,
};
use wgpu::{util::DeviceExt, SurfaceConfiguration};

const RENDER_TO_STAGE: i32 = 2;

const RENDER_PARTICLE_DEPTH: i32 = 0;
const RENDER_ALPHA: i32 = 1;
const RENDER_WATER: i32 = 2;

use crate::resources::load_shader;

use super::gpu_cache::BindGroupLayoutCache;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadVertex {
    position: [f32; 2],
    coord: [f32; 2],
}

impl crate::model::Vertex for QuadVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                },
            ],
        }
    }
}

const VERTICES: &[QuadVertex] = &[
    QuadVertex {
        position: [-1.0, 1.0],
        coord: [0.0, 1.0],
    }, // top-left
    QuadVertex {
        position: [1.0, 1.0],
        coord: [1.0, 1.0],
    }, // top-right
    QuadVertex {
        position: [1.0, -1.0],
        coord: [1.0, 0.0],
    }, // bottom-right
    QuadVertex {
        position: [-1.0, -1.0],
        coord: [0.0, 0.0],
    }, // bottom-left
];

const INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct RenderUniforms {
    pub camera_intrinsic: [f32; 4],
    pub screen_coordinate: [f32; 2],
    pub znear: f32,
    pub zfar: f32,
}

/// The `BindGroupLayout` is where to define one group(index) data
/// The index is set in order of get_bind_group_layouts() vectors
pub struct RenderState {
    pub _mesh_shader: wgpu::ShaderModule,
    pub _particle_shader: wgpu::ShaderModule,
    pub _water_shader: wgpu::ShaderModule,

    // render buffers
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    /// The render pipeline defines the layout of data that the GPU will receive
    pub particle_depth_render_pipeline: wgpu::RenderPipeline,
    pub depth_to_final_render_pipeline: wgpu::RenderPipeline,

    /// depth texture for surface processing
    pub particle_depth_texture: texture::Texture,
    pub water_depth_texture: texture::Texture,
    pub particle_depth_texture_bind_group: wgpu::BindGroup,

    pub clear_color: wgpu::Color,

    // uniform buffers
    pub uniform_data: RenderUniforms,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,

    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
}

impl RenderState {
    pub async fn new(
        device: &wgpu::Device,
        camera: &Camera,
        config: &SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let uniform_data = RenderUniforms {
            camera_intrinsic: camera
                .get_camera_intrinsic(config.width, config.height)
                .into(),
            screen_coordinate: [config.width as f32, config.height as f32],
            znear: camera.znear,
            zfar: camera.zfar,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout_cache.render_uniforms_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("render_uniforms_bind_group"),
        });

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
        let water_depth_texture =
            texture::Texture::create_depth_texture(device, config, "depth_texture");

        let particle_depth_texture_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout_cache.particle_depth_texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&particle_depth_texture.view),
                    },
                    // load zbuffer directly
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&particle_depth_texture.sampler),
                    },
                ],
                label: Some("particle_depth_bind_group"),
            });

        let clear_color = wgpu::Color {
            r: 0.01,
            g: 0.01,
            b: 0.01,
            a: 1.0,
        };

        // init shader
        let mesh_shader =
            device.create_shader_module(load_shader("render_mesh.wgsl").await.unwrap());

        let particle_shader =
            // device.create_shader_module(load_shader("render_particle_2d.wgsl").await.unwrap());
            device.create_shader_module(load_shader("render_particle_3d.wgsl").await.unwrap());

        let water_shader =
            device.create_shader_module(load_shader("render_water.wgsl").await.unwrap());

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
                    module: &particle_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &particle_shader,
                    entry_point: "fs_main",
                    targets: &[if RENDER_TO_STAGE == RENDER_PARTICLE_DEPTH {
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

        // render water from depth texture
        let depth_to_final_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout Depth"),
                bind_group_layouts: &[
                    &bind_group_layout_cache.camera_bind_group_layout,
                    &bind_group_layout_cache.render_uniforms_bind_group_layout,
                    &bind_group_layout_cache.particle_depth_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        use crate::model::Vertex;
        let depth_to_final_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Depth to Final Render Pipeline Particle"),
                layout: Some(&depth_to_final_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &water_shader,
                    entry_point: "vs_main",
                    buffers: &[QuadVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &water_shader,
                    entry_point: "fs_main",
                    targets: &[if RENDER_TO_STAGE == RENDER_WATER {
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
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        Self {
            vertex_buffer,
            index_buffer,
            _mesh_shader: mesh_shader,
            _particle_shader: particle_shader,
            _water_shader: water_shader,
            particle_depth_render_pipeline,
            depth_to_final_render_pipeline,
            particle_depth_texture,
            water_depth_texture,
            clear_color,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            particle_depth_texture_bind_group,
            uniform_data,
            uniform_buffer,
            uniform_bind_group,
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

        self.uniform_data.camera_intrinsic = camera
            .get_camera_intrinsic(config.width, config.height)
            .into();
        self.uniform_data.screen_coordinate = [config.width as f32, config.height as f32];
        self.uniform_data.znear = camera.znear;
        self.uniform_data.zfar = camera.zfar;
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniform_data]),
        );
    }

    pub fn render_depth(
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
            let color_attachments = if RENDER_TO_STAGE == RENDER_PARTICLE_DEPTH {
                [Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
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

    pub fn render_water(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Water Encoder"),
        });

        {
            let color_attachments = if RENDER_TO_STAGE == RENDER_WATER {
                [Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
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
                    view: &self.water_depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.depth_to_final_render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(2, &self.particle_depth_texture_bind_group, &[]);

            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
        }

        // submit will accept anything that implements IntoIter
        queue.submit(vec![encoder.finish()]);
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
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &self.particle_depth_texture.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            &self.particle_depth_texture.sampler,
                        ),
                    },
                ],
                label: Some("sampled_depth_bind_group"),
            });
    }
}
