use std::f32::consts::PI;

use rg_core::Grid;
use serde::Deserialize;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct ElevationSettings {
    pub beach_size: f32,
    pub inland_height: f32,
}

pub fn compute_elevation(island: &Grid<bool>, settings: &ElevationSettings) -> Grid<f32> {
    let mut elevation = island
        .to_f32()
        .resize(island.size() / 4)
        .to_bool(0.5)
        .compute_edt()
        .resize(island.size());

    reshape(&mut elevation, island, settings);
    elevation.blur(3);
    elevation.blur(3);

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
