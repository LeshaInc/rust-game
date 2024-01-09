#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::ecs::query::Has;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::mesh::MeshVertexBufferLayout;
use bevy::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
};
use bevy::scene::SceneInstance;

pub struct PixelMaterialPlugin;

impl Plugin for PixelMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<PixelMaterial>::default())
            .init_resource::<GlobalDitherOffset>()
            .init_resource::<GlobalFogHeight>()
            .init_resource::<PixelMaterialShaders>()
            .add_systems(
                PostUpdate,
                (replace_standard_material::<PixelMaterial>, update_globals),
            );
    }
}

#[derive(Debug, Clone, Component, AsBindGroup, TypeUuid, TypePath, Asset)]
#[uuid = "7e80d778-3cb8-4ec2-95bf-ceb03ce277e0"]
#[bind_group_data(PixelMaterialKey)]
pub struct PixelMaterial {
    #[uniform(0)]
    pub color: Color,
    #[uniform(0)]
    pub bands: u32,
    pub dither_enabled: bool,
    // TODO: shader globals
    #[uniform(0)]
    pub dither_offset: UVec2,
    #[uniform(0)]
    pub fog_height: f32,
}

impl Default for PixelMaterial {
    fn default() -> Self {
        PixelMaterial {
            color: Color::WHITE,
            bands: 4,
            dither_enabled: true,
            dither_offset: UVec2::ZERO,
            fog_height: 0.0,
        }
    }
}

impl Material for PixelMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/pixel.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Back);

        if key.bind_group_data.dither_enabled {
            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment.shader_defs.push("DITHER_ENABLED".into());
            }
        }

        Ok(())
    }
}

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct PixelMaterialKey {
    dither_enabled: bool,
}

impl From<&PixelMaterial> for PixelMaterialKey {
    fn from(material: &PixelMaterial) -> Self {
        Self {
            dither_enabled: material.dither_enabled,
        }
    }
}

#[derive(Debug, Default, Resource)]
pub struct GlobalDitherOffset(pub UVec2);

#[derive(Debug, Default, Resource)]
pub struct GlobalFogHeight(pub f32);

fn update_globals(
    mut materials: ResMut<Assets<PixelMaterial>>,
    dither_offset: Res<GlobalDitherOffset>,
    fog_height: Res<GlobalFogHeight>,
) {
    for (_, material) in materials.iter_mut() {
        material.dither_offset = dither_offset.0;
        material.fog_height = fog_height.0;
    }
}

#[derive(Debug, Resource)]
pub struct PixelMaterialShaders {
    pub pixel_funcs: Handle<Shader>,
}

impl FromWorld for PixelMaterialShaders {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self {
            pixel_funcs: asset_server.load("shaders/pixel_funcs.wgsl"),
        }
    }
}

#[derive(Component)]
pub struct ReplaceStandardMaterial<T: Material>(pub Handle<T>);

fn replace_standard_material<T: Material>(
    q_entities: Query<(
        Entity,
        &ReplaceStandardMaterial<T>,
        Has<Handle<StandardMaterial>>,
        Has<Handle<Scene>>,
        Has<SceneInstance>,
    )>,
    q_replaceable: Query<(), With<Handle<StandardMaterial>>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    for (entity, new_material, has_old_material, is_scene, has_scene_instance) in q_entities.iter()
    {
        if is_scene && !has_scene_instance {
            continue;
        }

        if has_old_material {
            commands
                .entity(entity)
                .remove::<Handle<StandardMaterial>>()
                .remove::<ReplaceStandardMaterial<T>>()
                .insert(new_material.0.clone());
        } else {
            commands
                .entity(entity)
                .remove::<ReplaceStandardMaterial<T>>();
        }

        for descendant in q_children.iter_descendants(entity) {
            if q_replaceable.contains(descendant) {
                commands
                    .entity(descendant)
                    .remove::<Handle<StandardMaterial>>()
                    .insert(new_material.0.clone());
            }
        }
    }
}
