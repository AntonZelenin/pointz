#version 450

layout(location=0) in vec3 pos;

layout(location=0) out vec4 position;

layout(set=0, binding=0)
uniform Uniforms {
    mat4 u_view_proj;
    vec3 u_view_position;
};

void main() {
    position = u_view_proj * vec4(pos, 1.0);
    gl_Position = position;
}
