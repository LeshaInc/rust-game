mod chunks;
mod grass;
mod heightmap;
mod map;
mod mesher;
mod poisson;

use bevy::prelude::*;
use rg_pixel_material::PixelMaterial;

pub use crate::chunks::{Chunks, NEIGHBOR_DIRS};
pub use crate::heightmap::ChunkHeightmap;
pub use crate::map::{ChunkMap, ChunkMapRefMut};

pub const CHUNK_SIZE: f32 = 32.0;
pub const CHUNK_RESOLUTION: u32 = 64;

pub const MAX_UPDATES_PER_FRAME: usize = 32;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Seed(0))
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Component)]
pub struct Chunk;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Component)]
pub struct ChunkPos(pub IVec2);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Resource)]
pub struct Seed(pub u64);

#[derive(Debug, Clone, Resource)]
pub struct TerrainMaterial(pub Handle<PixelMaterial>);

fn startup(
    mut commands: Commands,
    mut chunks: ResMut<Chunks>,
    mut materials: ResMut<Assets<PixelMaterial>>,
) {
    let material = materials.add(PixelMaterial {
        color: Color::rgb(0.3, 0.7, 0.3),
        ..default()
    });

    commands.insert_resource(TerrainMaterial(material.clone()));

    for sx in -3..=3 {
        for sz in -3..=3 {
            let pos = IVec2::new(sx, sz);

            let new_chunk = commands.spawn((
                Chunk,
                ChunkPos(pos),
                material.clone(),
                Transform::from_xyz(CHUNK_SIZE * sx as f32, 0.0, CHUNK_SIZE * sz as f32),
                GlobalTransform::default(),
                Visibility::Visible,
                ComputedVisibility::default(),
            ));

            chunks.insert(pos, new_chunk.id());
        }
    }
}
