#import bevy_pbr::mesh_view_bindings as bindings
#import rg::pixel_funcs as pixel

struct Vertex {
    @location(0) uv: vec2<f32>,

    @location(1) i_pos: vec3<f32>,
    @location(2) i_size: vec2<f32>,
    @location(3) i_color: vec3<f32>,
    @location(4) i_random: u32,
};

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

struct Uniforms {
    transform: mat4x4<f32>,  
    anchor: vec2<f32>,
};

@group(2) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_position: vec4<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) color: vec3<f32>,
    @location(4) random: u32,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let camera_dir = (bindings::view.view * vec4(0.0, 0.0, 1.0, 0.0)).xyz;
    let facing = normalize(camera_dir * vec3(1.0, 1.0, 0.0));

    let instance_transform = mat3x3(
        cross(facing, vec3(0.0, 0.0, 1.0)),
        vec3(0.0, 0.0, 1.0),
        facing
    );

    let world_origin_pos = (uniforms.transform * vec4(vertex.i_pos, 1.0)).xyz;
    let world_pos = world_origin_pos + instance_transform * vec3(
        (vertex.uv.x - uniforms.anchor.x) * vertex.i_size.x,
        (-vertex.uv.y + uniforms.anchor.y) * vertex.i_size.y * 16.0 / 14.0,
        0.0,
    );

    let world_normal = vec3(0.0, 1.0, 0.0);

    var out: VertexOutput;
    out.position = bindings::view.view_proj * vec4(world_pos, 1.0);
    out.uv = vertex.uv;
    out.world_position = vec4(world_origin_pos + vec3(0.0, 0.0, 0.01), 1.0);
    out.world_normal = world_normal;
    out.color = vertex.i_color;
    out.random = vertex.i_random;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv * vec2(0.125, 0.5);
    if in.random % 100u > 90u {
        uv.y += 0.5;
    }
    uv.x += f32(in.random % 4u) / 4.0;
    
    let color = vec4(in.color, 1.0) * textureSample(texture, texture_sampler, uv);
    if color.a < 0.5 {
        discard;
    }

    let albedo = color.rgb;

    var pixel_input: pixel::PixelInput;
    pixel_input.frag_coord = in.position;
    pixel_input.mesh_position = in.world_position;
    pixel_input.mesh_normal = in.world_normal;
    pixel_input.mesh_albedo = albedo;
    pixel_input.bands = 10u;
    pixel_input.dither = true;
    pixel_input.dither_offset = vec2(0u, 0u); //material.dither_offset;

    var out_color = pixel::process_all_lights(pixel_input);
    return vec4(out_color, 1.0);
}
