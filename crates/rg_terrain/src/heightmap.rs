use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_core::{Grid, SharedGrid};

use crate::{chunk_cell_to_world, Chunk, ChunkPos, Seed, CHUNK_RESOLUTION, MAX_UPDATES_PER_FRAME};

#[derive(Debug, Clone, Component)]
pub struct ChunkHeightmap(pub SharedGrid<f32>);

pub fn generate(_seed: u64, chunk_pos: IVec2) -> ChunkHeightmap {
    let _span = info_span!("chunk heightmap generator").entered();

    let mut grid = Grid::new_default(CHUNK_RESOLUTION.into());

    for (cell, height) in grid.entries_mut() {
        let pos = chunk_cell_to_world(chunk_pos, cell);

        *height = (pos.x * 0.1).sin() * (pos.y * 0.1).cos() * 3.0;
        *height += (pos.x * 0.2).sin() * (pos.y * 0.2).cos() * 2.0;
        *height += (pos.x * 0.4).sin() * (pos.y * 0.4).cos() * 1.0;
    }

    ChunkHeightmap(grid.into())
}

#[derive(Debug, Component)]
pub struct ChunkHeightmapTask(Task<ChunkHeightmap>);

pub fn schedule_system(
    q_chunks: Query<
        (Entity, &ChunkPos),
        (
            With<Chunk>,
            Without<ChunkHeightmap>,
            Without<ChunkHeightmapTask>,
        ),
    >,
    mut commands: Commands,
    seed: Res<Seed>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let seed = seed.0;

    for (chunk_id, &ChunkPos(chunk_pos)) in q_chunks.iter().take(MAX_UPDATES_PER_FRAME) {
        let task = task_pool.spawn(async move { generate(seed, chunk_pos) });
        commands.entity(chunk_id).insert(ChunkHeightmapTask(task));
    }
}

pub fn update_system(
    mut q_chunks: Query<(Entity, &mut ChunkHeightmapTask), With<Chunk>>,
    mut commands: Commands,
) {
    for (chunk_id, mut task) in q_chunks.iter_mut().take(MAX_UPDATES_PER_FRAME) {
        let Some(heightmap) = future::block_on(future::poll_once(&mut task.0)) else {
            continue;
        };

        commands
            .entity(chunk_id)
            .remove::<ChunkHeightmapTask>()
            .insert(heightmap);
    }
}
