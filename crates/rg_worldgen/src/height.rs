use bevy::prelude::*;
use rand::Rng;
use rg_core::Grid;
use serde::Deserialize;

use crate::{WorldgenProgress, WorldgenStage};

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct HeightSettings {
    pub beach_size: f32,
    pub land_height: f32,
    pub peak_height: f32,
    pub ocean_depth: f32,
    pub warp_scale: f32,
    pub warp_dist: f32,
}

pub fn generate_height_map<R: Rng>(
    rng: &mut R,
    progress: &WorldgenProgress,
    settings: &HeightSettings,
    island: &Grid<f32>,
) -> Grid<f32> {
    let _scope = info_span!("generate_height_map").entered();

    progress.set(WorldgenStage::Height, 0);

    let warp_map = generate_warp_map(rng, settings, island.size());
    progress.set(WorldgenStage::Height, 30);

    let mut height_map = shape(settings, island, &warp_map);
    progress.set(WorldgenStage::Height, 60);

    height_map.blur(2);
    progress.set(WorldgenStage::Height, 80);

    height_map.blur(2);
    progress.set(WorldgenStage::Height, 100);

    height_map
}

fn generate_warp_map<R: Rng>(rng: &mut R, settings: &HeightSettings, size: UVec2) -> Grid<Vec2> {
    let scale = size.x.min(size.y) as f32 / settings.warp_scale;

    let mut x_map = Grid::new(size, 0.0);
    x_map.add_fbm_noise(rng, scale, 1.0, 4);

    let mut y_map = Grid::new(size, 0.0);
    y_map.add_fbm_noise(rng, scale, 1.0, 4);

    Grid::from_fn(size, |cell| {
        (Vec2::new(x_map[cell], y_map[cell]) - 0.5) * 2.0 * settings.warp_dist
    })
}

fn shape(settings: &HeightSettings, island: &Grid<f32>, warp_map: &Grid<Vec2>) -> Grid<f32> {
    let _scope = info_span!("shape").entered();

    let mut height_map = Grid::new(island.size(), 0.0);

    let max_dist = island.max_value();

    for cell in height_map.cells() {
        let dist = island[cell];

        let k = -1.0 - 0.5;
        let x = (dist / (2.0 * settings.beach_size) + 0.5).clamp(0.0, 1.0);
        let alpha = (1.0 - 1.0 / (1.0 + (1.0 / x - 1.0).powf(k))).clamp(0.0, 1.0);

        let warped_dist = island.sample(cell.as_vec2() + warp_map[cell]);
        let dist = dist * (1.0 - alpha) + warped_dist * alpha;

        let mut height = settings.land_height * alpha - settings.ocean_depth * (1.0 - alpha);
        height +=
            (dist / max_dist).max(0.0).powi(4) * (settings.peak_height - settings.land_height);
        height_map[cell] = height;
    }

    height_map
}
