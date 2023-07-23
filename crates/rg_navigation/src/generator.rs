use bevy::prelude::*;
use bevy_rapier3d::na::Isometry3;
use bevy_rapier3d::prelude::RapierContext;
use bevy_rapier3d::rapier::prelude::{
    Aabb, Capsule, ColliderBuilder, ColliderHandle, ColliderSet, QueryFilter, QueryPipeline, Ray,
    RayIntersection, RigidBodySet,
};
use rg_core::{CollisionLayers, Grid};
use rg_terrain::{chunk_pos_to_world, CHUNK_SIZE, CHUNK_TILES};
use smallvec::SmallVec;

pub const NAVMESH_SIZE: u32 = 2 * CHUNK_TILES;

#[derive(Debug, Clone, Copy, Resource)]
pub struct NavMeshSettings {
    pub min_world_z: f32,
    pub max_world_z: f32,
    pub climb_height: f32,
    pub agent_height: f32,
    pub agent_radius: f32,
    pub agent_offset: f32,
}

impl Default for NavMeshSettings {
    fn default() -> Self {
        Self {
            min_world_z: -200.0,
            max_world_z: 200.0,
            climb_height: 0.5,
            agent_height: 1.8,
            agent_radius: 0.3,
            agent_offset: 0.2,
        }
    }
}

#[derive(Debug, Component)]
pub struct ChunkNavMesh {
    pub heightmap: Grid<f32>,
    pub connections: Grid<u8>,
}

pub fn extract_colliders(
    settings: &NavMeshSettings,
    physics_context: &RapierContext,
    chunk_pos: IVec2,
) -> ColliderSet {
    let min = chunk_pos_to_world(chunk_pos).extend(settings.min_world_z);
    let max = chunk_pos_to_world(chunk_pos + IVec2::ONE).extend(settings.max_world_z);
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

    colliders
}

pub fn generate_navmesh(
    settings: &NavMeshSettings,
    chunk_pos: IVec2,
    colliders: ColliderSet,
) -> ChunkNavMesh {
    let _span = info_span!("generate_navmesh").entered();

    let heightmap = generate_heightmap(settings, chunk_pos, colliders);
    let connections = generate_connections(settings, &heightmap);

    ChunkNavMesh {
        heightmap,
        connections,
    }
}

fn generate_heightmap(
    settings: &NavMeshSettings,
    chunk_pos: IVec2,
    colliders: ColliderSet,
) -> Grid<f32> {
    let _span = info_span!("generate_heightmap").entered();

    let rigid_bodies = RigidBodySet::new();
    let mut query_pipeline = QueryPipeline::new();
    query_pipeline.update(&rigid_bodies, &colliders);

    let size = UVec2::splat(NAVMESH_SIZE);
    Grid::par_from_fn(size, |cell| {
        let pos =
            chunk_pos_to_world(chunk_pos) + cell.as_vec2() / (NAVMESH_SIZE as f32) * CHUNK_SIZE;

        let ray_origin = pos.extend(settings.min_world_z);
        let max_toi = settings.max_world_z - settings.min_world_z;
        let solid = false;
        let filter = QueryFilter {
            groups: Some(CollisionLayers::STATIC_WALKABLE_GROUP.into()),
            ..Default::default()
        };

        let mut cell_heights = SmallVec::<[f32; 4]>::new();

        let callback = |_, intersection: RayIntersection| {
            let height = ray_origin.z + intersection.toi;
            cell_heights.push(height);
            true // continue search
        };

        query_pipeline.intersections_with_ray(
            &rigid_bodies,
            &colliders,
            &Ray::new(ray_origin.into(), Vec3::Z.into()),
            max_toi,
            solid,
            filter,
            callback,
        );

        cell_heights.sort_by(f32::total_cmp);

        for &height in &cell_heights {
            let capsule = Capsule::new_z(
                settings.agent_height * 0.5 - settings.agent_radius,
                settings.agent_radius,
            );

            let capsule_pos = Isometry3::translation(
                pos.x,
                pos.y,
                height + settings.agent_height * 0.5 + settings.agent_offset,
            );

            let filter = QueryFilter {
                groups: Some(CollisionLayers::STATIC_GROUP.into()),
                ..Default::default()
            };

            let is_collided = query_pipeline
                .intersection_with_shape(&rigid_bodies, &colliders, &capsule_pos, &capsule, filter)
                .is_some();

            if !is_collided {
                return height;
            }
        }

        f32::NAN
    })
}

fn generate_connections(settings: &NavMeshSettings, heightmap: &Grid<f32>) -> Grid<u8> {
    let _span = info_span!("generate_connections").entered();

    Grid::from_fn(heightmap.size(), |cell| {
        let cell_height = heightmap[cell];
        if cell_height.is_nan() {
            return 0;
        }

        let mut connections = 0;

        for (i, neighbor) in heightmap.neighborhood_4(cell) {
            let neighbor_height = heightmap[neighbor];
            if neighbor_height.is_nan() {
                continue;
            }

            if (cell_height - neighbor_height).abs() <= settings.climb_height {
                connections |= (1 << i) as u8;
            }
        }

        connections
    })
}
