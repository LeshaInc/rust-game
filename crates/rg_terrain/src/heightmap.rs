use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_core::{Grid, SharedGrid, SimplexNoise2};
use rg_worldgen::WorldMaps;

use crate::{chunk_cell_to_world, Chunk, ChunkPos, Seed, CHUNK_RESOLUTION, MAX_UPDATES_PER_FRAME};

#[derive(Debug, Clone, Component)]
pub struct ChunkHeightmap(pub SharedGrid<f32>);

pub fn generate(seed: u64, chunk_pos: IVec2, world_elevation: &Grid<f32>) -> ChunkHeightmap {
    let _span = info_span!("chunk heightmap generator").entered();

    let mut grid = Grid::new_default(CHUNK_RESOLUTION.into());
    let noise = SimplexNoise2::new(seed);

    for (cell, height) in grid.entries_mut() {
        let pos = chunk_cell_to_world(chunk_pos, cell);

        *height = world_elevation.sample(pos / 4.0) * 100.0;
        *height += noise.get(pos / 100.0) * 10.0;
        *height += noise.get(pos / 50.0) * 5.0;
        *height += noise.get(pos / 25.0) * 2.5;
        *height += noise.get(pos / 12.5) * 1.25;
        *height += noise.get(pos / 6.25) * 0.625;

        *height /= 3.0;
        *height = height.floor() + (30.0 * (height.fract() - 0.5)).tanh() * 0.5 + 0.5;
        *height *= 3.0;
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
    world_maps: Res<WorldMaps>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let seed = seed.0;

    for (chunk_id, &ChunkPos(chunk_pos)) in q_chunks.iter().take(MAX_UPDATES_PER_FRAME) {
        let elevation = world_maps.elevation.clone();
        let task = task_pool.spawn(async move { generate(seed, chunk_pos, &elevation) });
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
