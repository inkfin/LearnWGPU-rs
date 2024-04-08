/// Also bind in [BindGroupLayoutCache](render::BindGroupLayoutCache)
// pub enum BindGroupIndex {
//     Material = 0,
//     CameraUniforms = 1,
//     ParticleBuffer = 2,
// }

pub enum VertexDataLocation {
    Position = 0,
    Density = 1,
    Velocity = 2,
    SupportRadius = 3,
    Force = 4,
    ParticleRadius = 5,
}

pub trait ShaderVertexData {
    type RawType;
    fn to_raw(&self) -> Self::RawType;

    fn desc() -> wgpu::VertexBufferLayout<'static>;
}
