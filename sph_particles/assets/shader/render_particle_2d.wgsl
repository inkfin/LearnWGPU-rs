//!include world.h.wgsl

// particles.rs Particle
struct SphParticle {
    position: vec3<f32>, // only use xy
    density: f32,
    velocity: vec3<f32>,
    support_radius: f32,
    pressure: vec3<f32>,
    particle_radius: f32,
    ptype: u32,
    _pad: array<f32, 3>,
}

@group(1) @binding(0)
var<storage, read> particles_in: array<SphParticle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
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
    var positions_offset: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, 1.0),  // 左上
        vec2<f32>(-1.0, -1.0), // 左下
        vec2<f32>(1.0, -1.0),  // 右下
        vec2<f32>(1.0, 1.0),   // 右上
        vec2<f32>(-1.0, 1.0),  // 左上
        vec2<f32>(1.0, -1.0)   // 右下
    );

    var out: VertexOutput;
    let p = particles_in[in_vertex_index / 6u];
    let particle_radius = p.particle_radius;
    let position3f = p.position;

    var position4f = camera.view_proj * vec4<f32>(position3f, 1.0);
    let offset = particle_radius * vec4<f32>(positions_offset[in_vertex_index % 6u], 0.0, 1.0);
    position4f = position4f + offset;
    out.clip_position = position4f;
    out.uv = positions_offset[in_vertex_index % 6u] * 0.5;

    if p.ptype == 0u {
        // fluid
        out.color = vec4<f32>(0.1, 0.2, 1.0, 1.0);
    } else {
        out.color = vec4<f32>(0.3, 0.1, 0.2, 1.0);
    }
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // draw a circle
    if length(in.uv) < 0.4 {
        return in.color;
    } else {
        discard;
    }
    // adding this line because weird naga webgpu compile error
    return vec4f(0.0, 0.0, 0.0, 0.0);
}

