use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::rapier::prelude::{
    Aabb, ColliderBuilder, ColliderHandle, ColliderSet, QueryFilter, QueryPipeline, Ray,
    RayIntersection, RigidBodySet,
};
use futures_lite::future;
use rg_core::Grid;
use rg_terrain::{chunk_cell_to_world, Chunk, ChunkPos, CHUNK_RESOLUTION, MAX_UPDATES_PER_FRAME};

pub const MIN_Y: f32 = -200.0;
pub const MAX_Y: f32 = 200.0;
pub const CLIMB_HEIGHT: f32 = 0.5;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                schedule_tasks,
                update_tasks,
                draw_nav_mesh_gizmos.run_if(rg_dev_overlay::is_enabled),
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

        let min = chunk_cell_to_world(chunk_pos, IVec2::ZERO)
            .extend(MIN_Y)
            .xzy();
        let max = chunk_cell_to_world(chunk_pos + IVec2::ONE, IVec2::ZERO)
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

        let task = task_pool.spawn(async move {
            let generator = NavMeshGenerator::new(chunk_pos, colliders);
            generator.generate()
        });

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
    heightmap: Grid<f32>,
    connections: Grid<u8>,
}

pub struct NavMeshGenerator {
    chunk_pos: IVec2,
    colliders: ColliderSet,
    heightmap: Grid<f32>,
    connections: Grid<u8>,
}

impl NavMeshGenerator {
    pub fn new(chunk_pos: IVec2, colliders: ColliderSet) -> NavMeshGenerator {
        NavMeshGenerator {
            chunk_pos,
            colliders,
            heightmap: Grid::new(CHUNK_RESOLUTION, f32::NAN),
            connections: Grid::new(CHUNK_RESOLUTION, 0),
        }
    }

    pub fn generate(mut self) -> ChunkNavMesh {
        let _span = info_span!("nav mesh generator").entered();

        self.generate_heightmap();
        self.generate_connections();

        ChunkNavMesh {
            heightmap: self.heightmap,
            connections: self.connections,
        }
    }

    fn generate_heightmap(&mut self) {
        let rigid_bodies = RigidBodySet::new();
        let mut query_pipeline = QueryPipeline::new();
        query_pipeline.update(&rigid_bodies, &self.colliders);

        for cell in self.heightmap.cells() {
            let pos = chunk_cell_to_world(self.chunk_pos, cell);

            let ray_origin = pos.extend(MIN_Y).xzy();
            let max_toi = MAX_Y - MIN_Y;
            let solid = false;

            let filter = QueryFilter::new();

            let mut max_height = f32::NEG_INFINITY;
            let callback = |_, intersection: RayIntersection| {
                let height = ray_origin.y + intersection.toi;
                max_height = max_height.max(height);
                true // continue search
            };

            query_pipeline.intersections_with_ray(
                &rigid_bodies,
                &self.colliders,
                &Ray::new(ray_origin.into(), Vec3::Y.into()),
                max_toi,
                solid,
                filter,
                callback,
            );

            if max_height.is_finite() {
                self.heightmap.set(cell, max_height);
            }
        }
    }

    fn generate_connections(&mut self) {
        for cell in self.heightmap.cells() {
            let cell_height = self.heightmap[cell];
            for (i, neighbor) in self.heightmap.neighborhood_8(cell) {
                let neighbor_height = self.heightmap[neighbor];
                if (cell_height - neighbor_height).abs() <= CLIMB_HEIGHT {
                    self.connections[cell] |= (1 << i) as u8;
                }
            }
        }
    }
}

fn draw_nav_mesh_gizmos(q_chunks: Query<(&ChunkPos, &ChunkNavMesh)>, mut gizmos: Gizmos) {
    for (&ChunkPos(chunk_pos), nav_grid) in &q_chunks {
        for (cell, height) in nav_grid.heightmap.entries() {
            let pos = chunk_cell_to_world(chunk_pos, cell)
                .extend(height + 0.1)
                .xzy();

            for (i, neighbor) in nav_grid.heightmap.neighborhood_8(cell) {
                if nav_grid.connections[cell] & (1 << i) as u8 == 0 {
                    continue;
                }

                let neighbor_height = nav_grid.heightmap[neighbor];

                let neighbor_pos = chunk_cell_to_world(chunk_pos, neighbor)
                    .extend(neighbor_height + 0.1)
                    .xzy();

                gizmos.line(pos, neighbor_pos, Color::GREEN);
            }
        }
    }
}
