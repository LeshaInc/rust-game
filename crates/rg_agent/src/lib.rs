mod character;
mod movement;

use bevy::prelude::*;

pub use crate::character::{CharacterPlugin, ControlledCharacter, SpawnCharacter};
use crate::movement::MovementPlugin;

pub struct AgentPlugin;

impl Plugin for AgentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MovementPlugin).add_plugins(CharacterPlugin);
    }
}
