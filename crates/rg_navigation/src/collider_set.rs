use bevy::prelude::*;
use bevy_rapier3d::na::Isometry3;
use bevy_rapier3d::parry::query::Ray;
use bevy_rapier3d::prelude::{Collider as RapierCollider, RapierContext};
use bevy_rapier3d::rapier::prelude::{
    Capsule, Collider, ColliderBuilder, ColliderSet as RapierColliderSet, QueryFilter,
    QueryPipeline, RigidBodySet,
};
use rg_core::chunk::chunk_pos_to_world;
use rg_core::CollisionLayers;
use rg_navigation_api::NavMeshAffector;

use crate::{NavMeshSettings, CHUNK_OVERSCAN};

pub struct ColliderSet {
    collider_set: RapierColliderSet,
    rigid_body_set: RigidBodySet,
    query_pipeline: QueryPipeline,
}

impl ColliderSet {
    pub fn new() -> ColliderSet {
        ColliderSet {
            collider_set: RapierColliderSet::new(),
            rigid_body_set: RigidBodySet::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }

    pub fn extract(
        settings: &NavMeshSettings,
        context: &RapierContext,
        q_affectors: &Query<(), With<NavMeshAffector>>,
        origin: IVec2,
        chunk_pos: IVec2,
    ) -> ColliderSet {
        let mut set = ColliderSet::new();

        let min =
            (chunk_pos_to_world(origin, chunk_pos) - CHUNK_OVERSCAN).extend(settings.min_world_z);
        let max = (chunk_pos_to_world(origin, chunk_pos + IVec2::ONE) + CHUNK_OVERSCAN)
            .extend(settings.max_world_z);

        let shape = RapierCollider::cuboid(
            (max.x - min.x) * 0.5,
            (max.y - min.y) * 0.5,
            (max.z - min.z) * 0.5,
        );

        let shape_pos = (min + max) * 0.5;

        context.intersections_with_shape(
            shape_pos,
            Quat::IDENTITY,
            &shape,
            Default::default(),
            |entity| {
                let Some(&handle) = context.entity2collider().get(&entity) else {
                    return true;
                };

                let Some(collider) = context.colliders.get(handle) else {
                    return true;
                };

                let affects_navmesh = q_affectors.contains(entity);
                if affects_navmesh {
                    set.insert_collider(collider);
                }

                true
            },
        );

        set
    }

    pub fn insert_collider(&mut self, collider: &Collider) {
        self.collider_set.insert(
            ColliderBuilder::new(collider.shared_shape().clone())
                .position(*collider.position())
                .collision_groups(collider.collision_groups())
                .build(),
        );
    }

    pub fn update(&mut self) {
        self.query_pipeline
            .update(&self.rigid_body_set, &self.collider_set);
    }

    pub fn is_empty(&self) -> bool {
        self.collider_set.is_empty()
    }

    pub fn check_walkability(&self, settings: &NavMeshSettings, pos: Vec2) -> Option<f32> {
        let z = self.raycast(settings, pos)?;

        if self.intersects_agent(settings, pos.extend(z)) {
            return None;
        }

        Some(z)
    }

    pub fn raycast(&self, settings: &NavMeshSettings, pos: Vec2) -> Option<f32> {
        let filter = QueryFilter {
            predicate: Some(&|_, collider: &Collider| {
                collider
                    .collision_groups()
                    .memberships
                    .contains(CollisionLayers::WALKABLE.into())
            }),
            ..default()
        };

        self.query_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            &Ray::new(pos.extend(settings.max_world_z).into(), (-Vec3::Z).into()),
            settings.max_world_z - settings.min_world_z,
            false,
            filter,
        )?;

        let capsule = Capsule::new_z(
            settings.agent_height * 0.5 - settings.agent_radius,
            settings.agent_radius,
        );

        let capsule_pos = Isometry3::translation(
            pos.x,
            pos.y,
            settings.max_world_z + settings.agent_height * 0.5,
        );

        let (_, toi) = self.query_pipeline.cast_shape(
            &self.rigid_body_set,
            &self.collider_set,
            &capsule_pos,
            &(-Vec3::Z).into(),
            &capsule,
            settings.max_world_z - settings.min_world_z,
            false,
            filter,
        )?;

        Some(settings.max_world_z - toi.toi + settings.agent_offset)
    }

    pub fn intersects_agent(&self, settings: &NavMeshSettings, pos: Vec3) -> bool {
        let capsule = Capsule::new_z(
            settings.agent_height * 0.5 - settings.agent_radius,
            settings.agent_radius,
        );

        let capsule_pos = Isometry3::translation(pos.x, pos.y, pos.z + settings.agent_height * 0.5);

        let intersection = self.query_pipeline.intersection_with_shape(
            &self.rigid_body_set,
            &self.collider_set,
            &capsule_pos,
            &capsule,
            QueryFilter {
                predicate: Some(&|_, collider: &Collider| {
                    collider
                        .collision_groups()
                        .memberships
                        .contains(CollisionLayers::STATIC.into())
                }),
                ..default()
            },
        );

        intersection.is_some()
    }
}
