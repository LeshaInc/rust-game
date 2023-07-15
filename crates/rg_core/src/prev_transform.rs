use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Component, Deref)]
pub struct PrevTransform(pub Transform);

pub struct PrevTransformPlugin;

impl Plugin for PrevTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_prev_transform);
    }
}

fn update_prev_transform(mut q_entities: Query<(&Transform, &mut PrevTransform)>) {
    for (transform, mut prev_transform) in q_entities.iter_mut() {
        prev_transform.0 = *transform;
    }
}
