use bevy::prelude::*;
use rg_core::Grid;
use serde::Deserialize;

use crate::{WorldgenProgress, WorldgenStage};

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct ElevationSettings {
    pub beach_size: f32,
    pub land_height: f32,
    pub ocean_depth: f32,
}

pub fn compute_elevation(
    island: &Grid<bool>,
    settings: &ElevationSettings,
    progress: &WorldgenProgress,
) -> Grid<f32> {
    let _scope = info_span!("compute_elevation").entered();

    progress.set(WorldgenStage::Elevation, 0);

    let edt = compute_edt(island);
    progress.set(WorldgenStage::Elevation, 25);

    let inv_edt = compute_inv_edt(island);
    progress.set(WorldgenStage::Elevation, 50);

    let mut elevation = shape(island, &edt, &inv_edt, settings);
    progress.set(WorldgenStage::Elevation, 75);

    elevation.blur(2);
    progress.set(WorldgenStage::Elevation, 90);

    elevation.blur(2);
    progress.set(WorldgenStage::Elevation, 100);

    elevation
}

fn compute_edt(island: &Grid<bool>) -> Grid<f32> {
    let _scope = info_span!("compute_edt").entered();
    let bitmap = island.to_f32().resize(island.size() / 4).to_bool(0.5);
    let mut edt = bitmap.compute_edt_exact().resize(island.size());
    edt.remap_inplace(0.0, 1.0);
    edt
}

fn compute_inv_edt(island: &Grid<bool>) -> Grid<f32> {
    let _scope = info_span!("compute_inv_edt").entered();

    let island_f32 = island.to_f32();

    let mut grid = Grid::new(island.size() / 4 + 128, 1.0).with_origin(-IVec2::splat(64));
    for cell in grid.cells() {
        grid[cell] = 1.0 - island_f32.sample(cell.as_vec2() * 4.0);
    }

    let edt = grid.to_bool(0.5).compute_edt_exact();
    let mut res = Grid::new(island.size(), 0.0);

    for cell in res.cells() {
        res[cell] = edt.sample(cell.as_vec2() / 4.0);
    }

    res.remap_inplace(0.0, 1.0);

    res
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
