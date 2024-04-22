struct Uniforms {
    sigma1: f32,
    sigma2: f32,
    indexes_size: i32,
    filter_interval: i32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@group(1) @binding(0) var<storage, read_write> indices: array<vec2<i32>>;

@group(2) @binding(0) var tex_weight: texture_2d<f32>;

@group(3) @binding(0) var depth_in: texture_storage_2d<f32, read>;
@group(4) @binding(0) var depth_out: texture_storage_2d<f32, write>;

@compute
@workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cur_pixel_id = vec2<i32>(global_id.xy);
    let origin_depth = textureLoad(depth_in, cur_pixel_id, 0);

    if origin_depth.r > 0.0 {
        textureStore(depth_out, cur_pixel_id, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }

    var blure_depth: f32 = 0.0;
    var weight_sum: f32 = 0.0;

    for (var i = 0; i < uniforms.indexes_size; i++) {
        let index = indices[i] * uniforms.filter_interval;
        let sample_color = textureLoad(depth_in, cur_pixel_id + index, 0);
        if sample_color.r < 0.0 {
            let w = weight(length(vec2<f32>(index)), abs(origin_depth.r - sample_color.r));
            blure_depth += w * sample_color.r;
            weight_sum += w;
        }
    }

    if weight_sum != 0.0 {
        blure_depth /= weight_sum;
    }

    textureStore(depth_out, cur_pixel_id, vec4<f32>(blure_depth, 0.0, 0.0, 0.0));
}

fn weight(d1: f32, d2: f32) -> f32 {
    let tex_coord = vec2<f32>(d1 / (3.0 * uniforms.sigma1), d2 / (3.0 * uniforms.sigma2));
    return textureSample(tex_weight, sampler(tex_weight), tex_coord).r;
}

