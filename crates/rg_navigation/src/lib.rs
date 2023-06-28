use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::rapier::prelude::{
    Aabb, ColliderBuilder, ColliderHandle, ColliderSet, QueryFilter, QueryPipeline, Ray,
    RayIntersection, RigidBodySet,
};
use futures_lite::future;
use rg_terrain::{Chunk, ChunkPos, CHUNK_SIZE, MAX_UPDATES_PER_FRAME};

pub const NAV_MESH_RESOLUTION: u32 = 128;
pub const MIN_Y: f32 = -200.0;
pub const MAX_Y: f32 = 200.0;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (schedule_tasks, update_tasks));
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
            Without<NavMeshTask>,
            Without<NavMeshGenerated>,
            With<Collider>,
            With<Chunk>,
        ),
    >,
    physics_context: Res<RapierContext>,
    mut commands: Commands,
) {
    for (chunk_id, &ChunkPos(chunk_pos)) in q_chunks.iter().take(MAX_UPDATES_PER_FRAME) {
        let task_pool = AsyncComputeTaskPool::get();

        let min = (chunk_pos.as_vec2() * (CHUNK_SIZE as f32))
            .extend(MIN_Y)
            .xzy();
        let max = ((chunk_pos + IVec2::splat(1)).as_vec2() * (CHUNK_SIZE as f32))
            .extend(MAX_Y)
            .xzy();
        let aabb = Aabb::new(min.into(), max.into());

        let mut colliders = ColliderSet::new();
        let callback = |&handle: &ColliderHandle| {
            if let Some(collider) = physics_context.colliders.get(handle) {
                colliders.insert(
                    ColliderBuilder::new(collider.shared_shape().clone())
                        .position(*collider.position())
                        .build(),
                );
            }
            true // continue search
        };

        physics_context
            .query_pipeline
            .colliders_with_aabb_intersecting_aabb(&aabb, callback);

        let task = task_pool.spawn(async move { generate_navmesh(chunk_pos, colliders) });

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
            .insert(res)
            .insert(NavMeshGenerated);
    }
}

#[derive(Debug, Component)]
pub struct ChunkNavMesh {
    heights: Vec<f32>,
}

fn generate_navmesh(chunk_pos: IVec2, colliders: ColliderSet) -> ChunkNavMesh {
    let _span = info_span!("nav mesh generator").entered();

    let rigid_bodies = RigidBodySet::new();
    let mut query_pipeline = QueryPipeline::new();
    query_pipeline.update(&rigid_bodies, &colliders);

    let mut grid = vec![f32::NAN; (NAV_MESH_RESOLUTION as usize).pow(2)];

    let cells = (0..NAV_MESH_RESOLUTION)
        .flat_map(|x| (0..NAV_MESH_RESOLUTION).map(move |y| UVec2::new(x, y)));

    for cell in cells {
        let cell_idx = (cell.x as usize) * (NAV_MESH_RESOLUTION as usize) + (cell.y as usize);

        let pos = ((cell.as_vec2() + Vec2::splat(0.5)) / (NAV_MESH_RESOLUTION as f32)
            + chunk_pos.as_vec2())
            * CHUNK_SIZE;

        let origin = pos.extend(MIN_Y).xzy();
        let max_toi = MAX_Y - MIN_Y;
        let solid = false;

        let filter = QueryFilter::new();
        let callback = |_, intersection: RayIntersection| {
            grid[cell_idx] = origin.y + intersection.toi;
            true // continue search
        };

        query_pipeline.intersections_with_ray(
            &rigid_bodies,
            &colliders,
            &Ray::new(origin.into(), Vec3::Y.into()),
            max_toi,
            solid,
            filter,
            callback,
        );
    }

    ChunkNavMesh { heights: grid }
}

#[allow(dead_code)]
fn draw_nav_mesh_gizmos(q_chunks: Query<(&ChunkPos, &ChunkNavMesh)>, mut gizmos: Gizmos) {
    for (chunk_pos, nav_grid) in &q_chunks {
        for x in 0..NAV_MESH_RESOLUTION {
            for y in 0..NAV_MESH_RESOLUTION {
                let index = (x as usize) * (NAV_MESH_RESOLUTION as usize) + (y as usize);
                let height = nav_grid.heights[index];
                let pos_2d = ((UVec2::new(x, y).as_vec2() + Vec2::new(0.5, 0.5))
                    / (NAV_MESH_RESOLUTION as f32)
                    + chunk_pos.0.as_vec2())
                    * CHUNK_SIZE;
                let pos = pos_2d.extend(height + 0.1).xzy();
                gizmos.circle(pos, Vec3::Y, 0.05, Color::RED);
            }
        }
    }
}
