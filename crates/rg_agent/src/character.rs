use bevy::prelude::*;
use bevy_rapier3d::prelude::{
    CharacterAutostep, CharacterLength, Collider, CollisionGroups, KinematicCharacterController,
    KinematicCharacterControllerOutput, RigidBody,
};
use rg_core::CollisionLayers;
use rg_pixel_material::PixelMaterial;

use crate::MovementInput;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (spawn_character, control_character));
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

        commands.entity(entity).remove::<SpawnCharacter>().insert((
            RigidBody::KinematicPositionBased,
            MaterialMeshBundle {
                transform,
                mesh,
                material,
                ..default()
            },
            Collider::capsule_y(0.9, 0.3),
            KinematicCharacterController {
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
        ));
    }
}

fn control_character(
    mut q_character: Query<&mut MovementInput, With<ControlledCharacter>>,
    input: Res<Input<KeyCode>>,
) {
    let mut dir = Vec3::ZERO;

    if input.pressed(KeyCode::A) {
        dir.x -= 1.0;
    }
    if input.pressed(KeyCode::D) {
        dir.x += 1.0;
    }
    if input.pressed(KeyCode::W) {
        dir.z -= 1.0;
    }
    if input.pressed(KeyCode::S) {
        dir.z += 1.0;
    }

    dir = dir.normalize_or_zero();

    for mut movement in &mut q_character {
        movement.direction = dir;
    }
}
