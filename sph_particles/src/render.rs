use wgpu::{util::DeviceExt, SurfaceConfiguration};

use crate::{
    camera::Camera,
    model::{Model, Vertex},
    particles::DrawParticle,
    texture,
    uniforms::{CameraUniform, Instance},
    vertex_data::ShaderVertexData,
};

/// The `BindGroupLayout` is where to define one group(index) data
/// The index is set in order of get_bind_group_layouts() vectors
pub struct BindGroupLayoutCache {
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub particle_bind_group_layout: wgpu::BindGroupLayout,
}

impl BindGroupLayoutCache {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let particle_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        Self {
            texture_bind_group_layout,
            camera_bind_group_layout,
            particle_bind_group_layout,
        }
    }

    /// This function is used to get the bind group layouts in the order
    /// used in the render pipeline
    pub fn get_render_layouts_model(&self) -> Vec<&wgpu::BindGroupLayout> {
        vec![
            &self.texture_bind_group_layout,
            &self.camera_bind_group_layout,
        ]
    }

    pub fn get_render_layouts_particle(&self) -> Vec<&wgpu::BindGroupLayout> {
        vec![
            &self.texture_bind_group_layout,
            &self.camera_bind_group_layout,
            &self.particle_bind_group_layout,
        ]
    }

    pub fn get_compute_particles_layouts(&self) -> Vec<&wgpu::BindGroupLayout> {
        vec![&self.particle_bind_group_layout]
    }
}

pub struct RenderState {
    pub _mesh_shader: wgpu::ShaderModule,

    /// The render pipeline defines the layout of data that the GPU will receive
    pub render_pipeline_model: wgpu::RenderPipeline,
    pub render_pipeline_particle: wgpu::RenderPipeline,

    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,
    pub depth_texture: texture::Texture,
    pub clear_color: wgpu::Color,

    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
}

impl RenderState {
    pub fn new(
        device: &wgpu::Device,
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

        // init shader
        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mesh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shader/render_mesh.wgsl").into()),
        });

        // or use this macro:
        // let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline_layout_model =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout Model"),
                bind_group_layouts: &bind_group_layout_cache.get_render_layouts_model(),
                push_constant_ranges: &[],
            });

        let render_pipeline_layout_particle =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout Particle"),
                bind_group_layouts: &bind_group_layout_cache.get_render_layouts_particle(),
                push_constant_ranges: &[],
            });

        let render_pipeline_model =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline Model"),
                layout: Some(&render_pipeline_layout_model),
                vertex: wgpu::VertexState {
                    module: &mesh_shader,
                    entry_point: "vs_main",
                    buffers: &[crate::model::ModelVertex::desc(), crate::Instance::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
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

        let render_pipeline_particle =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline Particle"),
                layout: Some(&render_pipeline_layout_particle),
                vertex: wgpu::VertexState {
                    module: &mesh_shader,
                    entry_point: "vs_main",
                    buffers: &[crate::model::ModelVertex::desc(), crate::Instance::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
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

        let depth_texture = texture::Texture::create_depth_texture(device, config, "depth_texture");
        let clear_color = wgpu::Color {
            r: 0.3,
            g: 0.2,
            b: 0.1,
            a: 1.0,
        };

        const SPACE_BETWEEN: f32 = 3.0;
        const NUM_INSTANCES_PER_ROW: i32 = 12;
        let instances = crate::Instance::get_instances_grid(NUM_INSTANCES_PER_ROW, SPACE_BETWEEN);

        let instance_data = instances
            .iter()
            .map(crate::Instance::to_raw)
            .collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            _mesh_shader: mesh_shader,
            render_pipeline_model,
            render_pipeline_particle,
            instances,
            instance_buffer,
            depth_texture,
            clear_color,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
        }
    }

    pub fn update_uniforms(&mut self, camera: &Camera, queue: &wgpu::Queue) {
        self.camera_uniform.update_view_proj(camera);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn render_pass_particle(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        out_tex_view: &wgpu::TextureView,
        particle_state: super::particles::ParticleState,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: out_tex_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                // we don't use stencil for now
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        render_pass.set_pipeline(&self.render_pipeline_model);
    }

    pub fn render_pass_model(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        out_tex_view: &wgpu::TextureView,
        obj_model: &Model,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass Particle"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: out_tex_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                // we don't use stencil for now
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        render_pass.set_pipeline(&self.render_pipeline_model);

        use crate::model::DrawModel;
        render_pass.draw_model_instanced(
            obj_model,
            0..self.instances.len() as u32,
            &self.camera_bind_group,
        );
    }
}
