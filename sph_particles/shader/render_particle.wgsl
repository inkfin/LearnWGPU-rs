struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct ModelMatrix {
    @location(5) model_matrix_c0: vec4<f32>,
    @location(6) model_matrix_c1: vec4<f32>,
    @location(7) model_matrix_c2: vec4<f32>,
    @location(8) model_matrix_c3: vec4<f32>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    model: VertexInput,
    instance: ModelMatrix,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_c0,
        instance.model_matrix_c1,
        instance.model_matrix_c2,
        instance.model_matrix_c3,
    );

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}

