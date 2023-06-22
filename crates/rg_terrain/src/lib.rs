use bevy::prelude::*;

pub const CHUNK_SIZE: f32 = 32.0;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup)
            .add_systems(Update, draw_future_chunk_gizmos);
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Component)]
pub struct ChunkPos(pub IVec2);

#[derive(Debug, Clone, Copy, Resource)]
pub struct ChunkSize(pub Vec2);

#[derive(Debug, Clone, Copy, Component)]
pub struct FutureChunk;

fn startup(mut commands: Commands) {
    for sx in -3..=3 {
        for sz in -3..=3 {
            commands.spawn((
                FutureChunk,
                ChunkPos(IVec2::new(sx, sz)),
                Transform::from_xyz(CHUNK_SIZE * sx as f32, 0.0, CHUNK_SIZE * sz as f32),
            ));
        }
    }
}

fn draw_future_chunk_gizmos(
    q_future_chunks: Query<&Transform, With<FutureChunk>>,
    mut gizmos: Gizmos,
) {
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
