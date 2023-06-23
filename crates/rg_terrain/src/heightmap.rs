use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

use crate::{Chunk, ChunkMap, ChunkPos, Seed, CHUNK_RESOLUTION, CHUNK_SIZE};

#[derive(Debug, Default, Clone, Component)]
pub struct ChunkHeightmap(pub ChunkMap<f32>);

pub fn generate(_seed: u64, chunk_pos: IVec2) -> ChunkHeightmap {
    let _span = info_span!("chunk heightmap generator").entered();

    let mut heightmap = ChunkMap::default();
    let mut data = heightmap.make_mut();

    for sx in 0..CHUNK_RESOLUTION {
        for sz in 0..CHUNK_RESOLUTION {
            let fx = ((sx as f32) / (CHUNK_RESOLUTION as f32) + chunk_pos.x as f32) * CHUNK_SIZE;
            let fz = ((sz as f32) / (CHUNK_RESOLUTION as f32) + chunk_pos.y as f32) * CHUNK_SIZE;
            let y = (fx * 0.1).sin() * (fz * 0.1).cos() * 3.0;
            data.set(UVec2::new(sx, sz), y);
        }
    }

    ChunkHeightmap(heightmap)
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

    for (chunk_id, chunk_pos) in &q_chunks {
        let chunk_pos = chunk_pos.0;
        let task = task_pool.spawn(async move { generate(seed, chunk_pos) });
        commands.entity(chunk_id).insert(ChunkHeightmapTask(task));
    }
}

pub fn update_system(
    mut q_chunks: Query<(Entity, &mut ChunkHeightmapTask), With<Chunk>>,
    mut commands: Commands,
) {
    for (chunk_id, mut task) in &mut q_chunks {
        let Some(heightmap) = future::block_on(future::poll_once(&mut task.0)) else  {
            continue;
        };

        commands
            .entity(chunk_id)
            .remove::<ChunkHeightmapTask>()
            .insert(heightmap);
    }
}
