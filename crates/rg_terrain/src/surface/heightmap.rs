use std::sync::Arc;

use bevy::prelude::*;
use rg_core::{Grid, SimplexNoise2};

use crate::chunk::CHUNK_TILES;
use crate::tile_pos_to_world;

const OVERSCAN: u32 = 10;

pub struct HeightmapGenerator {
    seed: u64,
    chunk_pos: IVec2,
    world_elevation: Arc<Grid<f32>>,
    heightmap: Grid<f32>,
}

impl HeightmapGenerator {
    pub fn new(seed: u64, chunk_pos: IVec2, world_elevation: Arc<Grid<f32>>) -> HeightmapGenerator {
        let heightmap = Grid::new_default(UVec2::splat(CHUNK_TILES) + OVERSCAN * 2)
            .with_origin(-IVec2::splat(OVERSCAN as i32));

        HeightmapGenerator {
            seed,
            chunk_pos,
            world_elevation,
            heightmap,
        }
    }

    pub fn generate(mut self) -> Grid<f32> {
        let _span = info_span!("chunk heightmap generator").entered();

        let noise = SimplexNoise2::new(self.seed);

        for (cell, height) in self.heightmap.entries_mut() {
            let pos = tile_pos_to_world(self.chunk_pos, cell);

            let elevation = self.world_elevation.sample(pos / 2.0);
            *height = elevation * 100.0;

            let mut fbm = 0.0;
            fbm += noise.get(pos / 100.0) * 10.0;
            fbm += noise.get(pos / 50.0) * 5.0;
            fbm += noise.get(pos / 25.0) * 2.5;
            fbm += noise.get(pos / 12.5) * 1.25;
            fbm += noise.get(pos / 6.25) * 0.625;

            *height += fbm * elevation.powf(0.1);

            *height /= 3.0;
            *height = height.floor() + (30.0 * (height.fract() - 0.5)).tanh() * 0.5 + 0.5;
            *height *= 3.0;
        }

        self.heightmap.blur(4);
        self.heightmap.blur(4);

        self.heightmap
    }
}
