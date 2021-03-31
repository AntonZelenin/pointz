#version 450

layout(location=0) in vec3 a_position;

layout(location=0) out vec4 position;

layout(set=0, binding=0)
uniform Uniforms {
    mat4 u_view_proj;
    vec3 u_view_position;
};

layout(set=0, binding=1)
buffer Instances {
    mat4 transforms[];
};

void main() {
    vec4 model_space = transforms[gl_InstanceIndex] * vec4(a_position, 1.0);
    gl_Position = u_view_proj * model_space;
    position = gl_Position;
}
