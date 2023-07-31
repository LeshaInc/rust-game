use std::sync::Arc;

use bevy::prelude::*;
use rand::SeedableRng;
use rand_pcg::Pcg32;
use rg_core::Grid;
use rg_worldgen::{WorldMaps, WORLD_SCALE};

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

    let mut rng = Pcg32::seed_from_u64(seed);

    let overscan = 10;
    let size = UVec2::splat(CHUNK_TILES) + overscan * 2;
    let origin = -IVec2::splat(overscan as i32);

    let mut noise = Grid::new(size, 0.0).with_origin(origin + chunk_pos * (CHUNK_TILES as i32));
    noise.add_fbm_noise(&mut rng, 0.01, 8.0, 5);

    noise = noise.with_origin(origin);

    let mut height_map = Grid::from_fn_with_origin(size, origin, |cell| {
        let pos = tile_pos_to_world(chunk_pos, cell);

        let mut height = world_maps.height_map.sample(pos / WORLD_SCALE) * 80.0;
        let shore = world_maps.shore_map.sample(pos / WORLD_SCALE);

        height += (1.0 - shore) * noise[cell];

        let mut snapped = height;
        snapped /= 3.0;
        snapped = snapped.floor() + (70.0 * (snapped.fract() - 0.5)).tanh() * 0.5 + 0.5;
        snapped *= 3.0;

        let alpha = shore.powf(0.3);
        height = snapped * (1.0 - alpha) + height * alpha;

        height
    });

    height_map.blur(1);
    height_map.blur(1);

    for (cell, height) in height_map.entries_mut() {
        let pos = tile_pos_to_world(chunk_pos, cell);
        let river = world_maps.river_map.sample(pos / WORLD_SCALE);
        *height -= river * 3.0;
    }

    height_map.blur(2);
    height_map.blur(2);

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
        let river = world_maps.river_map.sample(pos / WORLD_SCALE);

        if river > 0.5 {
            return Tile::Sand;
        }

        Tile::Grass
    })
}
