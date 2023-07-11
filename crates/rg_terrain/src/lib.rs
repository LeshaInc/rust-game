#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod chunk;
mod grass;
mod scatter;
mod surface;
mod utils;

use bevy::prelude::*;

pub use crate::chunk::{
    chunk_pos_to_world, tile_pos_to_world, Chunk, ChunkDespawnRadius, ChunkPlugin, ChunkPos,
    ChunkSpawnCenter, ChunkSpawnRadius, Chunks, CHUNK_SIZE, CHUNK_TILES, TILE_SIZE,
};
use crate::grass::GrassPlugin;
use crate::scatter::ScatterPlugin;
use crate::surface::SurfacePlugin;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ChunkPlugin)
            .add_plugins(SurfacePlugin)
            .add_plugins(GrassPlugin)
            .add_plugins(ScatterPlugin);
    }
}
