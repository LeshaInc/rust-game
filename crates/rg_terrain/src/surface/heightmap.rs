use bevy::prelude::*;
use rg_core::{Grid, SimplexNoise2};
use rg_worldgen::{WorldMaps, RIVER_MAP_SCALE, WORLD_SCALE};

use crate::chunk::CHUNK_TILES;
use crate::tile_pos_to_world;

const OVERSCAN: u32 = 10;

pub fn generate_heightmap(seed: u64, chunk_pos: IVec2, world_maps: &WorldMaps) -> Grid<f32> {
    let _span = info_span!("generate_heightmap").entered();

    let noise = SimplexNoise2::new(seed);

    let size = UVec2::splat(CHUNK_TILES) + OVERSCAN * 2;
    let origin = -IVec2::splat(OVERSCAN as i32);

    let mut heightmap = Grid::from_fn_with_origin(size, origin, |cell| {
        let pos = tile_pos_to_world(chunk_pos, cell);

        let elevation = world_maps.elevation.sample(pos / WORLD_SCALE);
        let river = world_maps.rivers.sample(pos / RIVER_MAP_SCALE);

        let mut height = elevation * 160.0;

        let mut fbm = 0.0;
        fbm += noise.get(pos / 100.0);
        fbm += noise.get(pos / 50.0) / 2.0;
        fbm += noise.get(pos / 25.0) / 4.0;
        fbm += noise.get(pos / 12.5) / 8.0;
        fbm += noise.get(pos / 6.25) / 16.0;
        fbm += noise.get(pos / 3.125) / 32.0;

        height += (1.0 - river) * 14.0 * fbm * elevation.max(0.0).powf(0.5);

        let mut snap = height;
        snap /= 2.0;
        snap = snap.floor() + (70.0 * (snap.fract() - 0.5)).tanh() * 0.5 + 0.5;
        snap *= 2.0;

        height = snap * (1.0 - river) + height * river;
        height
    });

    heightmap.blur(3);
    heightmap.blur(3);

    for (cell, height) in heightmap.entries_mut() {
        let pos = tile_pos_to_world(chunk_pos, cell);
        let river = world_maps.rivers.sample(pos / RIVER_MAP_SCALE);
        *height -= river * 2.0;
    }

    heightmap.blur(1);
    heightmap.blur(1);

    heightmap
}
