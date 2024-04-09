use cgmath::Vector3;

use crate::vertex_data::ShaderVertexData;

pub struct Grid3D {
    pub cell_size: Vector3<f32>,
    pub boundary_upper: Vector3<f32>,
    pub boundary_lower: Vector3<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Grid3DRaw {
    pub cell_size: [f32; 3],
    pub boundary_upper: [f32; 3],
    pub boundary_lower: [f32; 3],
    _pad: [f32; 3],
}

impl ShaderVertexData for Grid3D {
    type RawType = Grid3DRaw;
    fn to_raw(&self) -> Grid3DRaw {
        Grid3DRaw {
            cell_size: self.cell_size.into(),
            boundary_upper: self.boundary_upper.into(),
            boundary_lower: self.boundary_lower.into(),
            _pad: [0.0; 3],
        }
    }

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Grid3DRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // cell_size
                wgpu::VertexAttribute {
                    offset: 0,
                    format: wgpu::VertexFormat::Float32x3,
                    shader_location: 0,
                },
                // boundary_upper
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    format: wgpu::VertexFormat::Float32x3,
                    shader_location: 1,
                },
                // boundary_lower
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    format: wgpu::VertexFormat::Float32x3,
                    shader_location: 2,
                },
            ],
        }
    }
}
