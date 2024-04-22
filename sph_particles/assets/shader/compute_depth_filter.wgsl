struct Uniforms {
    sigma1: f32,
    sigma2: f32,
    indexes_size: i32,
    filter_interval: i32,
    width: i32,
    height: i32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@group(0) @binding(1) var<storage, read> indices: array<vec2<i32>>;

@group(1) @binding(0) var tex_weight: texture_2d<f32>;
// @group(1) @binding(1) var sampler_weight: sampler;

@group(2) @binding(0) var depth_in: texture_2d<f32>;

@group(3) @binding(0) var depth_out: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cur_pixel_id = vec2<i32>(global_id.xy);
    if cur_pixel_id.x >= uniforms.width || cur_pixel_id.y >= uniforms.height {
        return;
    }

    let origin_depth = textureLoad(depth_in, cur_pixel_id, 0).r;

    if origin_depth > 0.9999 {
        textureStore(depth_out, cur_pixel_id, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }

    var blured_depth: f32 = 0.0;
    var weight_sum: f32 = 0.0;

    for (var i = 0; i < uniforms.indexes_size; i++) {
        let index = indices[i] * uniforms.filter_interval;
        let sample_color = textureLoad(depth_in, cur_pixel_id + index, 0).r;
        if sample_color < 0.9999 {
            let w = weight(length(vec2<f32>(index)), abs(origin_depth - sample_color));
            blured_depth += w * sample_color;
            weight_sum += w;
        }
    }

    if weight_sum != 0.0 {
        blured_depth /= weight_sum;
    }

    textureStore(depth_out, cur_pixel_id, vec4<f32>(blured_depth, 0.0, 0.0, 0.0));
}

fn weight(d1: f32, d2: f32) -> f32 {
    let tex_coord = vec2<i32>(i32(round(d1 / (3.0 * uniforms.sigma1))), i32(round(d2 / (3.0 * uniforms.sigma2))));
    return textureLoad(tex_weight, tex_coord, 0).r;
}

