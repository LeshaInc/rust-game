#define DITHER_ENABLED

#ifdef PREPASS
#import bevy_pbr::prepass_bindings as bindings
#else
#import bevy_pbr::mesh_view_bindings as bindings
#import rg::pixel_funcs as pixel
#endif

struct Vertex {
    @location(0) uv: vec2<f32>,

    @location(1) i_pos: vec3<f32>,
    @location(2) i_normal: vec3<f32>,
    @location(3) i_size: vec2<f32>,
    @location(4) i_color: vec3<f32>,
    @location(5) i_random: u32,
};

struct LeavesMaterial {
    dither_offset: vec2<u32>,
    fog_height: f32,
};

@group(1) @binding(0)
var<uniform> material: LeavesMaterial;

@group(1) @binding(1)
var texture: texture_2d<f32>;
@group(1) @binding(2)
var texture_sampler: sampler;

@group(1) @binding(3)
var noise: texture_2d<f32>;
@group(1) @binding(4)
var noise_sampler: sampler;

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
    let world_origin_pos = (uniforms.transform * vec4(vertex.i_pos, 1.0)).xyz;
    let noise = textureSampleLevel(noise, noise_sampler, (world_origin_pos.xy + world_origin_pos.z) / 5.0 % 1.0, 0.0);
    let translate = sin(2.0 * bindings::globals.time + noise.x * 10.0) * 0.1;

    let camera_dir = (bindings::view.view * vec4(0.0, 0.0, 1.0, 0.0)).xyz;
    let facing = normalize(camera_dir * vec3(1.0, 1.0, 0.0));

    let instance_transform = mat3x3(
        cross(facing, vec3(0.0, 0.0, 1.0)),
        vec3(0.0, 0.0, 1.0),
        facing
    );

    let world_pos = world_origin_pos + instance_transform * vec3(
        (vertex.uv.x - uniforms.anchor.x + translate) * vertex.i_size.x,
        (-vertex.uv.y + uniforms.anchor.y) * vertex.i_size.y * 16.0 / 14.0,
        0.0,
    );

    let world_normal = normalize((uniforms.transform * vec4(vertex.i_normal, 0.0)).xyz);

    var out: VertexOutput;
    out.position = bindings::view.view_proj * vec4(world_pos, 1.0);
    out.uv = vertex.uv;
    out.world_position = vec4(world_pos, 1.0);
    out.world_normal = world_normal;
    out.color = vertex.i_color * (0.9 + (f32(vertex.i_random) / 4294967295.0) * 0.1);
    out.random = vertex.i_random;
    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let color = vec4(in.color, 1.0) * textureSample(texture, texture_sampler, in.uv);
    if color.a < 0.5 {
        discard;
    }

    var normal = in.world_normal;
    if front_facing {
        normal = -normal;
    }

#ifdef PREPASS
    return vec4(normal * 0.5 + 0.5, 1.0);
#else
    let depth_samples = pixel::get_depth_samples(in.position.xy);
    let is_edge = pixel::check_depth_edge(depth_samples, 1.0);

    var albedo = color.rgb;
    albedo = mix(albedo, albedo * 0.5, f32(is_edge));

    var pixel_input: pixel::PixelInput;
    pixel_input.frag_coord = in.position;
    pixel_input.mesh_position = in.world_position;
    pixel_input.mesh_normal = normal;
    pixel_input.mesh_albedo = albedo;
    pixel_input.bands = 4u;
    pixel_input.dither = true;
    pixel_input.dither_offset = material.dither_offset;
    pixel_input.fog_height = material.fog_height;

    var out_color = pixel::process_all_lights(pixel_input);
    return vec4<f32>(out_color, 1.0);
#endif
}
