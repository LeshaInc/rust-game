use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{
    CharacterAutostep, CharacterLength, Collider, CollisionGroups, KinematicCharacterController,
    KinematicCharacterControllerOutput, RigidBody,
};
use rg_camera_controller::CameraController;
use rg_core::CollisionLayers;
use rg_pixel_material::{GlobalFogHeight, PixelMaterial};
use rg_terrain::ChunkSpawnCenter;

use crate::MovementInput;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                spawn_character,
                control_character,
                update_chunk_spawning_center,
            ),
        );

        app.add_systems(Update, (update_camera, update_fog_height));
    }
}

#[derive(Component)]
pub struct SpawnCharacter;

#[derive(Component)]
pub struct ControlledCharacter;

fn spawn_character(
    q_character: Query<(Entity, &Transform), With<SpawnCharacter>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<PixelMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, &transform) in &q_character {
        let material = materials.add(PixelMaterial { ..default() });
        let mesh = meshes.add(
            shape::Capsule {
                radius: 0.3,
                depth: 1.6,
                ..default()
            }
            .into(),
        );

        commands
            .entity(entity)
            .remove::<SpawnCharacter>()
            .insert((
                transform,
                GlobalTransform::default(),
                RigidBody::KinematicPositionBased,
                Collider::capsule_z(0.9, 0.3),
                KinematicCharacterController {
                    up: Vec3::Z,
                    autostep: Some(CharacterAutostep {
                        max_height: CharacterLength::Absolute(0.5),
                        min_width: CharacterLength::Absolute(0.1),
                        include_dynamic_bodies: false,
                    }),
                    snap_to_ground: Some(CharacterLength::Absolute(0.1)),
                    offset: CharacterLength::Absolute(0.01),
                    ..default()
                },
                CollisionGroups::new(
                    CollisionLayers::CHARACTER.into(),
                    (CollisionLayers::STATIC_GEOMETRY | CollisionLayers::DYNAMIC_GEOMETRY).into(),
                ),
                KinematicCharacterControllerOutput::default(),
                MovementInput::default(),
                ControlledCharacter,
                Visibility::Visible,
                ComputedVisibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn(MaterialMeshBundle {
                    transform: Transform::from_rotation(Quat::from_rotation_x(-90f32.to_radians())),
                    mesh,
                    material,
                    ..default()
                });
            });
    }
}

fn control_character(
    mut q_character: Query<&mut MovementInput, With<ControlledCharacter>>,
    q_camera: Query<&CameraController>,
    input: Res<Input<KeyCode>>,
) {
    let Ok(mut movement) = q_character.get_single_mut() else {
        return;
    };

    let Ok(camera) = q_camera.get_single() else {
        return;
    };

    let mut dir = Vec3::ZERO;

    if input.pressed(KeyCode::A) {
        dir.x -= 1.0;
    }
    if input.pressed(KeyCode::D) {
        dir.x += 1.0;
    }
    if input.pressed(KeyCode::W) {
        dir.y += 1.0;
    }
    if input.pressed(KeyCode::S) {
        dir.y -= 1.0;
    }

    dir = camera.rotation * dir.normalize_or_zero();
    movement.direction = dir;
}

fn update_chunk_spawning_center(
    q_character: Query<&Transform, With<ControlledCharacter>>,
    mut center: ResMut<ChunkSpawnCenter>,
) {
    let Ok(transform) = q_character.get_single() else {
        return;
    };

    center.0 = transform.translation.xy();
}

fn update_camera(
    q_character: Query<&Transform, With<ControlledCharacter>>,
    mut q_camera: Query<&mut CameraController>,
) {
    let Ok(character_transform) = q_character.get_single() else {
        return;
    };

    let Ok(mut camera) = q_camera.get_single_mut() else {
        return;
    };

    camera.target_translation = character_transform.translation;
}

fn update_fog_height(
    q_character: Query<&Transform, With<ControlledCharacter>>,
    mut fog_height: ResMut<GlobalFogHeight>,
) {
    let Ok(character_transform) = q_character.get_single() else {
        return;
    };

    fog_height.0 = character_transform.translation.z;
}
