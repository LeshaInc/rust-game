use std::f32::consts::TAU;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;
use rg_core::PoissonDiscSampling;
use rg_pixel_material::PixelMaterial;
use rg_worldgen::WorldSeed;

use crate::{chunk_pos_to_world, Chunk, ChunkPos, CHUNK_SIZE};

const MAX_UPDATES_PER_FRAME: usize = 4;

pub struct ScatterPlugin;

impl Plugin for ScatterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, scatter);
    }

    fn finish(&self, app: &mut App) {
        app.init_resource::<Prototype>();
    }
}

#[derive(Copy, Clone, Component)]
struct ChunkScattered;

fn scatter(
    q_chunks: Query<(Entity, &ChunkPos), (With<Chunk>, With<Collider>, Without<ChunkScattered>)>,
    seed: Res<WorldSeed>,
    prototype: Res<Prototype>,
    physics_context: Res<RapierContext>,
    mut commands: Commands,
) {
    for (chunk, chunk_pos) in q_chunks.iter().take(MAX_UPDATES_PER_FRAME) {
        let mut rng =
            Pcg32::seed_from_u64(seed.0 | (chunk_pos.0.x as u64) | (chunk_pos.0.y as u64) << 32);
        let points = PoissonDiscSampling::new(&mut rng, Vec2::splat(CHUNK_SIZE), 4.0).points;

        let mut children = Vec::with_capacity(points.len());

        for pos in points {
            let global_pos = chunk_pos_to_world(chunk_pos.0) + pos;

            let Some((_, toi)) = physics_context.cast_ray(
                global_pos.extend(1000.0),
                -Vec3::Z,
                2000.0,
                false,
                QueryFilter::new(),
            ) else {
                println!("failure");
                continue;
            };

            let z = 1000.0 - toi;
            let entity = prototype.spawn(&mut rng, &mut commands, pos.extend(z));
            children.push(entity);
        }

        commands
            .entity(chunk)
            .insert(ChunkScattered)
            .push_children(&children);
    }
}

#[derive(Resource)]
struct Prototype {
    mesh: Handle<Mesh>,
    material: Handle<PixelMaterial>,
}

impl Prototype {
    fn spawn<R: Rng>(&self, rng: &mut R, commands: &mut Commands, mut pos: Vec3) -> Entity {
        pos.y -= rng.gen_range(0.05..=0.25);

        let angle = rng.gen_range(0.0..TAU);
        let rotation = Quat::from_rotation_z(angle);
        let scale = rng.gen_range(0.7..=1.0);

        commands
            .spawn(MaterialMeshBundle {
                transform: Transform {
                    translation: pos,
                    rotation,
                    scale: Vec3::splat(scale),
                },
                mesh: self.mesh.clone(),
                material: self.material.clone(),
                ..default()
            })
            .id()
    }
}

impl FromWorld for Prototype {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(Res<AssetServer>, ResMut<Assets<PixelMaterial>>)> =
            SystemState::new(world);
        let (asset_server, mut materials) = system_state.get_mut(world);
        let mesh = asset_server.load("tree.glb#Mesh0/Primitive0");
        let material = materials.add(PixelMaterial { ..default() });
        Self { mesh, material }
    }
}
