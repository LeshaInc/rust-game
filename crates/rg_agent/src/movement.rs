use std::fmt::Debug;

use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;
use rg_core::CollisionLayer;

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
    pub position: Position,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub collision_layers: CollisionLayers,
    pub locked_axes: LockedAxes,
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self {
            movement_input: MovementInput::default(),
            movement_state: MovementState::default(),
            position: Position::default(),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            rigid_body: RigidBody::Kinematic,
            collider: Collider::default(),
            collision_layers: CollisionLayers::new(
                [CollisionLayer::Character],
                [
                    CollisionLayer::Static,
                    CollisionLayer::Dynamic,
                    CollisionLayer::Character,
                ],
            ),
            locked_axes: LockedAxes::ROTATION_LOCKED,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Component)]
pub struct MovementInput {
    pub direction: Vec2,
    pub jump: bool,
}

#[derive(Copy, Clone, Debug, Default, Component)]
pub struct MovementState {
    pub velocity: Vec3,
    pub jump_time: f32,
}

fn handle_movement_input(
    mut q_agents: Query<(
        Entity,
        &MovementInput,
        &mut MovementState,
        &Collider,
        &mut Position,
    )>,
    time: Res<Time>,
    query: Res<SpatialQueryPipeline>,
) {
    let dt = time.delta_seconds();

    // TODO
    let offset = 0.01;
    let step_height = 0.3;
    let gravity = 30.0;
    let speed = 6.0;
    let jump_velocity = 5.0;
    let jump_time = 0.3;
    let jump_acceleration = 5.0;
    let air_acceleration = 30.0;
    let ground_acceleration = 300.0;

    for (entity, input, mut state, collider, mut out_position) in &mut q_agents {
        let mut velocity = state.velocity;
        let mut position = out_position.0;
        let prev_position = position;

        let shape_cast = |pos, dir, limit| {
            query
                .cast_shape(
                    collider,
                    pos,
                    Quat::IDENTITY,
                    dir,
                    limit,
                    false,
                    SpatialQueryFilter::default()
                        .with_masks([CollisionLayer::Static])
                        .without_entities([entity]),
                )
                .map(|hit| hit.time_of_impact)
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
            if input.jump {
                velocity.z = jump_velocity;
                state.jump_time = jump_time;
            } else {
                velocity.z = 0.0;
                state.jump_time = 0.0;
            }
        } else {
            if input.jump && state.jump_time > 0.0 {
                velocity.z += jump_acceleration * dt;
                state.jump_time -= dt;
            } else {
                velocity.z -= gravity * dt;
                state.jump_time = 0.0;
            }
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
        out_position.0 = position;
    }
}
