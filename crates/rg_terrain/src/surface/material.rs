use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default());
    }

    fn finish(&self, app: &mut App) {
        app.init_resource::<DefaultTerrainMaterial>();
    }
}

#[derive(Debug, Clone, Component, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "cc76913b-20ee-45b2-8a71-d89ca79ec8a1"]
#[bind_group_data(TerrainMaterialKey)]
pub struct TerrainMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct TerrainMaterialKey {}

impl From<&TerrainMaterial> for TerrainMaterialKey {
    fn from(_material: &TerrainMaterial) -> Self {
        Self {}
    }
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
}

#[derive(Debug, Clone, Resource)]
pub struct DefaultTerrainMaterial(pub Handle<TerrainMaterial>);

impl FromWorld for DefaultTerrainMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(Res<AssetServer>, ResMut<Assets<TerrainMaterial>>)> =
            SystemState::new(world);

        let (asset_server, mut materials) = system_state.get_mut(world);

        let material = materials.add(TerrainMaterial {
            texture: asset_server.load("images/tiles/grass.png"),
        });

        Self(material)
    }
}
