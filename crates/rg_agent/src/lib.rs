mod character;

use bevy::prelude::*;

pub use crate::character::{CharacterPlugin, ControlledCharacter, SpawnCharacter};

pub struct AgentPlugin;

impl Plugin for AgentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CharacterPlugin);
    }
}
