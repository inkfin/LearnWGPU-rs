struct CameraUniform {
    mat_view: mat4x4<f32>,
    mat_proj: mat4x4<f32>,
    mat_view_inv: mat4x4<f32>,
    mat_proj_inv: mat4x4<f32>,
    eye: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0) var depth_in: texture_2d<f32>;

@group(2) @binding(0) var depth_out: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let resolution: vec2<f32> = vec2<f32>(textureDimensions(depth_in, 0));

    let cur_pixel_id = vec2<i32>(global_id.xy);
    if cur_pixel_id.x >= i32(resolution.x) || cur_pixel_id.y >= i32(resolution.y) {
        return;
    }

    if !written(cur_pixel_id) {
        return;
    }

    let depth: f32 = view_depth(cur_pixel_id, resolution);
    let step: vec2<i32> = vec2<i32>(1, 1);
    let offset: vec2<i32> = vec2<i32>(20, 20);

    var sum: vec2<f32> = vec2<f32>(0.0, 0.0);

    for (var x: i32 = -offset.x; x <= offset.x; x += step.x) {
        for (var y: i32 = -offset.y; y <= offset.y; y += step.y) {
            let neighbour_coords: vec2<i32> = cur_pixel_id + vec2<i32>(x, y);
            if written(neighbour_coords) {
                let neighbour_depth: f32 = view_depth(neighbour_coords, resolution);
                let r: f32 = length(vec2<f32>(f32(x) / resolution.x, f32(y) / resolution.y)) * resolution.y / 8.0;
                let w: f32 = exp(-r * r);
                let g: f32 = exp(-(neighbour_depth - depth) * (neighbour_depth - depth) * 0.05);

                sum += vec2<f32>(neighbour_depth * w * g, w * g);
            }
        }
    }

    let smoothed_depth: f32 = sum.x / sum.y;
    let v_pos: vec3<f32> = view_pos(cur_pixel_id, resolution) / depth * smoothed_depth;
    // let v_pos: vec3<f32> = view_pos(cur_pixel_id, resolution);
    let pv_pos: vec4<f32> = camera.mat_proj * vec4<f32>(v_pos, 1.0);

    textureStore(depth_out, cur_pixel_id, vec4<f32>(pv_pos.z / pv_pos.w, 0.0, 0.0, 0.0));
}

fn view_pos(coord: vec2<i32>, resolution: vec2<f32>) -> vec3<f32> {
    let zval = textureLoad(depth_in, coord, 0).x; // [0, 1]
    // wgpu spec: top-left is (0, 0), bottom-right is (vp.width, vp.height)
    let uv = vec2f(f32(coord.x) / resolution.x, f32(coord.y) / resolution.y) * 2.0 - 1.0; // [-1, 1]
    let p_pos = vec4f(uv, zval, 1.0);
    let v_pos = camera.mat_proj_inv * p_pos;
    return v_pos.xyz / v_pos.w;
}

fn view_depth(coord: vec2<i32>, resolution: vec2<f32>) -> f32 {
    // camera is always (0, 0, 0) in view space
    return length(view_pos(coord, resolution));
}

fn written(coord: vec2<i32>) -> bool {
    return textureLoad(depth_in, coord, 0).r < 0.999999;
}