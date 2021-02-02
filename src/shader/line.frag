#version 450

#define resolution vec2(500.0, 500.0)
#define Thickness 0.003

layout(set=0, binding=0)
uniform Uniforms {
    mat4 u_view_proj;
    vec3 u_view_position;
};

layout (location = 0) out vec4 outColor;

float drawLine(vec2 p1, vec2 p2) {
    vec2 uv = gl_FragCoord.xy / resolution.xy;

    float a = abs(distance(p1, uv));
    float b = abs(distance(p2, uv));
    float c = abs(distance(p1, p2));

    if ( a >= c || b >=  c ) return 0.0;

    float p = (a + b + c) * 0.5;

    // median to (p1, p2) vector
    float h = 2 / c * sqrt( p * ( p - a) * ( p - b) * ( p - c));

    return mix(1.0, 0.0, smoothstep(0.5 * Thickness, 1.5 * Thickness, h));
}

void main()
{
    vec3 start = vec3(0.1, 0.1, 0.1);
    vec3 end = vec3(0.3, 0.3, 0.3);

    //vec2 u_start = vec2(start.x / start.z, start.y / start.z);
    //vec2 u_end = vec2(end.x / end.z, end.y / end.z);

    vec4 v_start = u_view_proj * vec4(start, 1.0);
    vec4 v_end = u_view_proj * vec4(end, 1.0);

    outColor = vec4(drawLine(v_start.xy, v_end.xy));
}
