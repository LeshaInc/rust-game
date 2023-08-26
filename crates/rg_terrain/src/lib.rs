#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod grass;
mod maps;
mod scatter;
mod surface;
mod tiles;
mod utils;

use bevy::prelude::*;

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
        app.add_plugins(MapsPlugin)
            .add_plugins(SurfacePlugin)
            .add_plugins(GrassPlugin)
            .add_plugins(ScatterPlugins);
    }
}
