#version 450

layout(location=0) in vec3 pos;

layout(location=0) out vec4 position;
layout(location=1) out float radius;

layout(set=0, binding=0)
uniform Uniforms {
    mat4 u_view_proj;
    vec3 u_view_position;
};

layout(set=0, binding=1)
buffer Radii {
    float radii[];
};

void main() {
    position = u_view_proj * vec4(pos, 1.0);
    radius = radii[gl_VertexIndex] / position.z;
}
