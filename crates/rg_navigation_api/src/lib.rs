use bevy::prelude::*;

pub struct NavigationApiPlugin;

impl Plugin for NavigationApiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AddNavMeshChunk>()
            .add_event::<RemoveNavMeshChunk>();
    }
}

#[derive(Debug, Clone, Copy, Event)]
pub struct AddNavMeshChunk(pub IVec2);

#[derive(Debug, Clone, Copy, Event)]
pub struct RemoveNavMeshChunk(pub IVec2);

#[derive(Debug, Clone, Copy, Component)]
pub struct NavMeshAffector;
