use bevy::prelude::*;
use bevy::utils::HashMap;
use rg_core::NEIGHBORHOOD_8;

pub const CHUNK_SIZE: f32 = 16.0;
pub const TILE_SIZE: f32 = 0.5;
pub const CHUNK_TILES: u32 = 32;

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Chunks>()
            .init_resource::<ChunkSpawnCenter>()
            .init_resource::<ChunkSpawnRadius>()
            .init_resource::<ChunkDespawnRadius>()
            .add_systems(Update, (spawn_chunks, despawn_chunks));
    }
}

pub fn chunk_pos_to_world(chunk: IVec2) -> Vec2 {
    chunk.as_vec2() * CHUNK_SIZE
}

pub fn tile_pos_to_world(chunk: IVec2, tile: IVec2) -> Vec2 {
    chunk.as_vec2() * CHUNK_SIZE + tile.as_vec2() * TILE_SIZE
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Component)]
pub struct Chunk;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Component)]
pub struct ChunkPos(pub IVec2);

#[derive(Copy, Clone, Default, Resource)]
pub struct ChunkSpawnCenter(pub Vec2);

#[derive(Copy, Clone, Resource)]
pub struct ChunkSpawnRadius(pub f32);

impl Default for ChunkSpawnRadius {
    fn default() -> Self {
        Self(70.0)
    }
}

#[derive(Copy, Clone, Resource)]
pub struct ChunkDespawnRadius(pub f32);

impl Default for ChunkDespawnRadius {
    fn default() -> Self {
        Self(80.0)
    }
}

#[derive(Debug, Default, Resource)]
pub struct Chunks {
    map: HashMap<IVec2, Entity>,
}

impl Chunks {
    pub fn insert(&mut self, pos: IVec2, id: Entity) {
        self.map.insert(pos, id);
    }

    pub fn contains(&self, pos: IVec2) -> bool {
        self.map.contains_key(&pos)
    }

    pub fn get(&self, pos: IVec2) -> Option<Entity> {
        self.map.get(&pos).copied()
    }

    pub fn get_neighbors(&self, pos: IVec2) -> [Option<Entity>; 8] {
        NEIGHBORHOOD_8.map(|dir| self.get(pos + dir))
    }

    pub fn remove(&mut self, pos: IVec2) {
        self.map.remove(&pos);
    }

    pub fn retain(&mut self, mut f: impl FnMut(IVec2, Entity) -> bool) {
        self.map.retain(|k, v| f(*k, *v))
    }
}

fn spawn_chunks(
    mut commands: Commands,
    mut chunks: ResMut<Chunks>,
    center: Res<ChunkSpawnCenter>,
    radius: Res<ChunkSpawnRadius>,
) {
    let center = center.0;
    let radius = radius.0;

    let chunk_center = (center / CHUNK_SIZE).round().as_ivec2();
    let chunk_dist = (Vec2::new(radius, radius) / CHUNK_SIZE).ceil().as_ivec2();

    for sx in -chunk_dist.x..=chunk_dist.x {
        for sy in -chunk_dist.y..=chunk_dist.y {
            let chunk_pos = chunk_center + IVec2::new(sx, sy);
            let chunk_center = (chunk_pos.as_vec2() + Vec2::splat(0.5)) * CHUNK_SIZE;

            if chunk_center.distance_squared(center) > radius.powi(2) {
                continue;
            }

            if chunks.contains(chunk_pos) {
                continue;
            }

            let new_chunk = commands.spawn((
                Chunk,
                ChunkPos(chunk_pos),
                Transform::from_translation(chunk_pos_to_world(chunk_pos).extend(0.0)),
                GlobalTransform::default(),
                Visibility::Visible,
                ComputedVisibility::default(),
            ));

            chunks.insert(chunk_pos, new_chunk.id());
        }
    }
}

fn despawn_chunks(
    mut chunks: ResMut<Chunks>,
    mut commands: Commands,
    center: Res<ChunkSpawnCenter>,
    radius: Res<ChunkDespawnRadius>,
) {
    let center = center.0;
    let radius = radius.0;

    chunks.retain(|chunk_pos, chunk| {
        let chunk_center = (chunk_pos.as_vec2() + Vec2::splat(0.5)) * CHUNK_SIZE;

        if chunk_center.distance_squared(center) > radius.powi(2) {
            commands.entity(chunk).despawn_recursive();
            false
        } else {
            true
        }
    });
}
