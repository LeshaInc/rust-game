#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings

struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,

    @location(2) i_pos: vec3<f32>,
    @location(3) i_size: vec2<f32>,
    @location(4) i_color: vec3<f32>,
    @location(5) i_uv_rect: vec4<f32>,
};

struct Uniforms {
    transform: mat4x4<f32>,  
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let position = vec3(vertex.pos * vertex.i_size, 0.0) + vertex.i_pos;
    var out: VertexOutput;
    out.clip_position = view.view_proj * uniforms.transform * vec4<f32>(position, 1.0);
    out.color = vertex.i_color;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(in.color, 1.0);
}
