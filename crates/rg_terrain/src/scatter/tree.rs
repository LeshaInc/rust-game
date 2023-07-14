use std::f32::consts::TAU;

use bevy::asset::AssetPath;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::render_resource::AsBindGroup;
use bevy_rapier3d::prelude::Collider;
use rand::Rng;
use rg_billboard::{BillboardMaterial, BillboardMaterialPlugin, ScatterMultiBillboard};
use rg_pixel_material::{GlobalDitherOffset, GlobalFogHeight, PixelMaterial};
use rg_worldgen::{WorldMaps, WORLD_SCALE};

use super::ScatterPrototype;

#[derive(Resource)]
pub struct TreePrototype {
    trunk_mesh: Handle<Mesh>,
    crown_mesh: Handle<Mesh>,
    pixel_material: Handle<PixelMaterial>,
    leaves_material: Handle<LeavesMaterial>,
}

impl ScatterPrototype for TreePrototype {
    fn build_app(app: &mut App) {
        app.add_plugins(BillboardMaterialPlugin::<LeavesMaterial>::default())
            .add_systems(PostUpdate, update_globals);
    }

    fn poisson_disc_min_radius(&self) -> f32 {
        4.0
    }

    fn density(&self, world_maps: &WorldMaps, pos: Vec2) -> f32 {
        let elevation = world_maps.elevation.sample(pos / WORLD_SCALE);
        if elevation <= 0.0 {
            return 0.0;
        }
        1.0
    }

    fn spawn<R: Rng>(&self, rng: &mut R, commands: &mut Commands, mut pos: Vec3) -> Entity {
        pos.z -= rng.gen_range(0.00..=0.2);

        let angle = rng.gen_range(0.0..TAU);
        let rotation = Quat::from_rotation_z(angle);
        let scale = rng.gen_range(0.7..=1.0);

        let transform = Transform {
            translation: pos,
            rotation,
            scale: Vec3::splat(scale),
        };

        commands
            .spawn(MaterialMeshBundle {
                transform,
                mesh: self.trunk_mesh.clone(),
                material: self.pixel_material.clone(),
                ..default()
            })
            .with_children(|commands| {
                // crown
                commands.spawn(MaterialMeshBundle {
                    mesh: self.crown_mesh.clone(),
                    material: self.pixel_material.clone(),
                    ..default()
                });

                // leaves
                commands.spawn((
                    TransformBundle::default(),
                    VisibilityBundle::default(),
                    self.leaves_material.clone(),
                    ScatterMultiBillboard {
                        seed: 0,
                        count: 2048,
                        move_along_normal: 0.1..0.2,
                        instance_size: Vec2::new(12.0 / 48.0, 10.0 / 48.0),
                        instance_color: Vec3::new(1.0, 1.0, 1.0),
                        anchor: Vec2::new(0.5, 0.5),
                        mesh: self.crown_mesh.clone(),
                    },
                ));

                // trunk collider
                commands.spawn((
                    TransformBundle::from(Transform::from_xyz(0.0, 0.0, 1.28)),
                    Collider::capsule_z(1.28, 0.32),
                ));

                // crown collider
                commands.spawn((
                    TransformBundle::from(Transform::from_xyz(0.0, 0.0, 3.5)),
                    Collider::ball(1.0),
                ));
            })
            .id()
    }
}

impl FromWorld for TreePrototype {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<AssetServer>,
            ResMut<Assets<PixelMaterial>>,
            ResMut<Assets<LeavesMaterial>>,
        )> = SystemState::new(world);
        let (asset_server, mut pixel_materials, mut leaves_materials) = system_state.get_mut(world);

        let trunk_mesh = asset_server.load("tree.glb#Mesh0/Primitive0");
        let crown_mesh = asset_server.load("tree.glb#Mesh1/Primitive0");

        let pixel_material = pixel_materials.add(PixelMaterial {
            bands: 4,
            ..default()
        });

        let leaves_material = leaves_materials.add(LeavesMaterial {
            texture: asset_server.load("images/leaf.png"),
            dither_offset: UVec2::ZERO,
            fog_height: 0.0,
        });

        Self {
            trunk_mesh,
            crown_mesh,
            pixel_material,
            leaves_material,
        }
    }
}

#[derive(Debug, Default, Clone, Component, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "8b047c11-1b1d-4590-b5e8-e33f50c61b37"]
pub struct LeavesMaterial {
    #[uniform(0)]
    pub dither_offset: UVec2,
    #[uniform(0)]
    pub fog_height: f32,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}

impl BillboardMaterial for LeavesMaterial {
    fn vertex_shader() -> AssetPath<'static> {
        "shaders/leaves.wgsl".into()
    }

    fn fragment_shader() -> AssetPath<'static> {
        "shaders/leaves.wgsl".into()
    }
}

fn update_globals(
    mut materials: ResMut<Assets<LeavesMaterial>>,
    dither_offset: Res<GlobalDitherOffset>,
    fog_height: Res<GlobalFogHeight>,
) {
    for (_, material) in materials.iter_mut() {
        material.dither_offset = dither_offset.0;
        material.fog_height = fog_height.0;
    }
}
