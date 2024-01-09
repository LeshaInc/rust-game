#import bevy_pbr::forward_io::VertexOutput
#import rg::pixel_funcs as pixel

struct PixelMaterial {
    color: vec4<f32>,
    bands: u32,
    dither_offset: vec2<u32>,
    fog_height: f32,
};

@group(1) @binding(0)
var<uniform> material: PixelMaterial;

@fragment
fn fragment(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    let depth_samples = pixel::get_depth_samples(in.position.xy);
    let normal_samples = pixel::get_normal_samples(in.position.xy);

    let is_depth_edge = pixel::check_depth_edge(depth_samples, 0.3);
    let is_normal_edge = pixel::check_normal_edge(depth_samples, normal_samples, 0.1);
    let is_edge = is_depth_edge || is_normal_edge;

    var albedo = material.color.rgb;
#ifdef VERTEX_COLORS
    albedo *= in.color.rgb;
#endif
    albedo = mix(albedo, albedo * 0.5, f32(is_edge));

    var pixel_input: pixel::PixelInput;
    pixel_input.frag_coord = in.position;
    pixel_input.mesh_position = in.world_position;
    pixel_input.mesh_normal = in.world_normal;
    pixel_input.mesh_albedo = albedo;
    pixel_input.bands = material.bands;
    pixel_input.dither = !is_edge;
    pixel_input.dither_offset = material.dither_offset;
    pixel_input.fog_height = material.fog_height;
    
    var out_color = pixel::process_all_lights(pixel_input);
    return vec4<f32>(out_color, 1.0);
}
