use std::fmt::Debug;

use bevy::prelude::*;
use bevy_rapier3d::prelude::{KinematicCharacterController, KinematicCharacterControllerOutput};

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_movement_input);
    }
}

#[derive(Debug, Default, Component)]
pub struct MovementInput {
    pub direction: Vec3,
    pub jump: bool,
}

fn handle_movement_input(
    mut q_agents: Query<(
        &MovementInput,
        &mut KinematicCharacterController,
        &KinematicCharacterControllerOutput,
    )>,
    time: Res<Time>,
) {
    for (movement, mut controller, controller_output) in &mut q_agents {
        let mut translation = movement.direction * 4.0 * time.delta_seconds();

        if !controller_output.grounded {
            translation.z -= 0.1;
        }

        controller.translation = Some(translation);
    }
}
