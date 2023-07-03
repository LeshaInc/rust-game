mod elevation;
mod island_shaping;
mod rivers;

use bevy::prelude::UVec2;
use rand::SeedableRng;
use rand_pcg::Pcg32;

use crate::elevation::compute_elevation;
use crate::island_shaping::shape_island;
use crate::rivers::generate_rivers;

pub fn worldgen(seed: u64, size: UVec2) {
    let mut rng = Pcg32::seed_from_u64(seed);

    let island = shape_island(&mut rng, size);
    let elevation = compute_elevation(&island);
    let rivers = generate_rivers(&mut rng, &elevation);

    rivers.debug_save(&format!("/tmp/world_{seed}.png"));
}
