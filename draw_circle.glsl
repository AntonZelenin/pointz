#ifdef GL_ES
precision mediump float;
#endif

uniform vec2 u_resolution;

vec4 draw_circle(float radius, vec2 center, float thikness) {
    vec2 pos = gl_FragCoord.xy / u_resolution;
    float len = length(pos.xy - center);
    if (len < radius + thikness && len > radius - thikness) {
        return vec4(1.0, 1.0, 1.0, 1.0);
    }
    return vec4(0.0, 0.0, 0.0, 0.0);
}

void main() {
    gl_FragColor = draw_circle(0.2, vec2(0.5, 0.5), 0.001);
}