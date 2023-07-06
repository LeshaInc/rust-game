use bevy::asset::AssetPath;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::render_resource::AsBindGroup;
use rg_billboard::{BillboardMaterial, BillboardMaterialPlugin};

pub struct GrassMaterialPlugin;

impl Plugin for GrassMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BillboardMaterialPlugin::<GrassMaterial>::default());
    }

    fn finish(&self, app: &mut App) {
        app.init_resource::<DefaultGrassMaterial>();
    }
}

#[derive(Debug, Default, Clone, Component, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "d36218ae-d090-4ef1-a660-a4579db53935"]
pub struct GrassMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
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
        });

        Self(material)
    }
}
