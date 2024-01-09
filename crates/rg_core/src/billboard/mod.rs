#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod instance;
mod material;
mod scatter;

use bevy::prelude::*;
use bevy::render::extract_component::UniformComponentPlugin;
use bevy::render::render_asset::RenderAssetPlugin;
use bevy::render::view::VisibilitySystems;
use bevy::render::RenderApp;

use self::instance::{
    compute_multi_billboard_bounds, extract_multi_billboards, MultiBillboardUniform,
};
pub use self::instance::{BillboardInstance, MultiBillboard};
pub use self::material::{BillboardMaterial, BillboardMaterialKey, BillboardMaterialPlugin};
pub use self::scatter::{ScatterMultiBillboard, ScatterPlugin};

pub struct BillboardPlugin;

impl Plugin for BillboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScatterPlugin)
            .init_asset::<MultiBillboard>()
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
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}
