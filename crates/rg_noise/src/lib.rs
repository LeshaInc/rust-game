mod fbm;
mod simplex;

use bevy::prelude::*;

pub use crate::fbm::{FbmNoise, FbmNoiseSettings};
pub use crate::simplex::SimplexNoise;

pub trait Noise<const N: usize> {
    fn get(&self, pos: Vec2) -> [f32; N];
}
