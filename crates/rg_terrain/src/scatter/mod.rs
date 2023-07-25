pub mod bush;
pub mod tree;

use std::marker::PhantomData;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;
use rg_core::PoissonDiscSampling;
use rg_worldgen::{SharedWorldMaps, WorldMaps, WorldSeed};

use self::bush::BushPrototype;
use self::tree::TreePrototype;
use crate::chunk::ChunkFullyLoaded;
use crate::{chunk_pos_to_world, Chunk, ChunkPos, ChunkSpawnCenter, CHUNK_SIZE};

pub struct ScatterPlugins;

impl PluginGroup for ScatterPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<ScatterPlugins>()
            .add(ScatterPlugin::<TreePrototype>::default())
            .add(ScatterPlugin::<BushPrototype>::default())
    }
}

pub trait ScatterPrototype: Resource + FromWorld + 'static {
    const SEED: u64;

    fn build_app(app: &mut App) {
        let _ = app;
    }

    fn poisson_disc_min_radius(&self) -> f32;

    fn poisson_disc_max_tries(&self) -> u32 {
        64
    }

    fn density(&self, world_maps: &WorldMaps, pos: Vec2) -> f32 {
        let _ = (world_maps, pos);
        1.0
    }

    fn spawn<R: Rng>(&self, rng: &mut R, commands: &mut Commands, pos: Vec3) -> Entity;
}

pub struct ScatterPlugin<T: ScatterPrototype>(PhantomData<T>);

impl<T: ScatterPrototype> Default for ScatterPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: ScatterPrototype> Plugin for ScatterPlugin<T> {
    fn build(&self, app: &mut App) {
        T::build_app(app);
        app.add_systems(
            Update,
            scatter::<T>.run_if(resource_exists::<SharedWorldMaps>()),
        );
    }

    fn finish(&self, app: &mut App) {
        app.init_resource::<T>();
    }
}

#[derive(Copy, Clone, Component)]
struct ChunkScattered<T>(PhantomData<T>);

fn scatter<T: ScatterPrototype>(
    q_chunks: Query<(Entity, &ChunkPos), (With<Chunk>, With<Collider>, Without<ChunkScattered<T>>)>,
    seed: Res<WorldSeed>,
    world_maps: Res<SharedWorldMaps>,
    prototype: Res<T>,
    physics_context: Res<RapierContext>,
    spawn_center: Res<ChunkSpawnCenter>,
    mut commands: Commands,
) {
    let spawn_center = spawn_center.0;

    let Some((chunk_id, chunk_pos)) = q_chunks.iter().min_by(|a, b| {
        let a = spawn_center.distance_squared(((a.1).0.as_vec2() + Vec2::splat(0.5)) * CHUNK_SIZE);
        let b = spawn_center.distance_squared(((b.1).0.as_vec2() + Vec2::splat(0.5)) * CHUNK_SIZE);
        a.total_cmp(&b)
    }) else {
        return;
    };

    let mut rng = Pcg32::seed_from_u64(
        T::SEED ^ seed.0 ^ (chunk_pos.0.x as u64) ^ (chunk_pos.0.y as u64) << 32,
    );

    let sampling = PoissonDiscSampling::new_tileable(
        T::SEED ^ seed.0,
        chunk_pos.0,
        Vec2::splat(CHUNK_SIZE),
        prototype.poisson_disc_min_radius(),
        prototype.poisson_disc_max_tries(),
    );

    let points = sampling.points;

    let mut children = Vec::new();

    for pos in points {
        let global_pos = chunk_pos_to_world(chunk_pos.0) + pos;
        let density = prototype.density(&world_maps, global_pos);
        if !rng.gen_bool(density as f64) {
            continue;
        }

        let Some((_, toi)) = physics_context.cast_ray(
            global_pos.extend(1000.0),
            -Vec3::Z,
            2000.0,
            false,
            QueryFilter::new(),
        ) else {
            continue;
        };

        let z = 1000.0 - toi;
        let entity = prototype.spawn(&mut rng, &mut commands, pos.extend(z));
        children.push(entity);
    }

    commands
        .entity(chunk_id)
        .insert((ChunkScattered::<T>(PhantomData), ChunkFullyLoaded))
        .push_children(&children);
}
