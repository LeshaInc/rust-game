use std::fmt::Debug;

use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rg_core::CollisionLayers;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_movement_input);
    }
}

#[derive(Debug, Bundle)]
pub struct MovementBundle {
    pub movement_input: MovementInput,
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub locked_axes: LockedAxes,
    pub collision_groups: CollisionGroups,
    pub dominance: Dominance,
    pub velocity: Velocity,
    pub read_mass_properties: ReadMassProperties,
    pub external_impulse: ExternalImpulse,
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self {
            movement_input: MovementInput::default(),
            rigid_body: RigidBody::Dynamic,
            collider: Collider::default(),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            collision_groups: CollisionLayers::CHARACTER_GROUP,
            dominance: Dominance::group(100),
            velocity: Velocity::default(),
            read_mass_properties: ReadMassProperties::default(),
            external_impulse: ExternalImpulse::default(),
        }
    }
}

#[derive(Debug, Default, Component)]
pub struct MovementInput {
    pub direction: Vec2,
    pub jump: bool,
}

fn handle_movement_input(
    mut q_agents: Query<(
        Entity,
        &MovementInput,
        &Transform,
        &Velocity,
        &ReadMassProperties,
        &Collider,
        &mut ExternalImpulse,
    )>,
    time: Res<Time>,
    query: Res<RapierContext>,
) {
    for (entity, movement, transform, velocity, mass, collider, mut impulse) in &mut q_agents {
        let position = transform.translation;
        let velocity = velocity.linvel;
        let mass = mass.0.mass;

        let min_ground_distance = 0.05;
        let jump_velocity = 7.0;
        let jump_acceleration = 20.0;

        let ground_acceleration = 150.0;
        let max_ground_speed = 10.0;

        let air_acceleration = 50.0;
        let max_air_speed = 12.0;

        let distance_to_ground = query
            .cast_shape(
                position,
                Quat::IDENTITY,
                -Vec3::Z,
                collider,
                1.0,
                QueryFilter {
                    exclude_collider: Some(entity),
                    ..default()
                },
            )
            .map(|(_, toi)| toi.toi)
            .unwrap_or(f32::INFINITY);

        let is_grounded = distance_to_ground < min_ground_distance;

        let (acceleration, max_speed) = if is_grounded {
            (ground_acceleration, max_ground_speed)
        } else {
            (air_acceleration, max_air_speed)
        };

        if movement.jump {
            if is_grounded {
                impulse.impulse.z += mass * jump_velocity;
            } else {
                impulse.impulse.z += mass * jump_acceleration * time.delta_seconds();
            }
        }

        let mut next_velocity = velocity;

        let acc = (movement.direction - velocity.xy() / max_speed) * acceleration;
        next_velocity += (acc * time.delta_seconds()).extend(0.0);

        // let speed = next_velocity.xy().length().min(max_speed);
        // next_velocity = (next_velocity.normalize_or_zero().xy() * speed).extend(next_velocity.z);

        impulse.impulse += mass * (next_velocity - velocity);
    }
}
