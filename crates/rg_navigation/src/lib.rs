use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_rapier3d::na::Isometry3;
use bevy_rapier3d::parry::shape::Capsule;
use bevy_rapier3d::prelude::*;
use bevy_rapier3d::rapier::prelude::{
    Aabb, ColliderBuilder, ColliderHandle, ColliderSet, QueryFilter, QueryPipeline, Ray,
    RayIntersection, RigidBodySet,
};
use futures_lite::future;
use rg_core::{CollisionLayers, Grid};
use rg_dev_overlay::DevOverlaySettings;
use rg_terrain::{
    chunk_pos_to_world, tile_pos_to_world, Chunk, ChunkFullyLoaded, ChunkPos, CHUNK_TILES,
};

const MAX_UPDATES_PER_FRAME: usize = 32;

pub const MIN_HEIGHT: f32 = -200.0;
pub const MAX_HEIGHT: f32 = 200.0;
pub const CLIMB_HEIGHT: f32 = 0.5;
pub const AGENT_HEIGHT: f32 = 1.8;
pub const AGENT_RADIUS: f32 = 0.3;
pub const AGENT_OFFSET: f32 = 0.2;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
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
    mut commands: Commands,
) {
    for (chunk_id, &ChunkPos(chunk_pos)) in q_chunks.iter().take(MAX_UPDATES_PER_FRAME) {
        let task_pool = AsyncComputeTaskPool::get();

        let min = chunk_pos_to_world(chunk_pos).extend(MIN_HEIGHT);
        let max = chunk_pos_to_world(chunk_pos + IVec2::ONE).extend(MAX_HEIGHT);
        let aabb = Aabb::new(min.into(), max.into());

        let mut colliders = ColliderSet::new();
        let callback = |&handle: &ColliderHandle| {
            if let Some(collider) = physics_context.colliders.get(handle) {
                let affects_navmesh = collider
                    .collision_groups()
                    .memberships
                    .contains(CollisionLayers::STATIC.into());
                if !affects_navmesh {
                    return true; // continue search
                }

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
            .insert((res, NavMeshGenerated));
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
        let grid_size = UVec2::splat(CHUNK_TILES);
        NavMeshGenerator {
            chunk_pos,
            colliders,
            heightmap: Grid::new(grid_size, f32::NAN),
            connections: Grid::new(grid_size, 0),
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

        let mut cell_heights = Vec::new();

        for cell in self.heightmap.cells() {
            let pos = tile_pos_to_world(self.chunk_pos, cell);

            let ray_origin = pos.extend(MIN_HEIGHT);
            let max_toi = MAX_HEIGHT - MIN_HEIGHT;
            let solid = false;
            let filter = QueryFilter {
                groups: Some(CollisionLayers::STATIC_WALKABLE_GROUP.into()),
                ..Default::default()
            };

            cell_heights.clear();

            let callback = |_, intersection: RayIntersection| {
                let height = ray_origin.z + intersection.toi;
                cell_heights.push(height);
                true // continue search
            };

            query_pipeline.intersections_with_ray(
                &rigid_bodies,
                &self.colliders,
                &Ray::new(ray_origin.into(), Vec3::Z.into()),
                max_toi,
                solid,
                filter,
                callback,
            );

            cell_heights.sort_by(f32::total_cmp);

            for &height in &cell_heights {
                let capsule = Capsule::new_z(AGENT_HEIGHT * 0.5 - AGENT_RADIUS, AGENT_RADIUS);
                let capsule_pos = Isometry3::translation(
                    pos.x,
                    pos.y,
                    height + AGENT_HEIGHT * 0.5 + AGENT_OFFSET,
                );

                let filter = QueryFilter {
                    groups: Some(CollisionLayers::STATIC_GROUP.into()),
                    ..Default::default()
                };

                let is_collided = query_pipeline
                    .intersection_with_shape(
                        &rigid_bodies,
                        &self.colliders,
                        &capsule_pos,
                        &capsule,
                        filter,
                    )
                    .is_some();

                if !is_collided {
                    self.heightmap.set(cell, height);
                    break;
                }
            }
        }
    }

    fn generate_connections(&mut self) {
        for cell in self.heightmap.cells() {
            let cell_height = self.heightmap[cell];
            if cell_height.is_nan() {
                continue;
            }

            for (i, neighbor) in self.heightmap.neighborhood_8(cell) {
                let neighbor_height = self.heightmap[neighbor];
                if neighbor_height.is_nan() {
                    continue;
                }

                if (cell_height - neighbor_height).abs() <= CLIMB_HEIGHT {
                    self.connections[cell] |= (1 << i) as u8;
                }
            }
        }
    }
}

fn draw_nav_mesh_gizmos(
    q_chunks: Query<(&ChunkPos, &ChunkNavMesh, &ComputedVisibility)>,
    mut gizmos: Gizmos,
) {
    for (&ChunkPos(chunk_pos), nav_grid, visibility) in &q_chunks {
        if !visibility.is_visible() {
            continue;
        }

        for (cell, height) in nav_grid.heightmap.entries() {
            if height.is_nan() {
                continue;
            }

            let pos = tile_pos_to_world(chunk_pos, cell).extend(height + 0.1);

            for (i, neighbor) in nav_grid.heightmap.neighborhood_8(cell) {
                if nav_grid.connections[cell] & (1 << i) as u8 == 0 {
                    continue;
                }

                let neighbor_height = nav_grid.heightmap[neighbor];
                if neighbor_height.is_nan() {
                    continue;
                }

                let neighbor_pos =
                    tile_pos_to_world(chunk_pos, neighbor).extend(neighbor_height + 0.1);

                gizmos.line(pos, neighbor_pos, Color::GREEN);
            }
        }
    }
}
