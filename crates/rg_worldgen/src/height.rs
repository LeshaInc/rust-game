use bevy::prelude::*;
use rg_core::{EdtSettings, Grid};
use serde::Deserialize;

use crate::{WorldgenProgress, WorldgenStage};

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct HeightSettings {
    pub beach_size: f32,
    pub land_height: f32,
    pub ocean_depth: f32,
}

pub fn generate_height_map(
    progress: &WorldgenProgress,
    settings: &HeightSettings,
    island: &Grid<bool>,
) -> Grid<f32> {
    let _scope = info_span!("generate_height_map").entered();

    progress.set(WorldgenStage::Height, 0);

    let edt = island.compute_edt(EdtSettings {
        exact: true,
        invert: false,
        normalize: true,
        downsample: 4,
        padding: 0,
    });

    progress.set(WorldgenStage::Height, 25);

    let inv_edt = island.compute_edt(EdtSettings {
        exact: true,
        invert: true,
        normalize: true,
        downsample: 4,
        padding: 32,
    });

    progress.set(WorldgenStage::Height, 50);

    let mut height_map = shape(island, &edt, &inv_edt, settings);
    progress.set(WorldgenStage::Height, 75);

    height_map.blur(2);
    progress.set(WorldgenStage::Height, 90);

    height_map.blur(2);
    progress.set(WorldgenStage::Height, 100);

    height_map
}

fn shape(
    island: &Grid<bool>,
    edt: &Grid<f32>,
    inv_edt: &Grid<f32>,
    settings: &HeightSettings,
) -> Grid<f32> {
    let _scope = info_span!("shape").entered();

    let mut height_map = Grid::new(island.size(), 0.0);

    for cell in height_map.cells() {
        let dist = if island[cell] {
            edt[cell]
        } else {
            -inv_edt[cell]
        };

        let k = -1.0 - 0.5;
        let x = (dist / (2.0 * settings.beach_size) + 0.5).clamp(0.0, 1.0);
        let alpha = (1.0 - 1.0 / (1.0 + (1.0 / x - 1.0).powf(k))).clamp(0.0, 1.0);

        let mut height = settings.land_height * alpha - settings.ocean_depth * (1.0 - alpha);
        height += dist.max(0.0).powi(3);
        height_map[cell] = height;
    }

    height_map
}
