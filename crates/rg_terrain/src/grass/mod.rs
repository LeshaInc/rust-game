mod density;
mod generator;
mod material;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_billboard::{MultiBillboard, MultiBillboardBundle};
use rg_worldgen::{SharedWorldMaps, WorldSeed};

use self::density::generate_grass_density_map;
use self::generator::{generate, GrassResult};
use self::material::{DefaultGrassMaterial, GrassMaterialPlugin};
use crate::{Chunk, ChunkPos, MAX_TASKS_IN_FLIGHT};

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
        (Entity, &ChunkPos, &Handle<Mesh>),
        (With<Chunk>, Without<ChunkGrass>, Without<GrassTask>),
    >,
    q_in_flight: Query<(), With<GrassTask>>,
    seed: Res<WorldSeed>,
    meshes: Res<Assets<Mesh>>,
    world_maps: Res<SharedWorldMaps>,
    mut commands: Commands,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let seed = seed.0;

    let mut in_flight = q_in_flight.iter().count();

    for (chunk_id, &ChunkPos(chunk_pos), mesh) in q_chunks.iter() {
        if in_flight >= MAX_TASKS_IN_FLIGHT {
            break;
        }

        let Some(mesh) = meshes.get(mesh).cloned() else {
            continue;
        };

        in_flight += 1;

        let world_maps = world_maps.clone();

        let task = task_pool.spawn(async move {
            let density_map = generate_grass_density_map(seed, chunk_pos, &world_maps);
            generate(seed, chunk_pos, mesh, density_map)
        });

        commands.entity(chunk_id).insert(GrassTask(task));
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
