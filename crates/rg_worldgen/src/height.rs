use bevy::prelude::*;
use rg_core::progress::ProgressStage;
use rg_core::{Grid, Noise};
use serde::Deserialize;

use crate::NoiseMaps;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct HeightSettings {
    pub beach_size: f32,
    pub land_height: f32,
    pub peak_height: f32,
    pub ocean_depth: f32,
    pub warp_dist: f32,
    pub mountain_power: f32,
}

pub fn generate_height_map(
    progress: &mut ProgressStage,
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

        let a1 = settings.land_height;
        let a2 = settings.ocean_depth;
        let s1 = settings.beach_size;
        let s2 = s1 * a2 / a1;
        let k = 2.0 * a1 / s1;

        let mut height = if dist >= s1 {
            a1
        } else if dist <= -s2 {
            -a2
        } else if dist >= 0.0 {
            let x = dist / s1;
            a1 * ((k * s1 * x) / (a1 * (1.0 - x.powi(2)))).tanh()
        } else {
            let x = dist / s2;
            a2 * ((k * s2 * x) / (a2 * (1.0 - x.powi(2)))).tanh()
        };

        let warp = Vec2::from(noise_maps.height_warp.get(cell.as_vec2())) * 2.0 - 1.0;
        let warped_dist = island.sample(cell.as_vec2() + warp * settings.warp_dist);
        let alpha = (dist / settings.beach_size).min(1.0).max(0.0);
        let dist = dist * (1.0 - alpha) + warped_dist * alpha;

        height += (dist / max_dist).max(0.0).powf(settings.mountain_power)
            * (settings.peak_height - settings.land_height);

        height
    })
}
