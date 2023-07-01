use bevy::prelude::UVec2;
use rand::SeedableRng;
use rand_pcg::Pcg32;

use crate::island_shaping::shape_island;

mod island_shaping;

pub fn worldgen(seed: u64, size: UVec2) {
    let mut rng = Pcg32::seed_from_u64(seed);
    let grid = shape_island(&mut rng, size);
    grid.debug_save(&format!("/tmp/world_{seed}.png"));
}
