use std::sync::Arc;

use bevy::core::{cast_slice, Pod, Zeroable};
use bevy::ecs::system::lifetimeless::SRes;
use bevy::ecs::system::SystemParamItem;
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::primitives::Aabb;
use bevy::render::render_asset::{PrepareAssetError, RenderAsset};
use bevy::render::render_resource::{
    Buffer, BufferInitDescriptor, BufferUsages, ShaderType, VertexBufferLayout, VertexFormat,
    VertexStepMode,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::view::NoFrustumCulling;
use bevy::render::Extract;

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct BillboardInstance {
    pub pos: Vec3,
    pub size: Vec2,
    pub color: Vec3,
    pub random: u32,
}

impl BillboardInstance {
    pub fn vertex_buffer_layout() -> VertexBufferLayout {
        let mut layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Instance,
            [
                VertexFormat::Float32x3,
                VertexFormat::Float32x2,
                VertexFormat::Float32x3,
                VertexFormat::Uint32,
            ],
        );

        // vertex buffer comes first
        for attr in &mut layout.attributes {
            attr.shader_location += 1;
        }

        layout
    }
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct BillboardVertex {
    pub pos: Vec2,
}

impl BillboardVertex {
    pub fn vertex_buffer_layout() -> VertexBufferLayout {
        VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, [VertexFormat::Float32x2])
    }
}

#[derive(Debug, Clone, TypeUuid, TypePath)]
#[uuid = "75c2e3e7-ce11-441a-9751-0dd4556f8bda"]
pub struct MultiBillboard {
    pub instances: Arc<[BillboardInstance]>,
    pub anchor: Vec2,
}

impl MultiBillboard {
    pub fn compute_aabb(&self) -> Option<Aabb> {
        let positions = self.instances.iter().map(|v| v.pos);
        let min = positions.clone().reduce(Vec3::min)?;
        let max = positions.reduce(Vec3::max)?;

        let max_size = self.instances.iter().map(|v| v.size).reduce(Vec2::max)?;

        let mut aabb = Aabb::from_min_max(min, max);
        aabb.half_extents.x += max_size.x;
        aabb.half_extents.z += max_size.y;

        Some(aabb)
    }
}

pub fn compute_multi_billboard_bounds(
    q_multi_billboards: Query<
        (Entity, &Handle<MultiBillboard>),
        (Without<Aabb>, Without<NoFrustumCulling>),
    >,
    mut commands: Commands,
    multi_billboards: Res<Assets<MultiBillboard>>,
) {
    for (entity, handle) in &q_multi_billboards {
        let Some(multi_billboard) = multi_billboards.get(handle) else {
            continue;
        };

        if let Some(aabb) = multi_billboard.compute_aabb() {
            commands.entity(entity).insert(aabb);
        };
    }
}

#[derive(Debug, Clone)]
pub struct GpuMultiBillboard {
    pub num_instances: u32,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub instance_buffer: Buffer,
}

impl RenderAsset for MultiBillboard {
    type ExtractedAsset = MultiBillboard;
    type PreparedAsset = GpuMultiBillboard;
    type Param = SRes<RenderDevice>;

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        multi_billboard: Self::ExtractedAsset,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let vertex_buffer = create_vertex_buffer(render_device);
        let index_buffer = create_index_buffer(render_device);
        let instance_buffer = create_instance_buffer(render_device, &multi_billboard.instances);
        Ok(GpuMultiBillboard {
            num_instances: multi_billboard.instances.len() as u32,
            vertex_buffer,
            index_buffer,
            instance_buffer,
        })
    }
}

fn create_vertex_buffer(device: &RenderDevice) -> Buffer {
    let uvs = [
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 0.0),
    ];

    device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Billboard Vertex Buffer"),
        contents: cast_slice(&uvs),
        usage: BufferUsages::VERTEX,
    })
}

const QUAD_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

fn create_index_buffer(device: &RenderDevice) -> Buffer {
    device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Billboard Index Buffer"),
        contents: cast_slice(&QUAD_INDICES),
        usage: BufferUsages::INDEX,
    })
}

fn create_instance_buffer(device: &RenderDevice, instances: &[BillboardInstance]) -> Buffer {
    device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Billboard Instance Buffer"),
        contents: cast_slice(instances),
        usage: BufferUsages::VERTEX,
    })
}

#[derive(Debug, Clone, Copy, Component, ShaderType)]
pub struct MultiBillboardUniform {
    pub transform: Mat4,
    pub anchor: Vec2,
}

pub fn extract_multi_billboards(
    q_multi_billboards: Extract<
        Query<(
            Entity,
            &Handle<MultiBillboard>,
            &GlobalTransform,
            &ComputedVisibility,
        )>,
    >,
    multi_billboards: Extract<Res<Assets<MultiBillboard>>>,
    mut commands: Commands,
) {
    for (entity, multi_billboard_handle, transform, visibility) in &q_multi_billboards {
        if !visibility.is_visible() {
            continue;
        }

        let Some(multi_billboard) = multi_billboards.get(multi_billboard_handle) else {
            continue;
        };

        commands.get_or_spawn(entity).insert((
            multi_billboard_handle.clone(),
            MultiBillboardUniform {
                transform: transform.compute_matrix(),
                anchor: multi_billboard.anchor,
            },
        ));
    }
}
