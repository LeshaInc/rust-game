use bevy::prelude::*;
use rg_core::Grid;

use crate::WorldgenProgress;

pub fn generate_shore_map(
    progress: &WorldgenProgress,
    island_map: &Grid<f32>,
    river_map: &Grid<f32>,
) -> Grid<f32> {
    let _scope = info_span!("generate_shore_map").entered();

    progress.set(crate::WorldgenStage::Shores, 0);

    let mut shore_map = river_map.clone();

    for (shore, &dist) in shore_map.values_mut().zip(island_map.values()) {
        *shore = shore.max(1.0 - (dist / 3.0).max(0.0).min(1.0));
    }

    progress.set(crate::WorldgenStage::Shores, 50);

    shore_map.blur(3);
    shore_map.blur(3);

    progress.set(crate::WorldgenStage::Shores, 75);

    for shore in shore_map.values_mut() {
        *shore = (*shore / 0.1).min(1.0);
    }

    shore_map.blur(2);
    shore_map.blur(2);

    progress.set(crate::WorldgenStage::Shores, 100);

    shore_map
}
