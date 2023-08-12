use bevy::prelude::*;
use bevy::utils::HashMap;
use rg_core::Grid;

use crate::{chunk_pos_to_world, CHUNK_CELLS, CHUNK_SIZE};

#[derive(Debug, Default, Resource)]
pub struct NavMesh {
    chunks: HashMap<IVec2, NavMeshChunk>,
}

impl NavMesh {
    pub fn insert_chunks(&mut self, chunk_pos: IVec2, chunk: NavMeshChunk) {
        self.chunks.insert(chunk_pos, chunk);
    }

    pub fn remove_chunk(&mut self, chunk_pos: IVec2) {
        self.chunks.remove(&chunk_pos);
    }
}

#[derive(Debug, Component)]
pub struct NavMeshChunk {
    pub height_map: Grid<f32>,
    pub connections: Grid<u8>,
    pub edges: Vec<(Vec2, Vec2)>,
    pub triangulation_edges: Vec<(Vec2, Vec2)>,
}

impl NavMeshChunk {
    pub fn sample_height(&self, pos: Vec2) -> f32 {
        self.height_map
            .sample(pos / CHUNK_SIZE * (CHUNK_CELLS as f32) - 0.5)
    }
}

pub fn draw_navmesh_gizmos(navmesh: Res<NavMesh>, mut gizmos: Gizmos) {
    for (&chunk_pos, chunk) in navmesh.chunks.iter() {
        let world_pos = chunk_pos_to_world(chunk_pos);

        for &(start, end) in &chunk.triangulation_edges {
            let start_z = chunk.sample_height(start) + 0.1;
            let end_z = chunk.sample_height(end) + 0.1;

            let start = (world_pos + start).extend(start_z);
            let end = (world_pos + end).extend(end_z);
            gizmos.line(start, end, Color::RED);
        }
    }
}

pub fn draw_navmesh_heightmap_gizmos(navmesh: Res<NavMesh>, mut gizmos: Gizmos) {
    for (&chunk_pos, chunk) in navmesh.chunks.iter() {
        for (cell, height) in chunk.height_map.entries() {
            if height.is_nan() {
                continue;
            }

            let pos = (chunk_pos_to_world(chunk_pos)
                + (cell.as_vec2() + 0.5) / (CHUNK_CELLS as f32) * CHUNK_SIZE)
                .extend(height + 0.1);

            for (i, neighbor) in chunk.height_map.neighborhood_4(cell) {
                if chunk.connections[cell] & (1 << i) as u8 == 0 {
                    continue;
                }

                let neighbor_height = chunk.height_map[neighbor];
                if neighbor_height.is_nan() {
                    continue;
                }

                let neighbor_pos = (chunk_pos_to_world(chunk_pos)
                    + (neighbor.as_vec2() + 0.5) / (CHUNK_CELLS as f32) * CHUNK_SIZE)
                    .extend(neighbor_height + 0.1);

                gizmos.line(pos, neighbor_pos, Color::GREEN);
            }
        }
    }
}
