#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec2 a_tex_coords;
layout(location=2) in vec3 a_normal;
layout(location=3) in vec3 a_tangent;
layout(location=4) in vec3 a_bitangent;

layout(location=0) out vec2 v_tex_coords;
layout(location=1) out vec3 v_position;
layout(location=2) out vec3 v_light_position;
layout(location=3) out vec3 v_view_position;

layout(set=0, binding=0)
uniform Uniforms {
    mat4 u_view_proj;
    vec3 u_view_position;
};

layout(set=0, binding=1)
buffer readonly Instances {
    mat4 s_models[];
};

layout(set=2, binding=0)
uniform Light {
    vec3 light_position;
    vec3 light_color;
};


void main() {
    v_tex_coords = a_tex_coords;

    mat4 model_matrix = s_models[gl_InstanceIndex];
    // it used to be inverse for some reason, but I don't know why, check learn-wgpu tutorials
    // it doesn't work at least on mac and it's very slow to invert on gpu, do it on cpu if it's really needed
//    mat3 normal_matrix = mat3(transpose(inverse(model_matrix)));
    mat3 normal_matrix = mat3(transpose(model_matrix));

    vec3 normal = normalize(normal_matrix * a_normal);
    vec3 tangent = normalize(normal_matrix * a_tangent);
    vec3 bitangent = normalize(normal_matrix * a_bitangent);

    mat3 tangent_matrix = transpose(mat3(
        tangent,
        bitangent,
        normal
    ));

    vec4 model_space = s_models[gl_InstanceIndex] * vec4(a_position, 1.0);
    v_position = model_space.xyz;

    v_position = tangent_matrix * model_space.xyz;
    v_light_position = tangent_matrix * light_position;
    v_view_position = tangent_matrix * u_view_position;

    gl_Position = u_view_proj * model_space;
}
