use std::f32::consts::TAU;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;
use rand::Rng;
use rg_billboard::ScatterMultiBillboard;
use rg_pixel_material::PixelMaterial;
use rg_worldgen::{Biome, WorldMaps, WORLD_SCALE};

use super::tree::LeavesMaterial;
use super::ScatterPrototype;

#[derive(Resource)]
pub struct BushPrototype {
    bush_mesh: Handle<Mesh>,
    pixel_material: Handle<PixelMaterial>,
    leaves_material: Handle<LeavesMaterial>,
}

impl ScatterPrototype for BushPrototype {
    const SEED: u64 = 7408766663690913456;

    fn poisson_disc_min_radius(&self) -> f32 {
        2.0
    }

    fn density(&self, world_maps: &WorldMaps, pos: Vec2) -> f32 {
        let elevation = world_maps.elevation.sample(pos / WORLD_SCALE);
        if elevation <= 0.0 {
            return 0.0;
        }

        let biome = world_maps
            .biomes
            .get((pos / WORLD_SCALE).as_ivec2())
            .copied()
            .unwrap_or(Biome::Ocean);

        match biome {
            Biome::Ocean => 0.0,
            Biome::Forest => 0.05,
            Biome::Plains => 0.1,
        }
    }

    fn spawn<R: Rng>(&self, rng: &mut R, commands: &mut Commands, pos: Vec3) -> Entity {
        let angle = rng.gen_range(0.0..TAU);
        let rotation = Quat::from_rotation_z(angle);
        let scale = rng.gen_range(0.8..=1.0);

        let transform = Transform {
            translation: pos,
            rotation,
            scale: Vec3::splat(scale),
        };

        commands
            .spawn((
                Name::new("Bush"),
                MaterialMeshBundle {
                    transform,
                    mesh: self.bush_mesh.clone(),
                    material: self.pixel_material.clone(),
                    ..default()
                },
            ))
            .with_children(|commands| {
                // leaves
                commands.spawn((
                    TransformBundle::default(),
                    VisibilityBundle::default(),
                    self.leaves_material.clone(),
                    ScatterMultiBillboard {
                        seed: 0,
                        count: 512,
                        move_along_normal: 0.05..0.1,
                        instance_size: Vec2::new(12.0 / 48.0, 10.0 / 48.0),
                        instance_color: Vec3::new(1.0, 1.0, 1.0),
                        anchor: Vec2::new(0.5, 0.5),
                        mesh: self.bush_mesh.clone(),
                    },
                ));

                // crown collider
                commands.spawn((
                    TransformBundle::from(Transform::from_xyz(0.0, 0.0, 0.25)),
                    Collider::ball(0.3),
                ));
                fun_name();
            })
            .id()
    }
}

fn fun_name() {}

impl FromWorld for BushPrototype {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<AssetServer>,
            ResMut<Assets<PixelMaterial>>,
            ResMut<Assets<LeavesMaterial>>,
        )> = SystemState::new(world);
        let (asset_server, mut pixel_materials, mut leaves_materials) = system_state.get_mut(world);

        let bush_mesh = asset_server.load("bush.glb#Mesh0/Primitive0");

        let pixel_material = pixel_materials.add(PixelMaterial {
            bands: 4,
            ..default()
        });

        let leaves_material = leaves_materials.add(LeavesMaterial {
            texture: asset_server.load("images/leaf.png"),
            noise: asset_server.load("images/noise.png"),
            dither_offset: UVec2::ZERO,
            fog_height: 0.0,
        });

        Self {
            bush_mesh,
            pixel_material,
            leaves_material,
        }
    }
}