mod deserialized_resource;
mod grid;
mod layers;
mod noise;
mod poisson_disc;
mod prev_transform;
pub mod progress;
mod scale;
mod vec_utils;

use bevy::prelude::*;

pub use crate::deserialized_resource::{DeserializedResource, DeserializedResourcePlugin};
pub use crate::grid::{EdtSettings, Grid};
pub use crate::layers::CollisionLayers;
pub use crate::noise::{FbmNoise, FbmNoiseSettings, Noise, SimplexNoise};
pub use crate::poisson_disc::PoissonDiscSampling;
pub use crate::prev_transform::{PrevTransform, PrevTransformPlugin};
pub use crate::scale::{GameScale, GameScaleSetting, ScalePlugin, UiScale, UiScaleSetting};
pub use crate::vec_utils::VecToBits;

pub const NEIGHBORHOOD_4: [IVec2; 4] = [
    IVec2::new(0, -1),
    IVec2::new(1, 0),
    IVec2::new(0, 1),
    IVec2::new(-1, 0),
];

pub const NEIGHBORHOOD_8: [IVec2; 8] = [
    IVec2::new(0, -1),
    IVec2::new(1, -1),
    IVec2::new(1, 0),
    IVec2::new(1, 1),
    IVec2::new(0, 1),
    IVec2::new(-1, 1),
    IVec2::new(-1, 0),
    IVec2::new(-1, -1),
];
