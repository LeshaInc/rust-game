mod generator;
mod material;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_core::billboard::{MultiBillboard, MultiBillboardBundle};
use rg_core::chunk::{Chunk, ChunkPos};
use rg_worldgen::{SharedWorldMaps, WorldSeed};

use self::generator::{generate, GrassResult};
use self::material::{DefaultGrassMaterial, GrassMaterialPlugin};
use crate::{SharedChunkMaps, MAX_TASKS_IN_FLIGHT};

pub struct GrassPlugin;

impl Plugin for GrassPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GrassMaterialPlugin).add_systems(
            Update,
            (
                schedule_tasks.run_if(resource_exists::<SharedWorldMaps>()),
                update_tasks.run_if(|q: Query<&GrassTask>| !q.is_empty()),
            ),
        );
    }
}

#[derive(Component)]
struct GrassTask(Task<GrassResult>);

#[derive(Debug, Copy, Clone, Component)]
pub struct ChunkGrass(pub Entity);

fn schedule_tasks(
    q_chunks: Query<
        (Entity, &ChunkPos, &Handle<Mesh>, &SharedChunkMaps),
        (With<Chunk>, Without<ChunkGrass>, Without<GrassTask>),
    >,
    q_in_flight: Query<(), With<GrassTask>>,
    seed: Res<WorldSeed>,
    meshes: Res<Assets<Mesh>>,
    mut commands: Commands,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let seed = seed.0;

    let mut in_flight = q_in_flight.iter().count();

    for (chunk_id, &ChunkPos(chunk_pos), mesh, chunk_maps) in q_chunks.iter() {
        if in_flight >= MAX_TASKS_IN_FLIGHT {
            break;
        }

        let Some(mesh) = meshes.get(mesh).cloned() else {
            continue;
        };

        let chunk_maps = chunk_maps.clone();

        let task = task_pool
            .spawn(async move { generate(seed, chunk_pos, &mesh, &chunk_maps.grass_density_map) });
        commands.entity(chunk_id).insert(GrassTask(task));

        in_flight += 1;
    }
}

fn update_tasks(
    mut q_chunks: Query<(Entity, &mut GrassTask)>,
    mut multi_billboards: ResMut<Assets<MultiBillboard>>,
    material: Res<DefaultGrassMaterial>,
    mut commands: Commands,
) {
    for (chunk_id, mut task) in q_chunks.iter_mut() {
        let Some(res) = future::block_on(future::poll_once(&mut task.0)) else {
            continue;
        };

        let grass_id = commands
            .spawn((
                Name::new("Grass"),
                material.0.clone(),
                MultiBillboardBundle {
                    multi_billboard: multi_billboards.add(res.multi_billboard),
                    ..default()
                },
            ))
            .id();

        commands
            .entity(chunk_id)
            .remove::<GrassTask>()
            .insert(ChunkGrass(grass_id))
            .add_child(grass_id);
    }
}
