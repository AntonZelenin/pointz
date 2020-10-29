#version 450

layout(location=0) out vec4 f_color;

layout(set=0, binding=0)
uniform Uniforms {
    mat4 u_view_proj;
    vec3 u_view_position;
};

void main() {
    f_color = vec4(1.0, 0.0, 0.0, 1.0);
}
