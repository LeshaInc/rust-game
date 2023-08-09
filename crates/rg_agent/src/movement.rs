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
    pub movement_state: MovementState,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
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
            movement_state: MovementState::default(),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            rigid_body: RigidBody::KinematicPositionBased,
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

#[derive(Debug, Default, Component)]
pub struct MovementState {
    pub velocity: Vec3,
}

fn handle_movement_input(
    mut q_agents: Query<(
        Entity,
        &MovementInput,
        &mut MovementState,
        &Collider,
        &mut Transform,
    )>,
    time: Res<Time>,
    query: Res<RapierContext>,
) {
    let dt = time.delta_seconds();

    // TODO
    let offset = 0.01;
    let step_height = 0.3;
    let gravity = 30.0;
    let speed = 6.0;
    let jump_velocity = 9.0;
    let air_acceleration = 30.0;
    let ground_acceleration = 300.0;

    for (entity, input, mut state, collider, mut transform) in &mut q_agents {
        let mut position = transform.translation;
        let mut velocity = state.velocity;

        let prev_position = position;

        let shape_cast = |pos, dir, limit| {
            query
                .cast_shape(
                    pos,
                    Quat::IDENTITY,
                    dir,
                    collider,
                    limit,
                    QueryFilter {
                        exclude_collider: Some(entity),
                        flags: QueryFilterFlags::EXCLUDE_DYNAMIC,
                        ..default()
                    },
                )
                .map(|(_, toi)| toi.toi)
        };

        let is_grounded = shape_cast(position, -Vec3::Z, 2.0 * offset).is_some();
        let enable_stepping = is_grounded && !input.jump;

        let acceleration = if is_grounded {
            ground_acceleration
        } else {
            air_acceleration
        };

        let velocity_target = input.direction * speed;
        let change = velocity_target - velocity.xy();
        let impulse = change.normalize_or_zero() * change.length().min(acceleration * dt);
        velocity.x += impulse.x;
        velocity.y += impulse.y;

        if is_grounded {
            velocity.z = if input.jump { jump_velocity } else { 0.0 };
        } else {
            velocity.z -= gravity * dt;
        }

        if enable_stepping {
            // cast up
            let limit = step_height;
            let dist = shape_cast(position, Vec3::Z, limit).unwrap_or(limit + offset) - offset;
            position.z += dist;
        }

        // cast forward
        let dir = velocity.normalize_or_zero();
        let limit = velocity.length() * dt;
        let dist = shape_cast(position, dir, limit).unwrap_or(limit + offset) - offset;
        position += dir * dist;

        if enable_stepping {
            // cast down
            let limit = position.z - prev_position.z + step_height;
            let dist = shape_cast(position, -Vec3::Z, limit).unwrap_or(limit + offset) - offset;
            position.z -= dist;
        }

        let translation = position - prev_position;
        velocity = translation / dt;

        state.velocity = velocity;
        transform.translation = position;
    }
}
