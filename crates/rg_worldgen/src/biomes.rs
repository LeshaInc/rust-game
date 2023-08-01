use bevy::prelude::*;
use bytemuck::{CheckedBitPattern, NoUninit};
use rg_core::{Grid, Noise};

use crate::{NoiseMaps, WorldgenProgress, WorldgenStage};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, NoUninit, CheckedBitPattern)]
#[repr(u8)]
pub enum Biome {
    Ocean,
    Plains,
    Forest,
}

pub fn generate_biome_map(
    progress: &WorldgenProgress,
    noise_maps: &NoiseMaps,
    height_map: &Grid<f32>,
) -> Grid<Biome> {
    let _scope = info_span!("generate_biome_map").entered();

    progress.set(WorldgenStage::Biomes, 0);

    let size = height_map.size();
    let biome_map = Grid::par_from_fn(size, |cell| {
        if height_map[cell] < 0.0 {
            return Biome::Ocean;
        }

        let noise = noise_maps.biomes.get(cell.as_vec2())[0];
        if noise > 0.5 {
            Biome::Forest
        } else {
            Biome::Plains
        }
    });

    progress.set(WorldgenStage::Biomes, 100);

    biome_map
}
