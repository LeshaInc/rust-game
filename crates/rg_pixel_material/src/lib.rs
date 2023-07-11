use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::mesh::MeshVertexBufferLayout;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
};

pub struct PixelMaterialPlugin;

impl Plugin for PixelMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<PixelMaterial>::default())
            .init_resource::<GlobalDitherOffset>()
            .init_resource::<GlobalFogHeight>()
            .init_resource::<PixelMaterialShaders>()
            .add_systems(PostUpdate, update_globals);
    }
}

#[derive(Debug, Clone, Component, AsBindGroup, TypeUuid, TypePath)]
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
