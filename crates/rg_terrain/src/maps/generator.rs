use std::sync::Arc;

use bevy::prelude::*;
use rg_core::{Grid, SimplexNoise2};
use rg_worldgen::{WorldMaps, RIVER_MAP_SCALE, WORLD_SCALE};

use super::{ChunkMaps, SharedChunkMaps};
use crate::{tile_pos_to_world, Tile, CHUNK_TILES};

pub fn generate_maps(seed: u64, chunk_pos: IVec2, world_maps: &WorldMaps) -> SharedChunkMaps {
    let _span = info_span!("generate_maps").entered();

    let height_map = generate_height_map(seed, chunk_pos, world_maps);
    let tile_map = generate_tile_map(chunk_pos, world_maps, &height_map);

    SharedChunkMaps(Arc::new(ChunkMaps {
        height_map,
        tile_map,
    }))
}

fn generate_height_map(seed: u64, chunk_pos: IVec2, world_maps: &WorldMaps) -> Grid<f32> {
    let _span = info_span!("generate_height_map").entered();

    let noise = SimplexNoise2::new(seed);

    let overscan = 10;
    let size = UVec2::splat(CHUNK_TILES) + overscan * 2;
    let origin = -IVec2::splat(overscan as i32);

    let mut height_map = Grid::from_fn_with_origin(size, origin, |cell| {
        let pos = tile_pos_to_world(chunk_pos, cell);

        let mut height = world_maps.height_map.sample(pos / WORLD_SCALE) * 160.0;
        let river = world_maps.river_map.sample(pos / RIVER_MAP_SCALE);

        let mut fbm = 0.0;
        fbm += noise.get(pos / 100.0);
        fbm += noise.get(pos / 50.0) / 2.0;
        fbm += noise.get(pos / 25.0) / 4.0;
        fbm += noise.get(pos / 12.5) / 8.0;
        fbm += noise.get(pos / 6.25) / 16.0;
        fbm += noise.get(pos / 3.125) / 32.0;

        height += (1.0 - river) * 14.0 * fbm * (height / 160.0).max(0.0).powf(0.5);

        let mut snap = height;
        snap /= 2.0;
        snap = snap.floor() + (70.0 * (snap.fract() - 0.5)).tanh() * 0.5 + 0.5;
        snap *= 2.0;

        height = snap * (1.0 - river) + height * river;
        height
    });

    height_map.blur(3);
    height_map.blur(3);

    for (cell, height) in height_map.entries_mut() {
        let pos = tile_pos_to_world(chunk_pos, cell);
        let river = world_maps.river_map.sample(pos / RIVER_MAP_SCALE);
        *height -= river * 2.0;
    }

    height_map.blur(1);
    height_map.blur(1);

    height_map
}

fn generate_tile_map(
    chunk_pos: IVec2,
    world_maps: &WorldMaps,
    height_map: &Grid<f32>,
) -> Grid<Tile> {
    let _span = info_span!("generate_tile_map").entered();

    let size = UVec2::splat(CHUNK_TILES);
    Grid::from_fn(size, |cell| {
        if height_map[cell] < 0.0 {
            return Tile::Sand;
        }

        let pos = tile_pos_to_world(chunk_pos, cell);
        let river = world_maps.river_map.sample(pos / RIVER_MAP_SCALE);

        if river > 0.5 {
            return Tile::Sand;
        }

        Tile::Grass
    })
}
