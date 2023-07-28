use bevy::prelude::*;
use rg_core::{Grid, SimplexNoise2};
use rg_worldgen::{WorldMaps, WORLD_SCALE};

use crate::{tile_pos_to_world, CHUNK_TILES};

pub fn generate_grass_density_map(
    seed: u64,
    chunk_pos: IVec2,
    world_maps: &WorldMaps,
) -> Grid<f32> {
    let _span = info_span!("generate_grass_density_map").entered();

    let noise = SimplexNoise2::new(seed);

    let origin = -IVec2::splat(1);
    let size = UVec2::splat(CHUNK_TILES + 2);

    Grid::from_fn_with_origin(size, origin, |cell| {
        let pos = tile_pos_to_world(chunk_pos, cell);

        let elevation = world_maps.elevation.sample(pos / WORLD_SCALE);
        if elevation <= 0.0 {
            return 0.0;
        }

        let mut fbm = 0.0;
        fbm += noise.get(pos / 12.0);
        fbm += noise.get(pos / 6.0) / 2.0;
        fbm += noise.get(pos / 3.0) / 4.0;
        fbm += noise.get(pos / 1.5) / 8.0;
        fbm /= 1.0 + 0.5 + 0.25 + 0.125;

        fbm.clamp(0.0, 1.0).powf(1.0)
    })
}
