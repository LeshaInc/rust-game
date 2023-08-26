use bevy::prelude::*;
use rg_core::grid::Grid;
use rg_core::noise::Noise;
use rg_core::progress::ProgressStage;
use rg_worldgen_api::{Biome, NoiseMaps};

pub fn generate_biome_map(
    progress: &mut ProgressStage,
    noise_maps: &NoiseMaps,
    height_map: &Grid<f32>,
) -> Grid<Biome> {
    let _scope = info_span!("generate_biome_map").entered();

    let size = height_map.size();
    progress.task(|| {
        Grid::par_from_fn(size, |cell| {
            if height_map[cell] < 0.0 {
                return Biome::Ocean;
            }

            let noise = noise_maps.biomes.get(cell.as_vec2())[0];
            if noise > 0.5 {
                Biome::Forest
            } else {
                Biome::Plains
            }
        })
    })
}
