mod heightmap;
mod material;
mod mesh;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_xpbd_3d::prelude::*;
use futures_lite::future;
use rg_core::CollisionLayer;
use rg_worldgen::{SharedWorldMaps, WorldSeed};

use self::heightmap::generate_heightmap;
use self::material::{DefaultTerrainMaterial, TerrainMaterialPlugin};
use self::mesh::{generate_mesh, MeshResult};
use crate::{Chunk, ChunkPos, ChunkSpawnCenter, CHUNK_SIZE};

const MAX_TASKS_IN_FLIGHT: usize = 8;

pub struct SurfacePlugin;

impl Plugin for SurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainMaterialPlugin).add_systems(
            Update,
            (update_chunks, schedule_tasks.after(update_chunks))
                .run_if(resource_exists::<SharedWorldMaps>()),
        );
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
    world_maps: Res<SharedWorldMaps>,
    seed: Res<WorldSeed>,
    spawn_center: Res<ChunkSpawnCenter>,
    mut commands: Commands,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let seed = seed.0;
    let spawn_center = spawn_center.0;

    let mut in_flight = q_in_flight.iter().count();
    let mut new_tasks = Vec::new();

    for (chunk_id, &ChunkPos(chunk_pos)) in q_chunks.iter() {
        if in_flight >= MAX_TASKS_IN_FLIGHT {
            break;
        }

        in_flight += 1;

        let world_maps = world_maps.clone();
        new_tasks.push((chunk_id, chunk_pos, async move {
            let heightmap = generate_heightmap(seed, chunk_pos, &world_maps);
            generate_mesh(&heightmap)
        }));
    }

    new_tasks.sort_by(|a, b| {
        let a = spawn_center.distance_squared((a.1.as_vec2() + Vec2::splat(0.5)) * CHUNK_SIZE);
        let b = spawn_center.distance_squared((b.1.as_vec2() + Vec2::splat(0.5)) * CHUNK_SIZE);
        a.total_cmp(&b)
    });

    for (chunk_id, _, task) in new_tasks {
        commands
            .entity(chunk_id)
            .insert(SurfaceTask(task_pool.spawn(task)));
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
            material.0.clone(),
            RigidBody::Static,
            res.collider,
            Friction::new(1.0),
            DebugRender::none(),
            CollisionLayers::new(
                [CollisionLayer::Static],
                [CollisionLayer::Dynamic, CollisionLayer::Character],
            ),
        ));
    }
}
