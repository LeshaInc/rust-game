use bevy::prelude::*;
use rg_core::progress::ProgressWriter;
use rg_core::{Grid, Noise};
use serde::Deserialize;

use crate::{NoiseMaps, WorldgenStage};

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct HeightSettings {
    pub beach_size: f32,
    pub land_height: f32,
    pub peak_height: f32,
    pub ocean_depth: f32,
    pub warp_dist: f32,
}

pub fn generate_height_map(
    progress: &mut ProgressWriter<WorldgenStage>,
    settings: &HeightSettings,
    noise_maps: &NoiseMaps,
    island: &Grid<f32>,
) -> Grid<f32> {
    let _scope = info_span!("generate_height_map").entered();

    let mut height_map = progress.task(|| shape(settings, &noise_maps, island));

    progress.task(|| height_map.blur(2));
    progress.task(|| height_map.blur(2));

    height_map
}

fn shape(settings: &HeightSettings, noise_maps: &NoiseMaps, island: &Grid<f32>) -> Grid<f32> {
    let _scope = info_span!("shape").entered();

    let max_dist = island.max_value();

    Grid::par_from_fn(island.size(), |cell| {
        let dist = island[cell];

        let k = -1.0 - 0.5;
        let x = (dist / (2.0 * settings.beach_size) + 0.5).clamp(0.0, 1.0);
        let alpha = (1.0 - 1.0 / (1.0 + (1.0 / x - 1.0).powf(k))).clamp(0.0, 1.0);

        let warp = Vec2::from(noise_maps.height_warp.get(cell.as_vec2())) * 2.0 - 1.0;
        let warped_dist = island.sample(cell.as_vec2() + warp * settings.warp_dist);
        let dist = dist * (1.0 - alpha) + warped_dist * alpha;

        let mut height = settings.land_height * alpha - settings.ocean_depth * (1.0 - alpha);
        height +=
            (dist / max_dist).max(0.0).powi(4) * (settings.peak_height - settings.land_height);
        height
    })
}
