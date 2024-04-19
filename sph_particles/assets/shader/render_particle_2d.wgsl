//!include particle.h.wgsl

@group(1) @binding(0)
var<storage, read> particles_in: array<SphParticle>;

const particle_radius: f32 = 0.05;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;


@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var positions_offset: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, 1.0),  // top-left
        vec2<f32>(-1.0, -1.0), // bottom-left
        vec2<f32>(1.0, -1.0),  // bottom-right
        vec2<f32>(1.0, 1.0),   // top-right
        vec2<f32>(-1.0, 1.0),  // top-left
        vec2<f32>(1.0, -1.0)   // bottom-right
    );

    var out: VertexOutput;
    let p = particles_in[in_vertex_index / 6u];
    let position3f = p.position;

    let lw = normalize(camera.eye.xyz - position3f);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let x_axis = normalize(cross(up, lw));
    let y_axis = normalize(cross(lw, x_axis));

    let offset = particle_radius * vec3<f32>(positions_offset[in_vertex_index % 6u], 0.0);
    let position4f = camera.view_proj * vec4<f32>(position3f + offset.x * x_axis + offset.y * y_axis, 1.0);
    out.clip_position = position4f;
    out.uv = positions_offset[in_vertex_index % 6u];

    if p.ptype == 0u {
        // fluid
        out.color = vec4<f32>(0.1, 0.2, 1.0, 1.0);
        // out.color = vec4<f32>(p.mass, 0.0, 0.0, 1.0);
    } else {
        out.color = vec4<f32>(0.3, 0.1, 0.2, 1.0);
    }
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // draw a circle
    if length(in.uv) < 1.0 {
        return in.color;
    } else {
        discard;
    }
    // adding this line because weird naga webgpu compile error
    return vec4f(0.0, 0.0, 0.0, 0.0);
}

