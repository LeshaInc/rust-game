use bevy::prelude::*;
use rg_core::progress::ProgressWriter;
use rg_core::Grid;

use crate::WorldgenStage;

pub fn generate_shore_map(
    progress: &mut ProgressWriter<WorldgenStage>,
    island_map: &Grid<f32>,
    river_map: &Grid<f32>,
) -> Grid<f32> {
    let _scope = info_span!("generate_shore_map").entered();

    let mut shore_map = river_map.clone();

    progress.task(|| {
        for (shore, &dist) in shore_map.values_mut().zip(island_map.values()) {
            *shore = shore.max(1.0 - (dist / 3.0).max(0.0).min(1.0));
        }
    });

    progress.task(|| shore_map.blur(3));
    progress.task(|| shore_map.blur(3));

    progress.task(|| {
        for shore in shore_map.values_mut() {
            *shore = (*shore / 0.1).min(1.0);
        }
    });

    progress.task(|| shore_map.blur(2));
    progress.task(|| shore_map.blur(2));

    shore_map
}
