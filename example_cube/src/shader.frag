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
layout(set = 0, binding = 0) uniform sampler2D _group_0_binding_0_fs;

layout(location = 0) smooth in vec2 _vs2fs_location0;
layout(location = 0) out vec4 _fs2p_location0;

void main() {
    VertexOutput in_ = VertexOutput(gl_FragCoord, _vs2fs_location0);
    vec4 _e4 = texture(_group_0_binding_0_fs, vec2(in_.tex_coords));
    _fs2p_location0 = _e4;
    return;
}
