/// Also bind in [BindGroupLayoutCache](render::BindGroupLayoutCache)
// pub enum BindGroupIndex {
//     Material = 0,
//     CameraUniforms = 1,
//     ParticleBuffer = 2,
// }

pub trait ShaderVertexData {
    type RawType;
    fn to_raw(&self) -> Self::RawType;

    fn desc() -> wgpu::VertexBufferLayout<'static>;
}
