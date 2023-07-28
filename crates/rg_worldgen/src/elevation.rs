use bevy::prelude::*;
use rg_core::{EdtSettings, Grid};
use serde::Deserialize;

use crate::{WorldgenProgress, WorldgenStage};

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct ElevationSettings {
    pub beach_size: f32,
    pub land_height: f32,
    pub ocean_depth: f32,
}

pub fn compute_elevation(
    progress: &WorldgenProgress,
    settings: &ElevationSettings,
    island: &Grid<bool>,
) -> Grid<f32> {
    let _scope = info_span!("compute_elevation").entered();

    progress.set(WorldgenStage::Elevation, 0);

    let edt = island.compute_edt(EdtSettings {
        exact: true,
        invert: false,
        normalize: true,
        downsample: 4,
        padding: 0,
    });

    progress.set(WorldgenStage::Elevation, 25);

    let inv_edt = island.compute_edt(EdtSettings {
        exact: true,
        invert: true,
        normalize: true,
        downsample: 4,
        padding: 32,
    });

    progress.set(WorldgenStage::Elevation, 50);

    let mut elevation = shape(island, &edt, &inv_edt, settings);
    progress.set(WorldgenStage::Elevation, 75);

    elevation.blur(2);
    progress.set(WorldgenStage::Elevation, 90);

    elevation.blur(2);
    progress.set(WorldgenStage::Elevation, 100);

    elevation
}

fn shape(
    island: &Grid<bool>,
    edt: &Grid<f32>,
    inv_edt: &Grid<f32>,
    settings: &ElevationSettings,
) -> Grid<f32> {
    let _scope = info_span!("shape").entered();

    let mut elevation = Grid::new(island.size(), 0.0);

    for cell in elevation.cells() {
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
        elevation[cell] = height;
    }

    elevation
}
