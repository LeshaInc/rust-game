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
var texture: texture_2d_array<f32>;
@group(1) @binding(2)
var texture_sampler: sampler;
@group(1) @binding(3)
var tile_map: texture_2d<u32>;

@fragment
fn fragment(
    in: MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let tile_pos = vec2<u32>((transpose(mesh.inverse_transpose_model) * in.world_position).xy * 2.0);
    let tile = textureLoad(tile_map, tile_pos, 0).r;

    let uv = fract(in.world_position.xy * 2.0);
    var albedo = textureSample(texture, texture_sampler, uv, tile).rgb;

    let wall_color = vec3(0.04231, 0.02217, 0.01298);

    var dither_matrix = mat4x4<f32>(
        vec4<f32>( 0.0 / 16.0, 12.0 / 16.0,  3.0 / 16.0, 15.0 / 16.0),
        vec4<f32>( 8.0 / 16.0,  4.0 / 16.0, 11.0 / 16.0,  7.0 / 16.0),
        vec4<f32>( 2.0 / 16.0, 14.0 / 16.0,  1.0 / 16.0, 13.0 / 16.0),
        vec4<f32>(10.0 / 16.0,  6.0 / 16.0,  9.0 / 16.0,  5.0 / 16.0)
    );

    let idx = (vec2<u32>(in.position.xy) + material.dither_offset) % 4u;
    let bayer = dither_matrix[idx.x][idx.y];

    if abs(in.world_normal.z) < 0.1 {
        albedo = wall_color;
    } else {
        let fac = 1.0 - smoothstep(0.4, 0.9, in.color.x) * 1.0;
        albedo = mix(wall_color, albedo, f32(fac > bayer));
    };

    var pixel_input: pixel::PixelInput;
    pixel_input.frag_coord = in.position;
    pixel_input.mesh_position = in.world_position;
    pixel_input.mesh_normal = in.world_normal;
    pixel_input.mesh_albedo = albedo;
    pixel_input.bands = 32u;
    pixel_input.dither = false;
    pixel_input.fog_height = material.fog_height;
    
    var out_color = pixel::process_all_lights(pixel_input);
    return vec4<f32>(out_color, 1.0);
}
