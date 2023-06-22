use bevy::prelude::*;

use crate::{ChunkMap, CHUNK_RESOLUTION, CHUNK_SIZE};

#[derive(Debug, Default, Clone, Component)]
pub struct ChunkHeightmap(pub ChunkMap<f32>);

pub fn generate(_seed: u64, chunk_pos: IVec2) -> ChunkHeightmap {
    let mut heightmap = ChunkMap::default();
    let mut data = heightmap.make_mut();

    for sx in 0..CHUNK_RESOLUTION {
        for sz in 0..CHUNK_RESOLUTION {
            let fx = CHUNK_SIZE * (sx as f32) / (CHUNK_RESOLUTION as f32) + chunk_pos.x as f32;
            let fz = CHUNK_SIZE * (sz as f32) / (CHUNK_RESOLUTION as f32) + chunk_pos.y as f32;
            let y = fx.sin() * fz.cos() * 0.2;
            data.set(UVec2::new(sx, sz), y);
        }
    }

    ChunkHeightmap(heightmap)
}
