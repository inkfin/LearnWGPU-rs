//!include particle.h.wgsl

@group(1) @binding(0)
var<storage, read> particles_in: array<SphParticle>;

const particle_radius: f32 = 0.5;

struct CameraUniform {
    mat_view: mat4x4<f32>,
    mat_proj: mat4x4<f32>,
    mat_view_inv: mat4x4<f32>,
    mat_proj_inv: mat4x4<f32>,
    eye: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

var<private> positions_offset: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, 1.0),  // top-left
    vec2<f32>(-1.0, -1.0), // bottom-left
    vec2<f32>(1.0, -1.0),  // bottom-right
    vec2<f32>(1.0, 1.0),   // top-right
    vec2<f32>(-1.0, 1.0),  // top-left
    vec2<f32>(1.0, -1.0)   // bottom-right
);

//--------------------------------------------------------------

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let p = particles_in[in_vertex_index / 6u];
    let position3f = p.position;

    let lw = normalize(camera.eye.xyz - position3f);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let x_axis = normalize(cross(up, lw));
    let y_axis = normalize(cross(lw, x_axis));

    let offset = particle_radius * vec3<f32>(positions_offset[in_vertex_index % 6u], 0.0);
    let position4f = vec4<f32>(position3f + offset.x * x_axis + offset.y * y_axis, 1.0);
    out.v_pos = (camera.mat_view * position4f).xyz;
    out.clip_position = camera.mat_proj * camera.mat_view * position4f;
    out.uv = positions_offset[in_vertex_index % 6u];

    return out;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) v_pos: vec3<f32>,
    @location(1) uv: vec2<f32>, // [-1, 1]
};

//--------------------------------------------------------------

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let coord_length = length(in.uv);
    // draw a circle
    if coord_length > 1.0 {
        discard;
    }
    let theta = acos(coord_length);
    let offset = vec3(0.0, 0.0, particle_radius * sin(theta));
    let v_pos = in.v_pos + offset;

    let pv_pos = camera.mat_proj * vec4(v_pos, 1.0);

    let depth: f32 = (pv_pos.z / pv_pos.w);

    var output: FragmentOutput;
    output.zval = depth;
    output.color = vec4f(vec3f(scale_depth(depth)), 1.0);
    return output;
}

struct FragmentOutput {
    @builtin(frag_depth) zval: f32,
    @location(0) color: vec4<f32>,
};

// better for displaying
fn scale_depth(depth: f32) -> f32 {
    return (1.0 - depth) * 100.0;
}