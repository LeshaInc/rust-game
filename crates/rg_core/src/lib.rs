mod array_texture;
mod deserialized_resource;
mod layers;
mod poisson_disc;
mod prev_transform;
pub mod progress;
mod scale;
mod vec_utils;

use bevy::prelude::*;
pub use rg_grid::{EdtSettings, Grid, NEIGHBORHOOD_4, NEIGHBORHOOD_8};
pub use rg_noise::{FbmNoise, FbmNoiseSettings, Noise, SimplexNoise};

pub use crate::array_texture::{ArrayTexturePlugin, BuildArrayTexture};
pub use crate::deserialized_resource::{DeserializedResource, DeserializedResourcePlugin};
pub use crate::layers::CollisionLayers;
pub use crate::poisson_disc::PoissonDiscSampling;
pub use crate::prev_transform::{PrevTransform, PrevTransformPlugin};
pub use crate::scale::{GameScale, GameScaleSetting, ScalePlugin, UiScale, UiScaleSetting};
pub use crate::vec_utils::VecToBits;

pub trait FloatGridExt {
    fn add_noise<N: Noise<1> + Sync>(&mut self, noise: &N);
}

impl FloatGridExt for Grid<f32> {
    fn add_noise<N: Noise<1> + Sync>(&mut self, noise: &N) {
        let _scope = info_span!("add_noise").entered();

        self.par_map_inplace(|cell, value| {
            *value += noise.get(cell.as_vec2())[0];
        });
    }
}
