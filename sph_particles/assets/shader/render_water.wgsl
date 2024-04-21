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

const light_wpos = vec3<f32>(5.0, 5.0, 20.0);
const light_ambient = vec3<f32>(1.0, 1.0, 1.0) * 0.3;
const light_diffuse = vec3<f32>(1.0, 1.0, 1.0);
const light_specular = vec3<f32>(1.0, 1.0, 1.0);
const shininess = 7.0;
const water_color = vec3<f32>(0.1, 0.2, 1.0);

//----------------------------------------------------------------------

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let pixel_id = vec2<i32>(in.clip_position.xy);
    let depth = textureLoad(depth_texture, pixel_id, 0);
    if depth == 1.0 { discard; }

    // calculate normals
    let normal = compute_normal(pixel_id);

    // render color
    var pw: vec4<f32> = camera.mat_view_inv * vec4f(view_pos(pixel_id), 1.0);
    pw = pw / pw.w;
    let lw: vec3<f32> = normalize(light_wpos - pw.xyz);
    let ew = normalize(camera.eye - pw);
    let le = (camera.mat_view * vec4f(lw, 0.0)).xyz;
    let he = normalize(le + vec3f(0.0, 0.0, 1.0));

    let diffuse = max(0.0, dot(normal, lw)) * light_diffuse;
    let specular = pow(max(0.0, dot(normal, he)), shininess) * light_specular;
    let fresnel = pow(1.0 - abs(normal.z), 5.0);

    let color = water_color * (0.3 + 0.4 * diffuse + 1.2 * specular + 5.0 * fresnel);

    var output: FragmentOutput;
    output.color = vec4f(color, 1.0);
    // output.color = vec4<f32>(normal, 1.0);
    //uncomment this to debug depth texture
    // output.color = vec4<f32>(vec3f(scale_depth(depth)), 1.0);
    return output;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

//----------------------------------------------------------------------

fn z_to_depth(z: f32) -> f32 {
    return lerp(get_znear(), get_zfar(), z);
}

fn view_pos(coord: vec2<i32>) -> vec3<f32> {
    let zval = textureLoad(depth_texture, coord, 0); // [0, 1]
    // wgpu spec: top-left is (0, 0), bottom-right is (vp.width, vp.height)
    let uv = vec2f(1.0-f32(coord.x) / get_width(), f32(coord.y) / get_height()) * 2.0 - 1.0; // [-1, 1]
    let p_pos = vec4f(uv, zval, 1.0);
    let v_pos = camera.mat_proj_inv * p_pos;
    return v_pos.xyz / v_pos.w;
}

fn compute_normal(tex_coords: vec2<i32>) -> vec3<f32> {
    let pos = view_pos(tex_coords);
    
    var ddx = view_pos(tex_coords + vec2<i32>(1, 0)) - pos;
    let ddx2 = pos - view_pos(tex_coords + vec2<i32>(-1, 0));
    if abs(ddx.z) > abs(ddx2.z) { ddx = ddx2; }

    var ddy = view_pos(tex_coords + vec2<i32>(0, 1)) - pos;
    let ddy2 = pos - view_pos(tex_coords + vec2<i32>(0, -1));
    if abs(ddy.z) > abs(ddy2.z) { ddy = ddy2; }

    let normal: vec3<f32> = normalize(cross(ddx.xyz, ddy.xyz));

    return normal;
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + (b - a) * t;
}

// better for displaying
fn scale_depth(depth: f32) -> f32 {
    return (1.0 - depth) * 50.0;
}