shader_type spatial;

uniform vec4 shallow_color : source_color = vec4(0.2, 0.5, 0.6, 0.8);
uniform vec4 deep_color : source_color = vec4(0.05, 0.1, 0.2, 1.0);
uniform float beer_law_factor = 0.05;
uniform float foam_threshold = 0.5;
uniform vec4 foam_color : source_color = vec4(1.0, 1.0, 1.0, 1.0);

uniform sampler2D depth_texture : hint_depth_texture, repeat_disable, filter_nearest;

void fragment() {
    float depth = texture(depth_texture, SCREEN_UV).r;
    vec3 ndc = vec3(SCREEN_UV * 2.0 - 1.0, depth * 2.0 - 1.0);
    vec4 view = INV_PROJECTION_MATRIX * vec4(ndc, 1.0);
    view.xyz /= view.w;
    float linear_depth = -view.z;
    float frag_depth = -VERTEX.z;
    float water_depth = linear_depth - frag_depth;

    vec4 water_color = mix(shallow_color, deep_color, clamp(water_depth * beer_law_factor, 0.0, 1.0));
    
    float foam = smoothstep(foam_threshold, 0.0, water_depth);
    
    ALBEDO = mix(water_color.rgb, foam_color.rgb, foam);
    ALPHA = water_color.a;
    ROUGHNESS = 0.1;
}
