#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod chunks;
mod grass;
mod heightmap;
mod mesher;
mod poisson;
mod utils;

use bevy::asset::AssetPath;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::render_resource::AsBindGroup;
use bevy_rapier3d::prelude::CollisionGroups;
use rg_billboard::{BillboardMaterial, BillboardMaterialPlugin};
use rg_core::CollisionLayers;
use rg_pixel_material::PixelMaterial;

pub use crate::chunks::Chunks;
pub use crate::heightmap::ChunkHeightmap;

pub const CHUNK_SIZE: Vec2 = Vec2::splat(16.0);
pub const CHUNK_RESOLUTION: IVec2 = IVec2::splat(32);

pub const MAX_UPDATES_PER_FRAME: usize = 32;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Seed(0))
            .insert_resource(ChunkSpawnCenter(Vec2::ZERO))
            .insert_resource(ChunkSpawnRadius(70.0))
            .insert_resource(ChunkDespawnRadius(80.0))
            .add_plugins(BillboardMaterialPlugin::<GrassMaterial>::default())
            .insert_resource(Chunks::default())
            .add_systems(Startup, startup)
            .add_systems(
                Update,
                (
                    spawn_chunks,
                    despawn_chunks,
                    crate::heightmap::schedule_system,
                    crate::mesher::schedule_system,
                ),
            )
            .add_systems(
                Update,
                (
                    crate::heightmap::update_system,
                    crate::mesher::update_system,
                ),
            );
    }
}

pub fn chunk_cell_to_world(chunk_pos: IVec2, cell: IVec2) -> Vec2 {
    (cell.as_vec2() / CHUNK_RESOLUTION.as_vec2() + chunk_pos.as_vec2()) * CHUNK_SIZE
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Component)]
pub struct Chunk;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Component)]
pub struct ChunkPos(pub IVec2);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Resource)]
pub struct Seed(pub u64);

#[derive(Debug, Clone, Resource)]
pub struct TerrainMaterial(pub Handle<PixelMaterial>);

#[derive(Debug, Clone, Resource)]
pub struct TerrainGrassMaterial(pub Handle<GrassMaterial>);

#[derive(Debug, Default, Clone, Component, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "d36218ae-d090-4ef1-a660-a4579db53935"]
pub struct GrassMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

impl BillboardMaterial for GrassMaterial {
    fn vertex_shader() -> AssetPath<'static> {
        "shaders/grass.wgsl".into()
    }

    fn fragment_shader() -> AssetPath<'static> {
        "shaders/grass.wgsl".into()
    }
}

fn startup(
    mut commands: Commands,
    mut materials: ResMut<Assets<PixelMaterial>>,
    mut grass_materials: ResMut<Assets<GrassMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let material = materials.add(PixelMaterial {
        color: Color::rgb_u8(105, 172, 73),
        ..default()
    });

    commands.insert_resource(TerrainMaterial(material.clone()));

    commands.insert_resource(TerrainGrassMaterial(grass_materials.add(GrassMaterial {
        texture: asset_server.load("images/grass.png"),
    })));
}

#[derive(Copy, Clone, Resource)]
pub struct ChunkSpawnCenter(pub Vec2);

#[derive(Copy, Clone, Resource)]
pub struct ChunkSpawnRadius(pub f32);

#[derive(Copy, Clone, Resource)]
pub struct ChunkDespawnRadius(pub f32);

fn spawn_chunks(
    mut commands: Commands,
    mut chunks: ResMut<Chunks>,
    terrain_material: Res<TerrainMaterial>,
    center: Res<ChunkSpawnCenter>,
    radius: Res<ChunkSpawnRadius>,
) {
    let center = center.0;
    let radius = radius.0;

    let chunk_center = (center / CHUNK_SIZE).round().as_ivec2();
    let chunk_dist = (Vec2::new(radius, radius) / CHUNK_SIZE).ceil().as_ivec2();

    for sx in -chunk_dist.x..=chunk_dist.x {
        for sy in -chunk_dist.y..=chunk_dist.y {
            let chunk_pos = chunk_center + IVec2::new(sx, sy);
            let chunk_center = (chunk_pos.as_vec2() + Vec2::splat(0.5)) * CHUNK_SIZE;

            if chunk_center.distance_squared(center) > radius.powi(2) {
                continue;
            }

            if chunks.contains(chunk_pos) {
                continue;
            }

            let new_chunk = commands.spawn((
                Chunk,
                ChunkPos(chunk_pos),
                Transform::from_translation(
                    chunk_cell_to_world(chunk_pos, IVec2::ZERO)
                        .extend(0.0)
                        .xzy(),
                ),
                GlobalTransform::default(),
                CollisionGroups::new(
                    (CollisionLayers::STATIC_GEOMETRY | CollisionLayers::NAVMESH).into(),
                    (CollisionLayers::DYNAMIC_GEOMETRY | CollisionLayers::CHARACTER).into(),
                ),
                Visibility::Visible,
                ComputedVisibility::default(),
                terrain_material.0.clone(),
            ));

            chunks.insert(chunk_pos, new_chunk.id());
        }
    }
}

fn despawn_chunks(
    mut chunks: ResMut<Chunks>,
    mut commands: Commands,
    center: Res<ChunkSpawnCenter>,
    radius: Res<ChunkDespawnRadius>,
) {
    let center = center.0;
    let radius = radius.0;

    chunks.retain(|chunk_pos, chunk| {
        let chunk_center = (chunk_pos.as_vec2() + Vec2::splat(0.5)) * CHUNK_SIZE;

        if chunk_center.distance_squared(center) > radius.powi(2) {
            commands.entity(chunk).despawn_recursive();
            false
        } else {
            true
        }
    });
}
