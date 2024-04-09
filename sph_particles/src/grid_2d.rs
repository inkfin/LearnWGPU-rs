// TODO: Use grid to accelerate neighbor search later
use cgmath::Vector2;

use crate::vertex_data::ShaderVertexData;

const MAX_NUM_PARTICLES_PER_CELL: u32 = 500;
const MAX_NUM_NEIGHBORS: u32 = 500;

pub struct Grid2D {
    pub cell_size: Vector2<f32>,
    pub boundary_upper: Vector2<f32>,
    pub boundary_lower: Vector2<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Grid2DRaw {
    pub cell_size: [f32; 2],
    pub boundary_upper: [f32; 2],
    pub boundary_lower: [f32; 2],
    _pad: [f32; 2],
}

impl ShaderVertexData for Grid2D {
    type RawType = Grid2DRaw;
    fn to_raw(&self) -> Grid2DRaw {
        Grid2DRaw {
            cell_size: self.cell_size.into(),
            boundary_upper: self.boundary_upper.into(),
            boundary_lower: self.boundary_lower.into(),
            _pad: [0.0; 2],
        }
    }

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Grid2DRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // cell_size
                wgpu::VertexAttribute {
                    offset: 0,
                    format: wgpu::VertexFormat::Float32x2,
                    shader_location: 0,
                },
                // boundary_upper
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    format: wgpu::VertexFormat::Float32x2,
                    shader_location: 1,
                },
                // boundary_lower
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    format: wgpu::VertexFormat::Float32x2,
                    shader_location: 2,
                },
            ],
        }
    }
}
