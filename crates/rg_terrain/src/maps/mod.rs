mod generator;

use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_core::{DeserializedResourcePlugin, Grid};
use rg_worldgen::SharedWorldMaps;

use self::generator::generate_maps;
pub use self::generator::ChunkGenSettings;
use crate::{Chunk, ChunkPos, Tile, MAX_TASKS_IN_FLIGHT};

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DeserializedResourcePlugin::<ChunkGenSettings>::new(
            "default.chunkgen.ron",
        ))
        .add_systems(PreUpdate, update_tasks)
        .add_systems(
            Update,
            (
                schedule_tasks
                    .run_if(resource_exists::<SharedWorldMaps>())
                    .run_if(resource_exists::<ChunkGenSettings>()),
                update_tasks.run_if(|q: Query<&MapsTask>| !q.is_empty()),
            ),
        );
    }
}

#[derive(Debug)]
pub struct ChunkMaps {
    pub height_map: Grid<f32>,
    pub tile_map: Grid<Tile>,
    pub grass_density_map: Grid<f32>,
}

#[derive(Debug, Deref, Clone, Component)]
pub struct SharedChunkMaps(Arc<ChunkMaps>);

#[derive(Component)]
struct MapsTask(Task<SharedChunkMaps>);

fn schedule_tasks(
    q_chunks: Query<
        (Entity, &ChunkPos),
        (With<Chunk>, Without<SharedChunkMaps>, Without<MapsTask>),
    >,
    q_in_flight: Query<With<MapsTask>>,
    world_maps: Res<SharedWorldMaps>,
    settings: Res<ChunkGenSettings>,
    mut commands: Commands,
) {
    let task_pool = AsyncComputeTaskPool::get();

    let mut in_flight = q_in_flight.iter().count();

    for (chunk_id, &ChunkPos(chunk_pos)) in q_chunks.iter() {
        if in_flight >= MAX_TASKS_IN_FLIGHT {
            break;
        }

        in_flight += 1;

        let world_maps = world_maps.clone();
        let settings = settings.clone();
        let task = task_pool.spawn(async move { generate_maps(&settings, chunk_pos, &world_maps) });
        commands.entity(chunk_id).insert(MapsTask(task));
    }
}

fn update_tasks(mut q_chunks: Query<(Entity, &mut MapsTask)>, mut commands: Commands) {
    for (chunk_id, mut task) in q_chunks.iter_mut() {
        let Some(maps) = future::block_on(future::poll_once(&mut task.0)) else {
            continue;
        };

        commands.entity(chunk_id).remove::<MapsTask>().insert(maps);
    }
}
