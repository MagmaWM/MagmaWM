precision mediump float;
uniform sampler2D tex;
uniform vec2 size;
varying vec2 v_coords;
uniform float alpha;
uniform float radius;

float rounded_box(vec2 center, vec2 size, float radius) {
    return length(max(abs(center) - size + radius, 0.0)) - radius;
}

void main() {
    vec4 color = texture2D(tex, v_coords);
    vec2 center = size / 2.0;
    vec2 location = v_coords * size;
    vec4 mix_color;

    float distance = rounded_box(location - center, size / 2.0, radius);
    float smoothedAlpha = smoothstep(1.0, 0.0, distance);
    
    mix_color = mix(vec4(0.0, 0.0, 0.0, 0.0), color, smoothedAlpha);
    
    gl_FragColor = mix_color;
}