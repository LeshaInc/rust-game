mod elevation;
mod island_shaping;

use bevy::prelude::UVec2;
use rand::SeedableRng;
use rand_pcg::Pcg32;

use crate::elevation::compute_elevation;
use crate::island_shaping::shape_island;

pub fn worldgen(seed: u64, size: UVec2) {
    let mut rng = Pcg32::seed_from_u64(seed);

    let island = shape_island(&mut rng, size);
    let elevation = compute_elevation(&island);

    elevation.debug_save(&format!("/tmp/world_{seed}.png"));
}
