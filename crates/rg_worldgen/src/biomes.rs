use bevy::prelude::*;
use rand::Rng;
use rg_core::Grid;

use crate::{WorldgenProgress, WorldgenStage};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Biome {
    Ocean,
    Plains,
    Forest,
}

pub fn generate_biome_map<R: Rng>(
    rng: &mut R,
    progress: &WorldgenProgress,
    height_map: &Grid<f32>,
) -> Grid<Biome> {
    let _scope = info_span!("generate_biome_map").entered();

    progress.set(WorldgenStage::Biomes, 0);

    let size = height_map.size();

    let mut biomes = Grid::new(size, Biome::Ocean);
    let mut noise = Grid::new(size, 0.0);
    noise.add_fbm_noise(rng, 0.1, 1.0, 3);

    progress.set(WorldgenStage::Biomes, 50);

    for cell in biomes.cells() {
        if height_map[cell] < 0.0 {
            continue;
        }

        biomes[cell] = if noise[cell] > 0.5 {
            Biome::Forest
        } else {
            Biome::Plains
        }
    }

    progress.set(WorldgenStage::Biomes, 100);

    biomes
}
