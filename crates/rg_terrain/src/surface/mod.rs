mod heightmap;
mod material;
mod mesh;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_worldgen::{WorldMaps, WorldSeed};

use self::heightmap::HeightmapGenerator;
use self::material::{DefaultTerrainMaterial, TerrainMaterialPlugin};
use self::mesh::{MeshGenerator, MeshResult};
use crate::{Chunk, ChunkPos};

const MAX_TASKS_IN_FLIGHT: usize = 8;

pub struct SurfacePlugin;

impl Plugin for SurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainMaterialPlugin)
            .add_systems(Update, (update_chunks, schedule_tasks.after(update_chunks)));
    }
}

#[derive(Component)]
struct SurfaceTask(Task<MeshResult>);

fn schedule_tasks(
    q_chunks: Query<
        (Entity, &ChunkPos),
        (With<Chunk>, Without<Handle<Mesh>>, Without<SurfaceTask>),
    >,
    q_in_flight: Query<(), (With<Chunk>, With<SurfaceTask>)>,
    world_maps: Res<WorldMaps>,
    seed: Res<WorldSeed>,
    mut commands: Commands,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let seed = seed.0;

    let mut in_flight = q_in_flight.iter().count();

    for (chunk_id, &ChunkPos(chunk_pos)) in q_chunks.iter() {
        if in_flight >= MAX_TASKS_IN_FLIGHT {
            break;
        }

        in_flight += 1;

        let world_elevation = world_maps.elevation.clone();

        let task = task_pool.spawn(async move {
            let heightmap_generator = HeightmapGenerator::new(seed, chunk_pos, world_elevation);
            let heightmap = heightmap_generator.generate();
            let mesh_generator = MeshGenerator::new(heightmap);
            mesh_generator.generate()
        });

        commands.entity(chunk_id).insert(SurfaceTask(task));
    }
}

fn update_chunks(
    mut q_chunks: Query<(Entity, &mut SurfaceTask), With<Chunk>>,
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
            material.0.clone(),
        ));
    }
}
