use std::f32::consts::PI;
use std::time::Duration;

use bevy::ecs::system::SystemState;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::transform::TransformSystem;
use bevy_xpbd_3d::prelude::*;
use rg_camera_controller::CameraController;
use rg_core::{CollisionLayer, PrevTransform};
use rg_pixel_material::{GlobalFogHeight, PixelMaterial, ReplaceStandardMaterial};
use rg_terrain::ChunkSpawnCenter;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_camera,
                update_fog_height,
                spawn_character,
                control_character,
                find_animation_player,
                update_chunk_spawning_center,
            ),
        )
        .add_systems(
            PostUpdate,
            (update_rotation, update_models.after(update_rotation))
                .after(PhysicsSet::Sync)
                .before(TransformSystem::TransformPropagate),
        );
    }

    fn finish(&self, app: &mut App) {
        app.init_resource::<CharacterPrototype>();
    }
}

#[derive(Resource)]
struct CharacterPrototype {
    scene: Handle<Scene>,
    material: Handle<PixelMaterial>,
    idle_animation: Handle<AnimationClip>,
    running_animation: Handle<AnimationClip>,
}

impl FromWorld for CharacterPrototype {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(Res<AssetServer>, ResMut<Assets<PixelMaterial>>)> =
            SystemState::new(world);

        let (asset_server, mut materials) = system_state.get_mut(world);

        CharacterPrototype {
            scene: asset_server.load("character.glb#Scene0"),
            material: materials.add(PixelMaterial {
                dither_enabled: false,
                ..default()
            }),
            idle_animation: asset_server.load("character.glb#Animation0"),
            running_animation: asset_server.load("character.glb#Animation1"),
        }
    }
}

#[derive(Component)]
pub struct SpawnCharacter;

#[derive(Component)]
pub struct ControlledCharacter;

#[derive(Component)]
pub struct CharacterModel(pub Entity);

#[derive(Component)]
pub struct CharacterAnimationPlayer(pub Entity);

fn spawn_character(
    q_character: Query<(Entity, &Transform), With<SpawnCharacter>>,
    mut commands: Commands,
    prototype: Res<CharacterPrototype>,
) {
    let height = 1.8;
    let radius = 0.3;

    for (character, &transform) in &q_character {
        commands
            .entity(character)
            .remove::<SpawnCharacter>()
            .insert((
                Name::new("Character"),
                transform,
                PrevTransform(transform),
                GlobalTransform::default(),
                RigidBody::Kinematic,
                Collider::capsule_endpoints(radius * Vec3::Z, (height - radius) * Vec3::Z, radius),
                CollisionLayers::new([CollisionLayer::Character], [CollisionLayer::Static]),
                Position(transform.translation),
                ControlledCharacter,
                Visibility::Visible,
                ComputedVisibility::default(),
            ));

        commands
            .spawn((
                Name::new("Character Model"),
                CharacterModel(character),
                transform,
                GlobalTransform::default(),
                VisibilityBundle::default(),
            ))
            .with_children(|commands| {
                commands.spawn((
                    SceneBundle {
                        scene: prototype.scene.clone(),
                        ..default()
                    },
                    ReplaceStandardMaterial(prototype.material.clone()),
                ));
            });
    }
}

fn find_animation_player(
    q_model: Query<Entity, (With<CharacterModel>, Without<CharacterAnimationPlayer>)>,
    q_has_animation_player: Query<(), With<AnimationPlayer>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    for entity in q_model.iter() {
        for descendant in q_children.iter_descendants(entity) {
            if q_has_animation_player.contains(descendant) {
                commands
                    .entity(entity)
                    .insert(CharacterAnimationPlayer(descendant));
            }
        }
    }
}

fn control_character(
    mut q_character: Query<&mut Position, With<ControlledCharacter>>,
    q_camera: Query<&CameraController>,
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let Ok(mut position) = q_character.get_single_mut() else {
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
    position.0 += 4.0 * dir * time.delta_seconds();
}

fn update_rotation(mut q_agents: Query<(&mut Transform, &PrevTransform), Without<CharacterModel>>) {
    for (mut transform, prev_transform) in q_agents.iter_mut() {
        let velocity = (transform.translation - prev_transform.translation).xy();
        if velocity.abs_diff_eq(Vec2::ZERO, 1e-3) {
            continue;
        }

        let angle = velocity.y.atan2(velocity.x) + 0.5 * PI;
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

fn update_models(
    q_agents: Query<(&Transform, &PrevTransform), Without<CharacterModel>>,
    mut q_models: Query<(&CharacterModel, &mut Transform, &CharacterAnimationPlayer)>,
    mut q_animation_player: Query<&mut AnimationPlayer>,
    time: Res<Time>,
    prototype: Res<CharacterPrototype>,
) {
    for (model, mut model_transform, animation_player) in q_models.iter_mut() {
        let agent = model.0;
        let Ok((agent_transform, agent_prev_transform)) = q_agents.get(agent) else {
            continue;
        };

        let Ok(mut animation_player) = q_animation_player.get_mut(animation_player.0) else {
            continue;
        };

        let agent_dir = agent_transform.translation - agent_prev_transform.translation;
        let agent_velocity = agent_dir.xy().length() / time.delta_seconds();

        if agent_velocity < 0.1 {
            animation_player
                .play_with_transition(prototype.idle_animation.clone(), Duration::from_millis(200))
                .repeat();
        } else {
            animation_player
                .play_with_transition(
                    prototype.running_animation.clone(),
                    Duration::from_millis(200),
                )
                .set_speed(2.0)
                .repeat();
        }

        let alpha = 1.0 - 0.0001f32.powf(time.delta_seconds());
        model_transform.translation = model_transform
            .translation
            .lerp(agent_transform.translation, alpha);

        let alpha = 1.0 - 0.001f32.powf(time.delta_seconds());
        model_transform.rotation = model_transform
            .rotation
            .slerp(agent_transform.rotation, alpha);
    }
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
