#define DITHER_ENABLED

#import bevy_pbr::mesh_vertex_output MeshVertexOutput
#import bevy_pbr::mesh_bindings mesh
#import rg::pixel_funcs as pixel

struct TerrainMaterial {
    dither_offset: vec2<u32>,
    fog_height: f32,
};

@group(1) @binding(0)
var<uniform> material: TerrainMaterial;
@group(1) @binding(1)
var texture: texture_2d<f32>;
@group(1) @binding(2)
var texture_sampler: sampler;
@group(1) @binding(3)
var tile_map: texture_2d<u32>;

@fragment
fn fragment(
    in: MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let depth_samples = pixel::get_depth_samples(in.position.xy);
    let normal_samples = pixel::get_normal_samples(in.position.xy);

    let is_depth_edge = pixel::check_depth_edge(depth_samples, 0.3);
    let is_normal_edge = pixel::check_normal_edge(depth_samples, normal_samples, 0.1);
    let is_edge = is_depth_edge || is_normal_edge;

    let tile_pos = vec2<u32>((transpose(mesh.inverse_transpose_model) * in.world_position).xy * 2.0);
    let tile = textureLoad(tile_map, tile_pos, 0).r;

    var uv = (in.world_position.xy * 2.0) % 1.0;
    uv.y *= 0.5;
    if tile == 1u {
        uv.y += 0.5;
    }

    var albedo = textureSample(texture, texture_sampler, uv).rgb;

    if abs(in.world_normal.z) < 0.1 {
        albedo = vec3(0.12, 0.01, 0.01);
    }
    
    albedo = mix(albedo, albedo * 0.5, f32(is_edge));
    albedo *= 1.0 - smoothstep(0.5, 1.0, in.color.x) * 0.5;

    var pixel_input: pixel::PixelInput;
    pixel_input.frag_coord = in.position;
    pixel_input.mesh_position = in.world_position;
    pixel_input.mesh_normal = in.world_normal;
    pixel_input.mesh_albedo = albedo;
    pixel_input.bands = 8u;
    pixel_input.dither = true;
    pixel_input.dither_offset = material.dither_offset;
    pixel_input.fog_height = material.fog_height;
    
    var out_color = pixel::process_all_lights(pixel_input);
    return vec4<f32>(out_color, 1.0);
}
