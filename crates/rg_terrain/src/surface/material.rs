use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderRef, TextureDimension, TextureFormat,
};
use rg_pixel_material::{GlobalDitherOffset, GlobalFogHeight};

use crate::{Chunk, SharedChunkMaps};

pub struct SurfaceMaterialsPlugin;

impl Plugin for SurfaceMaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .add_systems(PostUpdate, (update_tile_maps, update_globals));
    }

    fn finish(&self, app: &mut App) {
        app.init_resource::<SurfaceMaterials>();
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
    #[texture(3, sample_type = "u_int")]
    pub tile_map: Handle<Image>,
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

#[derive(Debug, Clone, Component, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "435932e3-ff46-47eb-83c3-818ad8f3fb81"]
#[bind_group_data(WaterMaterialKey)]
pub struct WaterMaterial {
    #[uniform(0)]
    pub fog_height: f32,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct WaterMaterialKey {}

impl From<&WaterMaterial> for WaterMaterialKey {
    fn from(_material: &WaterMaterial) -> Self {
        Self {}
    }
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }
}

#[derive(Debug, Clone, Resource)]
pub struct SurfaceMaterials {
    pub terrain: Handle<TerrainMaterial>,
    pub water: Handle<WaterMaterial>,
}

impl FromWorld for SurfaceMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<AssetServer>,
            ResMut<Assets<TerrainMaterial>>,
            ResMut<Assets<WaterMaterial>>,
            ResMut<Assets<Image>>,
        )> = SystemState::new(world);

        let (asset_server, mut terrain_materials, mut water_materials, mut images) =
            system_state.get_mut(world);

        let tile_map = images.add(Image::new_fill(
            Extent3d::default(),
            TextureDimension::D2,
            &[0],
            TextureFormat::R8Uint,
        ));

        Self {
            terrain: terrain_materials.add(TerrainMaterial {
                dither_offset: UVec2::ZERO,
                fog_height: 0.0,
                texture: asset_server.load("images/tiles/terrain.png"),
                tile_map,
            }),
            water: water_materials.add(WaterMaterial { fog_height: 0.0 }),
        }
    }
}

fn update_globals(
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    dither_offset: Res<GlobalDitherOffset>,
    fog_height: Res<GlobalFogHeight>,
) {
    for (_, material) in terrain_materials.iter_mut() {
        material.dither_offset = dither_offset.0;
        material.fog_height = fog_height.0;
    }

    for (_, material) in water_materials.iter_mut() {
        material.fog_height = fog_height.0;
    }
}

fn update_tile_maps(
    mut q_chunks: Query<
        (&mut Handle<TerrainMaterial>, &SharedChunkMaps),
        (
            With<Chunk>,
            Or<(Changed<SharedChunkMaps>, Added<Handle<TerrainMaterial>>)>,
        ),
    >,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (mut material, chunk_maps) in q_chunks.iter_mut() {
        let tile_map = images.add(Image::new(
            Extent3d {
                width: chunk_maps.tile_map.size().x,
                height: chunk_maps.tile_map.size().y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            chunk_maps.tile_map.values().map(|&v| v as u8).collect(),
            TextureFormat::R8Uint,
        ));

        let old_material = materials.get(&material).unwrap().clone();
        *material = materials.add(TerrainMaterial {
            tile_map,
            ..old_material
        });
    }
}
