use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::transform::TransformSystem;
use bevy::utils::HashMap;
use bevy_rapier3d::na::Isometry;
use bevy_rapier3d::prelude::{Collider, PhysicsSet, RapierContext};
use futures_lite::future;

use crate::collider_set::ColliderSet;
use crate::generator::generate_chunk;
use crate::{NavMesh, NavMeshAffector, NavMeshChunk, NavMeshSettings, CHUNK_OVERSCAN, CHUNK_SIZE};

pub struct ObserverPlugin;

impl Plugin for ObserverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavMesh>()
            .init_resource::<AffectorBounds>()
            .init_resource::<DirtyChunks>()
            .init_resource::<ChunkTasks>()
            .add_systems(PreUpdate, poll_tasks)
            .add_systems(
                PostUpdate,
                (
                    handle_changed_affectors.after(TransformSystem::TransformPropagate),
                    handle_removed_affectors,
                    schedule_tasks.after(PhysicsSet::SyncBackend),
                )
                    .chain(),
            );
    }
}

#[derive(Default, Resource)]
struct AffectorBounds {
    map: HashMap<Entity, (IVec2, IVec2)>,
}

#[derive(Default, Resource)]
struct DirtyChunks {
    map: HashMap<IVec2, u32>,
}

#[derive(Default, Resource)]
struct ChunkTasks {
    map: HashMap<IVec2, Task<NavMeshChunk>>,
}

fn handle_changed_affectors(
    q_affectors: Query<
        (Entity, &GlobalTransform, &Collider),
        (With<NavMeshAffector>, Changed<Collider>),
    >,
    mut dirty_chunks: ResMut<DirtyChunks>,
    mut affector_bounds: ResMut<AffectorBounds>,
) {
    for (entity, transform, collider) in q_affectors.iter() {
        let (_, rotation, translation) = transform.to_scale_rotation_translation();

        let aabb = collider
            .raw
            .compute_aabb(&Isometry {
                rotation: rotation.into(),
                translation: translation.into(),
            })
            .scaled(&collider.scale().into());

        let min = Vec3::from(aabb.mins).xy() - CHUNK_OVERSCAN;
        let max = Vec3::from(aabb.mins).xy() + CHUNK_OVERSCAN;

        let min_chunk = (min / CHUNK_SIZE).floor().as_ivec2();
        let max_chunk = (max / CHUNK_SIZE).floor().as_ivec2();

        affector_bounds.map.insert(entity, (min_chunk, max_chunk));

        for x in min_chunk.x..=max_chunk.x {
            for y in min_chunk.y..=max_chunk.y {
                let chunk_pos = IVec2::new(x, y);
                dirty_chunks.map.insert(chunk_pos, 0);
            }
        }
    }
}

fn handle_removed_affectors(
    mut removed_affectors: RemovedComponents<NavMeshAffector>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    mut affector_bounds: ResMut<AffectorBounds>,
) {
    for entity in removed_affectors.iter() {
        let Some((min_chunk, max_chunk)) = affector_bounds.map.get(&entity) else {
            continue;
        };

        for x in min_chunk.x..=max_chunk.x {
            for y in min_chunk.y..=max_chunk.y {
                let chunk_pos = IVec2::new(x, y);
                dirty_chunks.map.insert(chunk_pos, 0);
            }
        }

        affector_bounds.map.remove(&entity);
    }
}

fn schedule_tasks(
    q_affectors: Query<(), With<NavMeshAffector>>,
    settings: Res<NavMeshSettings>,
    physics_context: Res<RapierContext>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    mut chunk_tasks: ResMut<ChunkTasks>,
    mut navmesh: ResMut<NavMesh>,
) {
    let pool = AsyncComputeTaskPool::get();

    dirty_chunks.map.retain(|&chunk_pos, tick| {
        if *tick < settings.change_delay {
            *tick += 1;
            return true;
        }

        if chunk_tasks.map.len() >= settings.max_tasks_in_flight {
            return true;
        }

        if chunk_tasks.map.contains_key(&chunk_pos) {
            return true;
        }

        let settings = settings.clone();

        let mut collider_set =
            ColliderSet::extract(&settings, &physics_context, &q_affectors, chunk_pos);

        if collider_set.is_empty() {
            navmesh.remove_chunk(chunk_pos);
            return false;
        }

        let task = pool.spawn(async move {
            collider_set.update();
            generate_chunk(&settings, &collider_set, chunk_pos)
        });

        chunk_tasks.map.insert(chunk_pos, task);

        false
    });
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
