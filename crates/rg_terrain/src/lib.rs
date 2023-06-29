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
use rg_billboard::{BillboardMaterial, BillboardMaterialPlugin};
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
            .add_plugins(BillboardMaterialPlugin::<GrassMaterial>::default())
            .insert_resource(Chunks::default())
            .add_systems(Startup, startup)
            .add_systems(
                Update,
                (
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
    mut chunks: ResMut<Chunks>,
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

    for sx in -3..=3 {
        for sz in -3..=3 {
            let pos = IVec2::new(sx, sz);

            let new_chunk = commands.spawn((
                Chunk,
                ChunkPos(pos),
                material.clone(),
                Transform::from_translation(
                    chunk_cell_to_world(pos, IVec2::ZERO).extend(0.0).xzy(),
                ),
                GlobalTransform::default(),
                Visibility::Visible,
                ComputedVisibility::default(),
            ));

            chunks.insert(pos, new_chunk.id());
        }
    }
}
