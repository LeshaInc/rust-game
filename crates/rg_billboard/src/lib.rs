mod instance;
mod render;

use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::prelude::*;
use bevy::render::extract_component::UniformComponentPlugin;
use bevy::render::render_asset::RenderAssetPlugin;
use bevy::render::render_phase::AddRenderCommand;
use bevy::render::render_resource::SpecializedMeshPipelines;
use bevy::render::view::VisibilitySystems;
use bevy::render::{Render, RenderApp, RenderSet};
use instance::MultiBillboardUniform;
use render::{DummyMesh, MultiBillboardPipeline};

use crate::instance::{compute_multi_billboard_bounds, extract_multi_billboards};
pub use crate::instance::{BillboardInstance, MultiBillboard};
use crate::render::{queue_multi_billboard_bind_group, queue_multi_billboards, DrawMultiBillboard};

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
            .init_resource::<SpecializedMeshPipelines<MultiBillboardPipeline>>()
            .add_render_command::<Opaque3d, DrawMultiBillboard>()
            .add_systems(ExtractSchedule, extract_multi_billboards)
            .add_systems(
                Render,
                (
                    queue_multi_billboard_bind_group.in_set(RenderSet::Queue),
                    queue_multi_billboards.in_set(RenderSet::Queue),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<MultiBillboardPipeline>();
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
