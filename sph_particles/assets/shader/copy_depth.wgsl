struct Uniforms {
    width: u32,
    height: u32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var depth_texture: texture_depth_2d;

@group(2) @binding(0)
var dst_texture_0: texture_storage_2d<r32float, write>;

@group(3) @binding(0)
var dst_texture_1: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if global_id.x >= uniforms.width || global_id.y >= uniforms.height {
        return;
    }

    let depth_value = textureLoad(depth_texture, global_id.xy, 0);

    textureStore(dst_texture_0, global_id.xy, vec4<f32>(depth_value, 0.0, 0.0, 1.0));
    textureStore(dst_texture_1, global_id.xy, vec4<f32>(depth_value, 0.0, 0.0, 1.0));
}
