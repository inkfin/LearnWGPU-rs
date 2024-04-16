//!include foo.h.wgsl

// particles.rs Particle
struct SphParticle {
    position: vec3<f32>, // only use xy
    density: f32,
    velocity: vec3<f32>,
    support_radius: f32,
    pressure: f32,
    ptype: u32, // 0 for fluid, 1 for boundary
    _pad: array<f32, 2>,
}

@group(1) @binding(0)
var<storage, read> particles_in: array<SphParticle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;


@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, 1.0),  // 左上
        vec2<f32>(-1.0, -1.0), // 左下
        vec2<f32>(1.0, -1.0),  // 右下
        vec2<f32>(1.0, 1.0),   // 右上
        vec2<f32>(-1.0, 1.0),  // 左上
        vec2<f32>(1.0, -1.0)   // 右下
    );

    var out: VertexOutput;
    let particle_radius = particles_in[in_vertex_index / 6u].particle_radius;
    let position3f = particles_in[in_vertex_index / 6u].position;
    var position4f = camera.view_proj * vec4<f32>(position3f, 1.0);
    let offset = particle_radius * vec4<f32>(positions[in_vertex_index % 6u], 0.0, 1.0);
    position4f = position4f + offset;
    // out.clip_position = camera.view_proj * vec4<f32>(position, 0.0, 1.0);
    // let position = positions[in_vertex_index];
    out.clip_position = position4f;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.1, 0.2, 1.0, 1.0);
    // let distance = distance(in.clip_position.xy, in.pixel_position.xy);
    // if distance < in.particle_radius {
    //     return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    // } else {
    //     return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    // }
}

