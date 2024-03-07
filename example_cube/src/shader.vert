#version 450

struct VertexInput {
    vec3 position;
    vec2 tex_coords;
};
struct VertexOutput {
    vec4 clip_position;
    vec2 tex_coords;
};
struct InstanceInput {
    vec4 model_matrix_c0_;
    vec4 model_matrix_c1_;
    vec4 model_matrix_c2_;
    vec4 model_matrix_c3_;
};
struct CameraUniform {
    mat4x4 view_proj;
};
layout(std140, set = 1, binding = 0) uniform CameraUniform_block_0Vertex {
    CameraUniform _group_1_binding_0_vs;
};

layout(location = 0) in vec3 _p2vs_location0;
layout(location = 1) in vec2 _p2vs_location1;
layout(location = 5) in vec4 _p2vs_location5;
layout(location = 6) in vec4 _p2vs_location6;
layout(location = 7) in vec4 _p2vs_location7;
layout(location = 8) in vec4 _p2vs_location8;
layout(location = 0) smooth out vec2 _vs2fs_location0;

void main() {
    VertexInput model = VertexInput(_p2vs_location0, _p2vs_location1);
    InstanceInput instance = InstanceInput(_p2vs_location5, _p2vs_location6, _p2vs_location7, _p2vs_location8);
    VertexOutput out_ = VertexOutput(vec4(0.0), vec2(0.0));
    mat4x4 model_matrix = mat4x4(instance.model_matrix_c0_, instance.model_matrix_c1_, instance.model_matrix_c2_, instance.model_matrix_c3_);
    mat4x4 _e11 = _group_1_binding_0_vs.view_proj;
    out_.clip_position = ((_e11 * model_matrix) * vec4(model.position, 1.0));
    out_.tex_coords = model.tex_coords;
    VertexOutput _e19 = out_;
    gl_Position = _e19.clip_position;
    _vs2fs_location0 = _e19.tex_coords;
    gl_Position.yz = vec2(-gl_Position.y, gl_Position.z * 2.0 - gl_Position.w);
    return;
}
