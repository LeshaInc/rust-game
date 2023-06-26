#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod instance;
mod material;

use bevy::prelude::*;
use bevy::render::extract_component::UniformComponentPlugin;
use bevy::render::render_asset::RenderAssetPlugin;
use bevy::render::view::VisibilitySystems;
use bevy::render::RenderApp;
use material::DummyMesh;

use crate::instance::{
    compute_multi_billboard_bounds, extract_multi_billboards, MultiBillboardUniform,
};
pub use crate::instance::{BillboardInstance, MultiBillboard};
pub use crate::material::{BillboardMaterial, BillboardMaterialKey, BillboardMaterialPlugin};

pub struct BillboardPlugin;

impl Plugin for BillboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<MultiBillboard>()
            .init_resource::<DummyMesh>()
            .add_plugins(RenderAssetPlugin::<MultiBillboard>::default())
            .add_plugins(UniformComponentPlugin::<MultiBillboardUniform>::default())
            .add_systems(
                PostUpdate,
                compute_multi_billboard_bounds.in_set(VisibilitySystems::CalculateBounds),
            );

        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, extract_multi_billboards);
    }
}

#[derive(Default, Bundle)]
pub struct MultiBillboardBundle {
    pub multi_billboard: Handle<MultiBillboard>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}
