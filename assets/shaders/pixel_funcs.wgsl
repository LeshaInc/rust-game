#define_import_path rg::pixel_funcs

#import bevy_pbr::utils
#import bevy_pbr::mesh_view_bindings as bindings
#import bevy_pbr::prepass_utils::{prepass_depth, prepass_normal}
#import bevy_pbr::shadows::fetch_directional_shadow

struct PixelInput {
    frag_coord: vec4<f32>,
    mesh_position: vec4<f32>,
    mesh_normal: vec3<f32>,
    mesh_albedo: vec3<f32>,
    bands: u32,
    dither: bool,
    dither_offset: vec2<u32>,
    fog_height: f32,
}

fn process_single_light(
    in: PixelInput,
    light_incident: vec3<f32>,
    light_color: vec3<f32>,
    light_attenuation: f32,
) -> vec3<f32> {
    var light = saturate(dot(in.mesh_normal, light_incident));
    light *= light_attenuation;
    light *= f32(in.bands);

#ifdef DITHER_ENABLED
    var dither_matrix = mat4x4<f32>(
        vec4<f32>( 0.0 / 16.0, 12.0 / 16.0,  3.0 / 16.0, 15.0 / 16.0),
        vec4<f32>( 8.0 / 16.0,  4.0 / 16.0, 11.0 / 16.0,  7.0 / 16.0),
        vec4<f32>( 2.0 / 16.0, 14.0 / 16.0,  1.0 / 16.0, 13.0 / 16.0),
        vec4<f32>(10.0 / 16.0,  6.0 / 16.0,  9.0 / 16.0,  5.0 / 16.0)
    );
    let idx = (vec2<u32>(in.frag_coord.xy) + in.dither_offset) % 4u;
    let bayer = dither_matrix[idx.x][idx.y];
    light = mix(round(light), mix(floor(light), ceil(light), f32(fract(light) > bayer)), f32(in.dither));
#elseif
    light = round(light);
#endif

    light /= f32(in.bands);
    
    return in.mesh_albedo * light_color * light;
}

fn process_all_lights(in: PixelInput) -> vec3<f32> {
    let view_z = dot(vec4<f32>(
        bindings::view.inverse_view[0].z,
        bindings::view.inverse_view[1].z,
        bindings::view.inverse_view[2].z,
        bindings::view.inverse_view[3].z
    ), in.mesh_position);

    var out_color = in.mesh_albedo * bindings::lights.ambient_color.rgb;

    let n_directional_lights = bindings::lights.n_directional_lights;
    for (var i: u32 = 0u; i < n_directional_lights; i++) {
        let light = &bindings::lights.directional_lights[i];
#ifndef DISABLE_SHADOWS
        let shadow = step(0.5, fetch_directional_shadow(i, in.mesh_position, in.mesh_normal, view_z));
#elseif
        let shadow = 1.0;
#endif
        out_color += process_single_light(in, (*light).direction_to_light, (*light).color.rgb, shadow);
    }

    // down fog
    out_color = mix(out_color, vec3(0.5, 0.5, 1.0), 0.05 * smoothstep(0.0, 3.0, -in.mesh_position.z + in.fog_height));
    out_color = mix(out_color, vec3(0.5, 0.5, 1.0), 0.8 * smoothstep(2.0, 10.0, -in.mesh_position.z + in.fog_height));

    // up fog
    out_color = mix(out_color, vec3(1.0, 0.5, 0.7), 0.1 * smoothstep(3.0, 10.0, in.mesh_position.z - in.fog_height));

    // far fog
    out_color = mix(out_color, vec3(1.0), smoothstep(0.95, 1.0, 1.0 - in.frag_coord.z));

    return out_color;
}

struct DepthSamples {
    c: f32,
    u: f32,
    d: f32,
    l: f32,
    r: f32,
};

struct NormalSamples {
    c: vec3<f32>,
    u: vec3<f32>,
    d: vec3<f32>,
    l: vec3<f32>,
    r: vec3<f32>,
};

fn raw_depth_to_linear(raw: f32) -> f32 {
    let clip_pos = vec4(vec2(0.0, 0.0), raw, 1.0);
    let view_space = bindings::view.inverse_projection * clip_pos;
    return -view_space.z / view_space.w;
}

#ifdef DEPTH_PREPASS
fn get_linear_depth(frag_coord: vec2<f32>) -> f32 {
    let raw_depth = prepass_depth(vec4(frag_coord, 0.0, 1.0), 0u);
    return raw_depth_to_linear(raw_depth);
}
#elseif
fn get_linear_depth(frag_coord: vec2<f32>) -> f32 {
    return 0.0;
}
#endif

fn get_depth_samples(frag_coord: vec2<f32>) -> DepthSamples {
    var samples: DepthSamples;
    samples.c = get_linear_depth(frag_coord);
    samples.u = get_linear_depth(frag_coord + vec2( 0.0, -1.0));
    samples.d = get_linear_depth(frag_coord + vec2( 0.0,  1.0));
    samples.l = get_linear_depth(frag_coord + vec2(-1.0,  0.0));
    samples.r = get_linear_depth(frag_coord + vec2( 1.0,  0.0));
    return samples;
}

fn check_depth_edge(s: DepthSamples, treshold: f32) -> bool {
    let edge = saturate(s.u - s.c) + saturate(s.d - s.c) + saturate(s.l - s.c) + saturate(s.r - s.c);
    return edge > treshold;
}


#ifdef DEPTH_PREPASS
fn get_view_normal(frag_coord: vec2<f32>) -> vec3<f32> {
    let view_mat = mat3x3(bindings::view.view[0].xyz, bindings::view.view[1].xyz, bindings::view.view[2].xyz);
    return view_mat * prepass_normal(vec4(frag_coord, 0.0, 1.0), 0u);
}
#elseif
fn get_view_normal(frag_coord: vec2<f32>) -> vec3<f32> {
    return vec3(0.0);
}
#endif

fn get_normal_samples(frag_coord: vec2<f32>) -> NormalSamples {
    var samples: NormalSamples;
    samples.c = get_view_normal(frag_coord);
    samples.u = get_view_normal(frag_coord + vec2( 0.0, -1.0));
    samples.d = get_view_normal(frag_coord + vec2( 0.0,  1.0));
    samples.l = get_view_normal(frag_coord + vec2(-1.0,  0.0));
    samples.r = get_view_normal(frag_coord + vec2( 1.0,  0.0));
    return samples;
}

fn normal_neighbour_edge(base_normal: vec3<f32>, new_normal: vec3<f32>, depth_diff: f32) -> f32 {
    let normal_diff = dot(base_normal - new_normal, vec3(-1.0, -1.0, -1.0));
    let normal_indicator = saturate(smoothstep(-0.025, 0.025, normal_diff));
    let depth_indicator = saturate(sign(depth_diff + 0.1));
    return (1.0 - dot(base_normal, new_normal)) * depth_indicator * normal_indicator;
}

fn check_normal_edge(ds: DepthSamples, ns: NormalSamples, treshold: f32) -> bool {
    let edge = normal_neighbour_edge(ns.c, ns.u, ds.u - ds.c)
             + normal_neighbour_edge(ns.c, ns.d, ds.d - ds.c)
             + normal_neighbour_edge(ns.c, ns.l, ds.l - ds.c)
             + normal_neighbour_edge(ns.c, ns.r, ds.r - ds.c);
    return edge > treshold;
}
