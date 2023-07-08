use std::sync::Arc;

use bevy::prelude::*;
use rg_core::{Grid, SimplexNoise2};

use crate::{tile_pos_to_world, CHUNK_TILES};

pub struct DensityMapGenerator {
    seed: u64,
    chunk_pos: IVec2,
    world_elevation: Arc<Grid<f32>>,
    density_map: Grid<f32>,
}

impl DensityMapGenerator {
    pub fn new(
        seed: u64,
        chunk_pos: IVec2,
        world_elevation: Arc<Grid<f32>>,
    ) -> DensityMapGenerator {
        let density_map = Grid::new_default(UVec2::splat(CHUNK_TILES));
        DensityMapGenerator {
            seed,
            chunk_pos,
            world_elevation,
            density_map,
        }
    }

    pub fn generate(mut self) -> Grid<f32> {
        let _span = info_span!("chunk grass density map generator").entered();

        let noise = SimplexNoise2::new(self.seed);

        for (cell, density) in self.density_map.entries_mut() {
            let pos = tile_pos_to_world(self.chunk_pos, cell);

            let elevation = self.world_elevation.sample(pos / 2.0);
            if elevation <= 0.0 {
                continue;
            }

            let mut fbm = 0.0;
            fbm += noise.get(pos / 6.0);
            fbm += noise.get(pos / 3.0) * 0.5;
            fbm += noise.get(pos / 1.5) * 0.25;
            fbm /= 1.0 + 0.5 + 0.25;

            *density = fbm.clamp(0.0, 1.0);
            if *density < 0.2 {
                *density = 0.0;
            }
        }

        self.density_map
    }
}