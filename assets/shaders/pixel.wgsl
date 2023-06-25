#import rg::pixel_funcs

struct PixelMaterial {
    color: vec4<f32>,
    bands: u32,
    dither_offset: vec2<u32>,
};

@group(1) @binding(0)
var<uniform> material: PixelMaterial;

@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    let depth_samples = get_depth_samples(frag_coord.xy);
    let normal_samples = get_normal_samples(frag_coord.xy);

    let is_depth_edge = check_depth_edge(depth_samples, 0.3);
    let is_normal_edge = check_normal_edge(depth_samples, normal_samples, 0.1);
    let is_edge = is_depth_edge || is_normal_edge;

    var albedo = material.color.rgb;
    albedo = mix(albedo, albedo * 0.5, f32(is_edge));

    var pixel_input: PixelInput;
    pixel_input.frag_coord = frag_coord;
    pixel_input.mesh_position = world_position;
    pixel_input.mesh_normal = world_normal;
    pixel_input.mesh_albedo = albedo;
    pixel_input.bands = material.bands;
    pixel_input.dither = !is_edge;
    pixel_input.dither_offset = material.dither_offset;
    
    var out_color = process_all_lights(pixel_input);
    return vec4<f32>(out_color, 1.0);
}
