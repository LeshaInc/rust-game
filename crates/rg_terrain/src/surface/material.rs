use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use rg_pixel_material::{GlobalDitherOffset, GlobalFogHeight};

pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(Update, update_globals);
    }

    fn finish(&self, app: &mut App) {
        app.init_resource::<DefaultTerrainMaterial>();
    }
}

#[derive(Debug, Clone, Component, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "cc76913b-20ee-45b2-8a71-d89ca79ec8a1"]
#[bind_group_data(TerrainMaterialKey)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub dither_offset: UVec2,
    #[uniform(0)]
    pub fog_height: f32,
    #[texture(1)]
    #[sampler(2)]
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
            texture: asset_server.load("images/tiles/terrain.png"),
            dither_offset: UVec2::ZERO,
            fog_height: 0.0,
        });

        Self(material)
    }
}

fn update_globals(
    mut materials: ResMut<Assets<TerrainMaterial>>,
    dither_offset: Res<GlobalDitherOffset>,
    fog_height: Res<GlobalFogHeight>,
) {
    for (_, material) in materials.iter_mut() {
        material.dither_offset = dither_offset.0;
        material.fog_height = fog_height.0;
    }
}
