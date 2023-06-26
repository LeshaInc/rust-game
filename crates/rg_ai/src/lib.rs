#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

pub mod actions;
pub mod behavior_tree;

use actions::DefaultActionsPlugin;
use bevy::prelude::*;

pub use crate::behavior_tree::{
    Action, AddAction, Behavior, BehaviorTree, BehaviorTreePlugin, BehaviorTreeSystem,
};

#[derive(Default)]
pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BehaviorTreePlugin, DefaultActionsPlugin));
    }
}
