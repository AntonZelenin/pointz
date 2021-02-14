#version 450

layout(location=0) in vec3 pos;

layout(set=0, binding=0)
uniform Uniforms {
    mat4 u_view_proj;
    vec3 u_view_position;
};

void main() {
    gl_Position = u_view_proj * vec4(pos, 1.0);
}
