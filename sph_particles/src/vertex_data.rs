/// Also bind in [BindGroupLayoutCache](render::BindGroupLayoutCache)
pub enum BindGroupIndex {
    Material = 0,
    CameraUniforms = 1,
    ParticleBuffer = 2,
}

pub enum VertexDataLocation {
    Position = 0,
    Velocity = 1,
    Force = 2,
    Density = 3,
    SupportRadius = 4,
    ParticleRadius = 5,
}

pub trait ShaderVertexData {
    type RawType;
    fn to_raw(&self) -> Self::RawType;

    fn desc() -> wgpu::VertexBufferLayout<'static>;
}
