use std::time::Duration;

use bevy::prelude::*;

use crate::{Action, AddAction, Behavior, BehaviorTreeSystem};

#[derive(Default)]
pub struct DefaultActionsPlugin;

impl Plugin for DefaultActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_action::<SequenceUntilFailure>()
            .add_action::<SequenceUntilSuccess>()
            .add_action::<AlwaysSucceed>()
            .add_action::<AlwaysFail>()
            .add_action::<InvertResult>()
            .add_action::<Sleep>()
            .add_action::<LogMessage>();
    }
}

#[derive(Default, Clone, Reflect)]
pub struct SequenceUntilFailure {
    index: usize,
}

impl Action for SequenceUntilFailure {
    fn register(app: &mut App) {
        app.add_systems(
            Update,
            process_sequence_until_failure.in_set(BehaviorTreeSystem::Process),
        );
    }
}

fn process_sequence_until_failure(mut q_agents: Query<&mut Behavior<SequenceUntilFailure>>) {
    for mut behavior in &mut q_agents {
        if behavior.child_failed() {
            behavior.failure();
            continue;
        }

        let index = behavior.action.index;
        if index < behavior.num_children() {
            behavior.run_child(index);
            behavior.action.index += 1;
        } else {
            behavior.success();
        }
    }
}

#[derive(Default, Clone, Reflect)]
pub struct SequenceUntilSuccess {
    index: usize,
}

impl Action for SequenceUntilSuccess {
    fn register(app: &mut App) {
        app.add_systems(
            Update,
            process_sequence_until_success.in_set(BehaviorTreeSystem::Process),
        );
    }
}

fn process_sequence_until_success(mut q_agents: Query<&mut Behavior<SequenceUntilSuccess>>) {
    for mut behavior in &mut q_agents {
        if behavior.child_succeeded() {
            behavior.success();
            continue;
        }

        let index = behavior.action.index;
        if index < behavior.num_children() {
            behavior.run_child(index);
            behavior.action.index += 1;
        } else {
            behavior.failure();
        }
    }
}

#[derive(Default, Clone, Reflect)]
pub struct AlwaysSucceed;

impl Action for AlwaysSucceed {
    fn register(app: &mut App) {
        app.add_systems(
            Update,
            process_always_succeed.in_set(BehaviorTreeSystem::Process),
        );
    }
}

fn process_always_succeed(mut q_agents: Query<&mut Behavior<AlwaysSucceed>>) {
    for mut behavior in &mut q_agents {
        if behavior.num_children() == 0 || behavior.has_returned_from_child() {
            behavior.success();
        } else {
            behavior.run_child(0);
        }
    }
}

#[derive(Default, Clone, Reflect)]
pub struct AlwaysFail;

impl Action for AlwaysFail {
    fn register(app: &mut App) {
        app.add_systems(
            Update,
            process_always_fail.in_set(BehaviorTreeSystem::Process),
        );
    }
}

fn process_always_fail(mut q_agents: Query<&mut Behavior<AlwaysFail>>) {
    for mut behavior in &mut q_agents {
        if behavior.num_children() == 0 || behavior.has_returned_from_child() {
            behavior.failure();
        } else {
            behavior.run_child(0);
        }
    }
}

#[derive(Default, Clone, Reflect)]
pub struct InvertResult;

impl Action for InvertResult {
    fn register(app: &mut App) {
        app.add_systems(
            Update,
            process_invert_result.in_set(BehaviorTreeSystem::Process),
        );
    }
}

fn process_invert_result(mut q_agents: Query<&mut Behavior<InvertResult>>) {
    for mut behavior in &mut q_agents {
        if !behavior.has_returned_from_child() {
            behavior.run_child(0);
            continue;
        }

        if behavior.child_succeeded() {
            behavior.failure()
        } else {
            behavior.success()
        }
    }
}

#[derive(Default, Clone, Reflect)]
pub struct Sleep {
    pub duration: Duration,
}

impl Action for Sleep {
    fn register(app: &mut App) {
        app.add_systems(Update, process_sleep.in_set(BehaviorTreeSystem::Process));
    }
}

fn process_sleep(mut q_agents: Query<&mut Behavior<Sleep>>, time: Res<Time>) {
    let delta = time.delta();
    for mut behavior in &mut q_agents {
        match behavior.action.duration.checked_sub(delta) {
            Some(v) => behavior.action.duration = v,
            None => behavior.success(),
        }
    }
}

#[derive(Default, Clone, Reflect)]
pub struct LogMessage {
    pub message: String,
}

impl Action for LogMessage {
    fn register(app: &mut App) {
        app.add_systems(
            Update,
            process_log_message.in_set(BehaviorTreeSystem::Process),
        );
    }
}

fn process_log_message(mut q_agents: Query<(Entity, &mut Behavior<LogMessage>)>) {
    for (entity, mut behavior) in &mut q_agents {
        debug!(
            "message from agent {:?}: {}",
            entity, behavior.action.message
        );
        behavior.success();
    }
}
