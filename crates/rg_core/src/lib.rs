pub mod billboard;
pub mod chunk;
pub mod grid;
pub mod material;
pub mod noise;
pub mod progress;
pub mod scale;

mod array_texture;
mod camera;
mod deserialized_resource;
mod layers;
mod poisson_disc;
mod prev_transform;
mod vec_utils;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub use crate::array_texture::*;
pub use crate::camera::*;
pub use crate::deserialized_resource::*;
pub use crate::layers::*;
pub use crate::poisson_disc::*;
pub use crate::prev_transform::*;
pub use crate::vec_utils::*;

pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(self::billboard::BillboardPlugin)
            .add(self::chunk::ChunkPlugin)
            .add(self::material::PixelMaterialPlugin)
            .add(self::scale::ScalePlugin)
            .add(ArrayTexturePlugin)
            .add(PrevTransformPlugin)
            .add(CameraControllerPlugin)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, SystemSet)]
pub enum CoreSystems {
    UpdateOrigin,
}
