mod material;
mod mesh;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_core::chunk::Chunk;
use rg_core::CollisionLayers;
use rg_navigation_api::NavMeshAffector;

use self::material::{SurfaceMaterials, SurfaceMaterialsPlugin};
use self::mesh::{generate_mesh, MeshResult};
use crate::{SharedChunkMaps, MAX_TASKS_IN_FLIGHT};

pub struct SurfacePlugin;

impl Plugin for SurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SurfaceMaterialsPlugin).add_systems(
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
        let task = task_pool
            .spawn(async move { generate_mesh(&chunk_maps.height_map, &chunk_maps.water_map) });
        commands.entity(chunk_id).insert(SurfaceTask(task));
    }
}

fn update_tasks(
    mut q_chunks: Query<(Entity, &mut SurfaceTask)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<SurfaceMaterials>,
) {
    for (chunk_id, mut task) in q_chunks.iter_mut() {
        let Some(res) = future::block_on(future::poll_once(&mut task.0)) else {
            continue;
        };

        let water = commands
            .spawn(MaterialMeshBundle {
                mesh: meshes.add(res.water_mesh),
                material: material.water.clone(),
                ..default()
            })
            .id();

        commands
            .entity(chunk_id)
            .remove::<SurfaceTask>()
            .insert((
                NavMeshAffector,
                meshes.add(res.terrain_mesh),
                res.terrain_collider,
                CollisionLayers::STATIC_WALKABLE_GROUP,
                material.terrain.clone(),
            ))
            .add_child(water);
    }
}
