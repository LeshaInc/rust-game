mod fbm;
mod simplex;

use bevy::prelude::*;

pub use self::fbm::{FbmNoise, FbmNoiseSettings};
pub use self::simplex::SimplexNoise;

pub trait Noise<const N: usize> {
    fn get(&self, pos: Vec2) -> [f32; N];
}
