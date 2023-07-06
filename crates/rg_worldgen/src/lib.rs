mod elevation;
mod island_shaping;
mod rivers;

use std::sync::Arc;

use bevy::prelude::{Resource, UVec2};
use rand::SeedableRng;
use rand_pcg::Pcg32;
use rg_core::Grid;

use crate::elevation::compute_elevation;
use crate::island_shaping::shape_island;
use crate::rivers::generate_rivers;

#[derive(Debug, Copy, Clone, Resource)]
pub struct WorldSeed(pub u64);

pub fn worldgen(seed: u64, size: UVec2) -> WorldMaps {
    let mut rng = Pcg32::seed_from_u64(seed);

    let island = shape_island(&mut rng, size);
    let mut elevation = compute_elevation(&island);
    let _rivers = generate_rivers(&mut rng, &mut elevation);

    elevation.debug_save(&format!("/tmp/world_{seed}.png"));

    WorldMaps {
        elevation: Arc::new(elevation),
    }
}

#[derive(Debug, Resource)]
pub struct WorldMaps {
    pub elevation: Arc<Grid<f32>>,
}
