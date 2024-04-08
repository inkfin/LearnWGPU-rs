// particles.rs Particle
struct SphParticle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    force: vec3<f32>,
    density: f32,
    support_radius: f32,
    particle_radius: f32,
}

@group(2) @binding(0)
var<storage, read> particles_in: array<SphParticle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pixel_position: vec4<f32>,
    @location(1) @interpolate(flat) particle_radius: f32,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let position = particles_in[vertex_index].position;
    out.clip_position = camera.view_proj * vec4<f32>(position, 1.0);
    out.pixel_position = out.clip_position;
    out.particle_radius = particles_in[vertex_index].particle_radius;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance = distance(in.clip_position.xyz, in.pixel_position.xyz);
    if distance < in.particle_radius {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}

