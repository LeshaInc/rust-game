mod generator;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_rapier3d::prelude::RapierContext;
use futures_lite::future;
use rg_dev_overlay::DevOverlaySettings;
use rg_terrain::{Chunk, ChunkFullyLoaded, ChunkPos};

use crate::generator::{
    extract_colliders, generate_navmesh, node_pos_to_world, node_pos_to_world_f32, ChunkNavMesh,
    NavMeshSettings,
};

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
    for (&ChunkPos(chunk_pos), nav_grid, visibility) in &q_chunks {
        if chunk_pos.x % 2 != 0 || chunk_pos.y % 2 != 0 {
            continue;
        }

        if !visibility.is_visible() {
            continue;
        }

        for (cell, height) in nav_grid.heightmap.entries() {
            if height.is_nan() {
                continue;
            }

            let _pos = node_pos_to_world(chunk_pos, cell).extend(height + 0.1);

            for (i, neighbor) in nav_grid.heightmap.neighborhood_4(cell) {
                if nav_grid.connections[cell] & (1 << i) as u8 == 0 {
                    continue;
                }

                let neighbor_height = nav_grid.heightmap[neighbor];
                if neighbor_height.is_nan() {
                    continue;
                }

                let _neighbor_pos =
                    node_pos_to_world(chunk_pos, neighbor).extend(neighbor_height + 0.1);

                // gizmos.line(pos, neighbor_pos, Color::GREEN);
            }
        }

        for &(start, end) in &nav_grid.edges {
            let start_z = 25.0; // nav_grid.heightmap.sample(start) + 0.1;
            let end_z = 25.0; //nav_grid.heightmap.sample(end) + 0.1;

            let start = node_pos_to_world_f32(chunk_pos, start).extend(start_z);
            let end = node_pos_to_world_f32(chunk_pos, end).extend(end_z);

            let color = Color::RED;

            gizmos.line(start, end, color);
            gizmos.line(
                end,
                end + (start - end).normalize() * 0.1
                    + (start - end).normalize().cross(Vec3::Z) * 0.05,
                color,
            );
            gizmos.line(
                end,
                end + (start - end).normalize() * 0.1
                    - (start - end).normalize().cross(Vec3::Z) * 0.05,
                color,
            );
        }
    }
}
