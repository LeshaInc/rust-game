#define DISABLE_SHADOWS

#import bevy_pbr::forward_io::VertexOutput
#import rg::pixel_funcs as pixel

struct WaterMaterial {
    fog_height: f32,
};

@group(1) @binding(0)
var<uniform> material: WaterMaterial;

@fragment
fn fragment(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    let prepass_depth = pixel::get_linear_depth(in.position.xy);
    let our_depth = pixel::raw_depth_to_linear(in.position.z / in.position.w);

    let foam = (1.0 - clamp((prepass_depth - our_depth) / 0.2, 0.0, 1.0)) * 0.2;
    
    let main_color = vec3(0.01298, 0.12744, 0.22323);
    let foam_color = vec3(1.0, 1.0, 1.0);
    let albedo = mix(main_color, foam_color, foam);

    var pixel_input: pixel::PixelInput;
    pixel_input.frag_coord = in.position;
    pixel_input.mesh_position = in.world_position;
    pixel_input.mesh_normal = in.world_normal;
    pixel_input.mesh_albedo = albedo;
    pixel_input.bands = 32u;
    pixel_input.dither = false;
    pixel_input.fog_height = material.fog_height;
    
    var out_color = pixel::process_all_lights(pixel_input);
    let alpha = clamp((prepass_depth - our_depth) / 1.0, 0.6, 0.9);
    return vec4<f32>(out_color * alpha, alpha);
}
