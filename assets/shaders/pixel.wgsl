#import bevy_pbr::utils
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::prepass_utils
#import bevy_pbr::shadows

struct PixelMaterial {
    color: vec4<f32>,
    bands: u32,
    dither_offset: vec2<u32>,
};

@group(1) @binding(0)
var<uniform> material: PixelMaterial;

fn single_light(
    frag_coord: vec2<f32>,
    mesh_normal: vec3<f32>,
    mesh_albedo: vec3<f32>,
    light_incident: vec3<f32>,
    light_color: vec3<f32>,
    light_attenuation: f32,
) -> vec3<f32> {
    var light = saturate(dot(mesh_normal, light_incident));
    light *= light_attenuation;
    light *= f32(material.bands);

#ifdef DITHER_ENABLED
    var dither_matrix = mat4x4<f32>(
        vec4<f32>( 0.0 / 16.0, 12.0 / 16.0,  3.0 / 16.0, 15.0 / 16.0),
        vec4<f32>( 8.0 / 16.0,  4.0 / 16.0, 11.0 / 16.0,  7.0 / 16.0),
        vec4<f32>( 2.0 / 16.0, 14.0 / 16.0,  1.0 / 16.0, 13.0 / 16.0),
        vec4<f32>(10.0 / 16.0,  6.0 / 16.0,  9.0 / 16.0,  5.0 / 16.0)
    );
    let idx = (vec2<u32>(frag_coord) + material.dither_offset) % 4u;
    let bayer = dither_matrix[idx.x][idx.y];
    light = mix(floor(light), ceil(light), f32(fract(light) > bayer));
#elseif
    light = round(light);
#endif

    light /= f32(material.bands);
    
    return mesh_albedo * light_color * light;
}

fn all_lights(
    frag_coord: vec4<f32>,
    world_position: vec4<f32>,
    mesh_normal: vec3<f32>,
    mesh_albedo: vec3<f32>,
) -> vec3<f32> {
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), world_position);

    var out_color = mesh_albedo * lights.ambient_color.rgb;

    let n_directional_lights = lights.n_directional_lights;
    for (var i: u32 = 0u; i < n_directional_lights; i++) {
        let light = &lights.directional_lights[i];
        let shadow = step(0.5, fetch_directional_shadow(i, world_position, mesh_normal, view_z));
        out_color += single_light(
            frag_coord.xy, mesh_normal, mesh_albedo,
            (*light).direction_to_light, (*light).color.rgb, shadow
        );
    }

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

fn get_linear_depth(frag_coord: vec2<f32>) -> f32 {
    let raw_depth = prepass_depth(vec4(frag_coord, 0.0, 1.0), 0u);
    let clip_pos = vec4(vec2(0.0, 0.0), raw_depth, 1.0);
    let view_space = view.inverse_projection * clip_pos;
    return -view_space.z / view_space.w;
}

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


fn get_view_normal(frag_coord: vec2<f32>) -> vec3<f32> {
    let view_mat = mat3x3(view.view[0].xyz, view.view[1].xyz, view.view[2].xyz);
    return view_mat * prepass_normal(vec4(frag_coord, 0.0, 1.0), 0u);
}

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

@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    let depth_samples = get_depth_samples(frag_coord.xy);
    let normal_samples = get_normal_samples(frag_coord.xy);

    let is_depth_edge = check_depth_edge(depth_samples, 0.3);
    let is_normal_edge = check_normal_edge(depth_samples, normal_samples, 1.0);

    var albedo = material.color.rgb;
    if (is_depth_edge) {
        albedo *= 0.5;
    } else if (is_normal_edge) {
        albedo *= 0.5;
    }

    var out_color = all_lights(frag_coord, world_position, world_normal, albedo);

    out_color = mix(out_color, vec3(1.0), 0.03 * smoothstep(2.0, 6.0, world_position.y));
    out_color = mix(out_color, vec3(0.0), 0.5 * smoothstep(2.0, 6.0, -world_position.y));
    
    return vec4<f32>(out_color, 1.0);
}
