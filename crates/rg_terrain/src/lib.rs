mod generator;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rg_pixel_material::PixelMaterial;

pub const CHUNK_SIZE: f32 = 32.0;
pub const CHUNK_RESOLUTION: u32 = 64;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Seed(0))
            .add_systems(Startup, startup)
            .add_systems(Update, (refresh_chunks, draw_chunk_gizmos));
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Component)]
pub struct ChunkPos(pub IVec2);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Resource)]
pub struct Seed(pub u64);

#[derive(Debug, Component)]
pub struct FutureChunk(Task<Mesh>);

fn startup(mut commands: Commands, seed: Res<Seed>) {
    let task_pool = AsyncComputeTaskPool::get();
    let seed = seed.0;

    for sx in -3..=3 {
        for sz in -3..=3 {
            let chunk_pos = IVec2::new(sx, sz);
            let task = task_pool.spawn(async move { generator::generate(seed, chunk_pos) });

            commands.spawn((
                FutureChunk(task),
                ChunkPos(chunk_pos),
                Transform::from_xyz(CHUNK_SIZE * sx as f32, 0.0, CHUNK_SIZE * sz as f32),
            ));
        }
    }
}

fn refresh_chunks(
    mut q_future_chunks: Query<(Entity, &Transform, &mut FutureChunk)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PixelMaterial>>,
) {
    for (chunk_id, transform, mut future_chunk) in &mut q_future_chunks {
        let Some(mesh) = future::block_on(future::poll_once(&mut future_chunk.0)) else  {
            continue;
        };

        let mesh = meshes.add(mesh);
        println!("added chunk!");

        commands
            .entity(chunk_id)
            .remove::<FutureChunk>()
            .insert(MaterialMeshBundle {
                mesh,
                material: materials.add(PixelMaterial {
                    color: Color::rgb(0.3, 0.7, 0.3),
                    dither_enabled: false,
                    ..default()
                }),
                transform: *transform,
                ..default()
            });
    }
}

fn draw_chunk_gizmos(q_future_chunks: Query<&Transform, With<FutureChunk>>, mut gizmos: Gizmos) {
    for transform in &q_future_chunks {
        let points = [
            Vec3::new(-CHUNK_SIZE, 0.0, -CHUNK_SIZE),
            Vec3::new(CHUNK_SIZE, 0.0, -CHUNK_SIZE),
            Vec3::new(CHUNK_SIZE, 0.0, CHUNK_SIZE),
            Vec3::new(-CHUNK_SIZE, 0.0, CHUNK_SIZE),
            Vec3::new(-CHUNK_SIZE, 0.0, -CHUNK_SIZE),
        ];
        gizmos.linestrip(
            points.map(|pt| transform.transform_point(pt * 0.5)),
            Color::BLUE,
        );
    }
}
