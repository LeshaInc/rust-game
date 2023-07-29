mod material;
mod mesh;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_core::CollisionLayers;

use self::material::{DefaultTerrainMaterial, TerrainMaterialPlugin};
use self::mesh::{generate_mesh, MeshResult};
use crate::{Chunk, SharedChunkMaps, MAX_TASKS_IN_FLIGHT};

pub struct SurfacePlugin;

impl Plugin for SurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainMaterialPlugin).add_systems(
            Update,
            (
                schedule_tasks,
                update_tasks.run_if(|q: Query<&SurfaceTask>| !q.is_empty()),
            ),
        );
    }
}

#[derive(Component)]
struct SurfaceTask(Task<MeshResult>);

fn schedule_tasks(
    q_chunks: Query<
        (Entity, &SharedChunkMaps),
        (With<Chunk>, Without<Handle<Mesh>>, Without<SurfaceTask>),
    >,
    q_in_flight: Query<(), With<SurfaceTask>>,
    mut commands: Commands,
) {
    let task_pool = AsyncComputeTaskPool::get();

    let mut in_flight = q_in_flight.iter().count();

    for (chunk_id, chunk_maps) in q_chunks.iter() {
        if in_flight >= MAX_TASKS_IN_FLIGHT {
            break;
        }

        in_flight += 1;

        let chunk_maps = chunk_maps.clone();
        let task = task_pool.spawn(async move { generate_mesh(&chunk_maps.height_map) });
        commands.entity(chunk_id).insert(SurfaceTask(task));
    }
}

fn update_tasks(
    mut q_chunks: Query<(Entity, &mut SurfaceTask)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<DefaultTerrainMaterial>,
) {
    for (chunk_id, mut task) in q_chunks.iter_mut() {
        let Some(res) = future::block_on(future::poll_once(&mut task.0)) else {
            continue;
        };

        commands.entity(chunk_id).remove::<SurfaceTask>().insert((
            meshes.add(res.mesh),
            res.collider,
            CollisionLayers::STATIC_WALKABLE_GROUP,
            material.0.clone(),
        ));
    }
}
