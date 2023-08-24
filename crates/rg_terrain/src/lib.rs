#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod chunk;
mod grass;
mod maps;
mod scatter;
mod surface;
mod tiles;
mod utils;

use bevy::prelude::*;

pub use crate::chunk::{
    chunk_pos_to_world, tile_pos_to_world, update_origin, Chunk, ChunkDespawnRadius,
    ChunkFullyLoaded, ChunkPlugin, ChunkPos, ChunkSpawnCenter, ChunkSpawnRadius, Chunks,
    FloatingOrigin, WorldOrigin, WorldOriginChanged, CHUNK_SIZE, CHUNK_TILES, TILE_SIZE,
};
use crate::grass::GrassPlugin;
use crate::maps::MapsPlugin;
pub use crate::maps::{ChunkMaps, SharedChunkMaps};
use crate::scatter::ScatterPlugins;
use crate::surface::SurfacePlugin;
pub use crate::tiles::Tile;

pub const MAX_TASKS_IN_FLIGHT: usize = 4;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ChunkPlugin)
            .add_plugins(MapsPlugin)
            .add_plugins(SurfacePlugin)
            .add_plugins(GrassPlugin)
            .add_plugins(ScatterPlugins);
    }
}
