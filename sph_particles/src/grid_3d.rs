use cgmath::Vector3;

use crate::vertex_data::ShaderVertexData;

pub struct Grid3D {
    pub cell_size: Vector3<f32>,
    pub boundary_upper: Vector3<f32>,
    pub boundary_lower: Vector3<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawGrid3D {
    pub cell_size: [f32; 3],
    pub boundary_upper: [f32; 3],
    pub boundary_lower: [f32; 3],
    _pad: [f32; 3],
}

impl ShaderVertexData for Grid3D {
    type RawType = RawGrid3D;
    fn to_raw(&self) -> RawGrid3D {
        RawGrid3D {
            cell_size: self.cell_size.into(),
            boundary_upper: self.boundary_upper.into(),
            boundary_lower: self.boundary_lower.into(),
            _pad: [0.0; 3],
        }
    }

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        todo!()
    }
}

