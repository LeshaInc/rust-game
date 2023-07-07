use std::f32::consts::PI;

use rg_core::Grid;
use serde::Deserialize;

use crate::{WorldgenProgress, WorldgenStage};

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct ElevationSettings {
    pub beach_size: f32,
    pub inland_height: f32,
}

pub fn compute_elevation(
    island: &Grid<bool>,
    settings: &ElevationSettings,
    progress: &WorldgenProgress,
) -> Grid<f32> {
    progress.set(WorldgenStage::Elevation, 0);

    let mut elevation = island
        .to_f32()
        .resize(island.size() / 4)
        .to_bool(0.5)
        .compute_edt()
        .resize(island.size());

    progress.set(WorldgenStage::Elevation, 20);

    reshape(&mut elevation, island, settings);
    progress.set(WorldgenStage::Elevation, 40);

    elevation.blur(3);
    progress.set(WorldgenStage::Elevation, 60);

    elevation.blur(3);
    progress.set(WorldgenStage::Elevation, 100);

    elevation
}

fn reshape(elevation: &mut Grid<f32>, island: &Grid<bool>, settings: &ElevationSettings) {
    for (cell, height) in elevation.entries_mut() {
        if !island[cell] {
            *height = 0.0;
        }

        *height = if *height < settings.beach_size {
            (0.5 - 0.5 * (*height * PI / settings.beach_size).cos()) * settings.inland_height
        } else {
            height.powi(4) + settings.inland_height
        };
    }
}
