#version 450

layout(location=0) in vec3 position;
layout(location=1) in float radius;

layout(location=0) out vec4 outColor;

layout(set=0, binding=0)
uniform Resolution {
    vec2 u_resolution;
};

vec4 draw_circle(float radius, vec2 center) {
    float thikness = 1.0;
    vec2 pos = gl_FragCoord.xy / u_resolution;
    float len = length(pos.xy - center);
    if (len < radius + thikness && len > radius - thikness) {
        return vec4(1.0, 1.0, 1.0, 1.0);
    }
    return vec4(0.0, 0.0, 0.0, 0.0);
}

void main() {
    outColor = draw_circle(radius, position.xy);
}