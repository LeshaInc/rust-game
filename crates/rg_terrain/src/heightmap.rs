use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_core::{Grid, SharedGrid, SimplexNoise2};

use crate::{chunk_cell_to_world, Chunk, ChunkPos, Seed, CHUNK_RESOLUTION, MAX_UPDATES_PER_FRAME};

#[derive(Debug, Clone, Component)]
pub struct ChunkHeightmap(pub SharedGrid<f32>);

pub fn generate(seed: u64, chunk_pos: IVec2) -> ChunkHeightmap {
    let _span = info_span!("chunk heightmap generator").entered();

    let mut grid = Grid::new_default(CHUNK_RESOLUTION.into());
    let noise = SimplexNoise2::new(seed);

    for (cell, height) in grid.entries_mut() {
        let pos = chunk_cell_to_world(chunk_pos, cell);

        *height = noise.get(pos / 200.0) * 5.0;
        *height += noise.get(pos / 100.0) * 2.5;
        *height += noise.get(pos / 50.0) * 1.25;
        *height += noise.get(pos / 25.0) * 0.625;
        *height += noise.get(pos / 12.5) * 0.3125;

        let floored = (*height / 3.0).floor() * 3.0;

        let mut alpha = noise.get(pos / 10.0);
        alpha += noise.get(pos / 20.0) * 0.5;
        alpha += noise.get(pos / 40.0) * 0.25;
        alpha = (alpha * 0.3 + 0.7).clamp(0.0, 1.0);

        *height = *height * alpha + floored * (1.0 - alpha);

        *height += noise.get(pos / 20.0) * 1.0;
        *height += noise.get(pos / 10.0) * 0.5;
        *height += noise.get(pos / 5.0) * 0.25;

        *height *= 2.0;
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
