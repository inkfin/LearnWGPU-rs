//!include foo.h.wgsl

struct CameraUniform {
    mat_view: mat4x4<f32>,
    mat_proj: mat4x4<f32>,
    mat_view_inv: mat4x4<f32>,
    mat_proj_inv: mat4x4<f32>,
    eye: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct Uniforms {
    camera_intrinsic: vec4<f32>,
    screen_coordinate_znear_zfar: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

fn get_height() -> f32 {
    return uniforms.screen_coordinate_znear_zfar.x;
}
fn get_width() -> f32 {
    return uniforms.screen_coordinate_znear_zfar.y;
}
fn get_znear() -> f32 {
    return uniforms.screen_coordinate_znear_zfar.z;
}
fn get_zfar() -> f32 {
    return uniforms.screen_coordinate_znear_zfar.w;
}


@group(2) @binding(0)
var depth_texture: texture_depth_2d;
@group(2) @binding(1)
var depth_sampler: sampler;


//----------------------------------------------------------------------

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) coord: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let clip_position = vec4<f32>(in.pos, 0.0, 1.0);
    var output: VertexOutput;
    output.clip_position = clip_position;
    output.tex_coords = in.coord;
    return output;
}

//----------------------------------------------------------------------

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

//----------------------------------------------------------------------

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let pixel_id = vec2<i32>(
        in.clip_position.xy //* vec2<f32>(get_width(), get_height())
    );
    // let depth = textureSample(depth_texture, depth_sampler, in.clip_position.xy);
    let depth = (1.0 - textureLoad(depth_texture, pixel_id, 0)) * 100;
    // calculate normals
    // let normal = compute_normal(pixel_id);

    // render color

    var output: FragmentOutput;
    // output.color = vec4<f32>(normal, 1.0);
    // output.color = mix(
    //     vec4<f32>(0.1, 0.2, 1.0, 1.0),
    //     vec4<f32>(0.0, 0.0, 0.0, 1.0),
    //     depth,
    // );
    output.color = vec4<f32>(vec3f(depth), 1.0);
    return output;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

//----------------------------------------------------------------------

fn z_to_depth(z: f32) -> f32 {
    return lerp(get_znear(), get_zfar(), z);
}

// fn view_pos(coord: vec2<i32>) -> vec3<f32> {
//     let depth = textureLoad(depth_texture, coord, 0);
//     let p_pos = vec4<f32>(vec3<f32>(f32(coord.x), f32(coord.y), depth) * 2.0 - 1.0, 1.0);
//     let v_pos = camera.mat_proj_inv * p_pos;
//     return v_pos.xyz / v_pos.w;
// }

// fn compute_normal(tex_coords: vec2<i32>) -> vec3<f32> {
//     var ddx = view_pos(tex_coords + vec2<i32>(1, 0));
//     let ddx2 = view_pos(tex_coords + vec2<i32>(-1, 0));
//     if abs(ddx.z) > abs(ddx2.z) { ddx = ddx2; }

//     var ddy = view_pos(tex_coords + vec2<i32>(0, 1));
//     let ddy2 = view_pos(tex_coords + vec2<i32>(0, -1));
//     if abs(ddy.z) > abs(ddy2.z) { ddy = ddy2; }

//     let normal: vec3<f32> = normalize(cross(ddx.xyz, ddy.xyz));

//     return normal; // change Z scalar to get best effect
// }

// fn compute_normal(depth: f32, coord: vec2<i32>) -> vec3<f32> {
//     let p0 = reproject(depth, coord);
//     let p1 = reproject(depth, coord + vec2<i32>(1, 0));
//     let p2 = reproject(depth, coord + vec2<i32>(0, 1));
//     let v0 = p1 - p0;
//     let v1 = p2 - p0;
//     return normalize(cross(v0, v1));
// }

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + (b - a) * t;
}
