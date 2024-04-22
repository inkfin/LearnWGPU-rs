use wgpu::{util::DeviceExt, SurfaceConfiguration};

use crate::{
    camera::{Camera, CameraUniform},
    texture,
};

use super::BindGroupLayoutCache;

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

pub struct RenderQuadPass {
    pub _shader: wgpu::ShaderModule,

    // render buffers
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    // uniform buffers
    pub uniform_data: RenderUniforms,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,

    // camera uniforms
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,

    pub render_pipeline: wgpu::RenderPipeline,
}

impl RenderQuadPass {
    pub async fn new(
        device: &wgpu::Device,
        camera: &Camera,
        config: &SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) -> Self {
        use crate::resources::load_shader;
        let shader = device.create_shader_module(load_shader("render_water.wgsl").await.unwrap());

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

        // render uniforms
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

        // camera uniforms
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

        // render water from depth texture
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Water Render Pipeline Layout"),
                bind_group_layouts: &[
                    &bind_group_layout_cache.camera_bind_group_layout,
                    &bind_group_layout_cache.render_uniforms_bind_group_layout,
                    &bind_group_layout_cache.particle_depth_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        use crate::model::Vertex;
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Water Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[QuadVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[if super::RENDER_TARGET == super::RENDER_WATER {
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            _shader: shader,
            vertex_buffer,
            index_buffer,
            uniform_data,
            uniform_buffer,
            uniform_bind_group,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            render_pipeline,
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

    pub fn render(
        &self,
        particle_depth_texture_bind_group: &wgpu::BindGroup,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let color_attachments = if super::RENDER_TARGET == super::RENDER_WATER {
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
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(2, &particle_depth_texture_bind_group, &[]);

            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
        }

        // submit will accept anything that implements IntoIter
        queue.submit(vec![encoder.finish()]);
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        bind_group_layout_cache: &BindGroupLayoutCache,
    ) {
    }
}
