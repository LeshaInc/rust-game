use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::HashMap;
use bevy_rapier3d::prelude::{PhysicsSet, RapierContext};
use futures_lite::future;
use rg_core::chunk::WorldOrigin;
use rg_navigation_api::{AddNavMeshChunk, NavMeshAffector, RemoveNavMeshChunk};

use crate::collider_set::ColliderSet;
use crate::generator::generate_chunk;
use crate::{NavMesh, NavMeshChunk, NavMeshSettings};

pub struct ListenerPlugin;

impl Plugin for ListenerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavMesh>()
            .add_event::<AddNavMeshChunk>()
            .add_event::<RemoveNavMeshChunk>()
            .init_resource::<ChunkTasks>()
            .add_systems(PreUpdate, poll_tasks)
            .add_systems(
                PostUpdate,
                (handle_removed, handle_added.after(PhysicsSet::SyncBackend)).chain(),
            );
    }
}

#[derive(Default, Resource)]
struct ChunkTasks {
    map: HashMap<IVec2, Task<NavMeshChunk>>,
}

fn handle_added(
    q_affectors: Query<(), With<NavMeshAffector>>,
    mut ev_added: EventReader<AddNavMeshChunk>,
    settings: Res<NavMeshSettings>,
    physics_context: Res<RapierContext>,
    mut chunk_tasks: ResMut<ChunkTasks>,
    mut navmesh: ResMut<NavMesh>,
    origin: Res<WorldOrigin>,
) {
    let pool = AsyncComputeTaskPool::get();
    let origin = origin.0;
    let settings = *settings;

    for &AddNavMeshChunk(chunk_pos) in ev_added.read() {
        if chunk_tasks.map.contains_key(&chunk_pos) {
            continue;
        }

        let mut collider_set =
            ColliderSet::extract(&settings, &physics_context, &q_affectors, origin, chunk_pos);

        if collider_set.is_empty() {
            navmesh.remove_chunk(chunk_pos);
            continue;
        }

        let task = pool.spawn(async move {
            collider_set.update();
            generate_chunk(&settings, &collider_set, origin, chunk_pos)
        });

        chunk_tasks.map.insert(chunk_pos, task);
    }
}

fn handle_removed(
    mut ev_removed: EventReader<RemoveNavMeshChunk>,
    mut chunk_tasks: ResMut<ChunkTasks>,
    mut navmesh: ResMut<NavMesh>,
) {
    for RemoveNavMeshChunk(chunk_pos) in ev_removed.read() {
        navmesh.remove_chunk(*chunk_pos);
        chunk_tasks.map.remove(chunk_pos);
    }
}

fn poll_tasks(mut chunk_tasks: ResMut<ChunkTasks>, mut navmesh: ResMut<NavMesh>) {
    chunk_tasks.map.retain(|&chunk_pos, task| {
        let Some(chunk) = future::block_on(future::poll_once(task)) else {
            return true;
        };

        if chunk.is_empty {
            navmesh.remove_chunk(chunk_pos);
        } else {
            navmesh.insert_chunks(chunk_pos, chunk);
        }

        false
    });
}
