precision mediump float;
uniform vec2 size;
varying vec2 v_coords;

uniform vec2 gradientDirection;
uniform vec3 startColor;
uniform vec3 endColor;
uniform float thickness;
uniform float halfThickness;

void main() {
    vec2 center = size / 2.0 - vec2(0.5);
    vec2 location = v_coords * size;
    vec4 mix_color;

    float distance = max(abs(location.x - center.x) - (size.x / 2.0 - halfThickness), abs(location.y - center.y) - (size.y / 2.0 - halfThickness));
    float smoothedAlpha = 1.0 - smoothstep(0.0, 1.0, abs(distance) - (halfThickness));

    float dotProduct = dot(v_coords, gradientDirection);

    vec3 gradientColor = mix(startColor, endColor, smoothstep(0.0, 1.0, dotProduct));

    mix_color = mix(vec4(0.0, 0.0, 0.0, 0.0), vec4(gradientColor, smoothedAlpha), smoothedAlpha);

    gl_FragColor = mix_color;
}
