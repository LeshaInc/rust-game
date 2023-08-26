use bevy::asset::AssetPath;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::render_resource::AsBindGroup;
use rg_core::billboard::{BillboardMaterial, BillboardMaterialPlugin};
use rg_core::material::{GlobalDitherOffset, GlobalFogHeight};

pub struct GrassMaterialPlugin;

impl Plugin for GrassMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BillboardMaterialPlugin::<GrassMaterial>::default())
            .add_systems(PostUpdate, update_globals);
    }

    fn finish(&self, app: &mut App) {
        app.init_resource::<DefaultGrassMaterial>();
    }
}

#[derive(Debug, Default, Clone, Component, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "d36218ae-d090-4ef1-a660-a4579db53935"]
pub struct GrassMaterial {
    #[uniform(0)]
    pub dither_offset: UVec2,
    #[uniform(0)]
    pub fog_height: f32,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub noise: Handle<Image>,
}

impl BillboardMaterial for GrassMaterial {
    fn vertex_shader() -> AssetPath<'static> {
        "shaders/grass.wgsl".into()
    }

    fn fragment_shader() -> AssetPath<'static> {
        "shaders/grass.wgsl".into()
    }
}

#[derive(Debug, Clone, Resource)]
pub struct DefaultGrassMaterial(pub Handle<GrassMaterial>);

impl FromWorld for DefaultGrassMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(Res<AssetServer>, ResMut<Assets<GrassMaterial>>)> =
            SystemState::new(world);

        let (asset_server, mut materials) = system_state.get_mut(world);

        let material = materials.add(GrassMaterial {
            texture: asset_server.load("images/grass.png"),
            noise: asset_server.load("images/noise.png"),
            dither_offset: UVec2::ZERO,
            fog_height: 0.0,
        });

        Self(material)
    }
}

fn update_globals(
    mut materials: ResMut<Assets<GrassMaterial>>,
    dither_offset: Res<GlobalDitherOffset>,
    fog_height: Res<GlobalFogHeight>,
) {
    for (_, material) in materials.iter_mut() {
        material.dither_offset = dither_offset.0;
        material.fog_height = fog_height.0;
    }
}
