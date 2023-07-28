#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod generator;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_rapier3d::prelude::RapierContext;
use futures_lite::future;
use rg_dev_overlay::DevOverlaySettings;
use rg_terrain::{chunk_pos_to_world, Chunk, ChunkFullyLoaded, ChunkPos};

use crate::generator::{extract_colliders, generate_navmesh, ChunkNavMesh, NavMeshSettings};

const MAX_UPDATES_PER_FRAME: usize = 32;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavMeshSettings>().add_systems(
            Update,
            (
                schedule_tasks,
                update_tasks,
                draw_nav_mesh_gizmos
                    .run_if(|s: Res<DevOverlaySettings>| s.enabled && s.show_navmesh),
            ),
        );
    }
}

#[derive(Component)]
struct NavMeshTask(pub Task<ChunkNavMesh>);

#[derive(Component)]
struct NavMeshGenerated;

fn schedule_tasks(
    q_chunks: Query<
        (Entity, &ChunkPos),
        (
            With<Chunk>,
            With<ChunkFullyLoaded>,
            Without<NavMeshTask>,
            Without<NavMeshGenerated>,
        ),
    >,
    physics_context: Res<RapierContext>,
    settings: Res<NavMeshSettings>,
    mut commands: Commands,
) {
    let settings = *settings;
    let task_pool = AsyncComputeTaskPool::get();

    for (chunk_id, &ChunkPos(chunk_pos)) in q_chunks.iter().take(MAX_UPDATES_PER_FRAME) {
        let colliders = extract_colliders(&settings, &physics_context, chunk_pos);
        let task =
            task_pool.spawn(async move { generate_navmesh(&settings, chunk_pos, colliders) });
        commands.entity(chunk_id).insert(NavMeshTask(task));
    }
}

fn update_tasks(
    mut q_chunks: Query<(Entity, &mut NavMeshTask), With<Chunk>>,
    mut commands: Commands,
) {
    for (chunk_id, mut task) in q_chunks.iter_mut().take(MAX_UPDATES_PER_FRAME) {
        let Some(res) = future::block_on(future::poll_once(&mut task.0)) else {
            continue;
        };

        commands
            .entity(chunk_id)
            .remove::<NavMeshTask>()
            .insert((res, NavMeshGenerated));
    }
}

fn draw_nav_mesh_gizmos(
    q_chunks: Query<(&ChunkPos, &ChunkNavMesh, &ComputedVisibility)>,
    mut gizmos: Gizmos,
) {
    for (&ChunkPos(chunk_pos), nav_mesh, visibility) in &q_chunks {
        // if chunk_pos.x % 2 != 0 || chunk_pos.y % 2 != 0 {
        //     continue;
        // }

        if !visibility.is_visible() {
            continue;
        }

        let chunk_pos_world = chunk_pos_to_world(chunk_pos);

        for &(start, end) in &nav_mesh.triangulation_edges {
            let start_z = nav_mesh.sample_height(start) + 0.1;
            let end_z = nav_mesh.sample_height(end) + 0.1;

            // let start_z = 25.0;
            // let end_z = 25.0;

            let start = (chunk_pos_world + start).extend(start_z);
            let end = (chunk_pos_world + end).extend(end_z);
            gizmos.line(start, end, Color::RED);

            gizmos.circle(start, Vec3::Z, 0.05, Color::RED);
            gizmos.circle(end, Vec3::Z, 0.05, Color::RED);
        }
    }
}
