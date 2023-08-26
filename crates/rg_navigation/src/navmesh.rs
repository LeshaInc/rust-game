use bevy::prelude::*;
use bevy::utils::HashMap;
use rg_core::chunk::{chunk_pos_to_world, WorldOrigin, CHUNK_SIZE, CHUNK_TILES};
use rg_core::grid::Grid;
use smallvec::SmallVec;

use crate::NAVMESH_QUALITY;

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
    pub is_empty: bool,
    pub height_map: Grid<f32>,
    pub connections: Grid<u8>,
    pub triangles: Vec<Triangle>,
}

impl NavMeshChunk {
    pub fn sample_height(&self, pos: Vec2) -> f32 {
        self.height_map
            .sample(pos / CHUNK_SIZE * ((CHUNK_TILES * NAVMESH_QUALITY) as f32) - 0.5)
    }
}

#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Vec2; 3],
    pub links: SmallVec<[Link; 3]>,
}

#[derive(Debug, Clone, Copy)]
pub struct Link {
    pub kind: LinkKind,
    pub segment: [Vec2; 2],
    pub edge: u8,
    pub opposite_triangle: u32,
    pub opposite_link: u8,
    pub opposite_edge: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum LinkKind {
    Internal,
    PosX,
    NegX,
    PosY,
    NegY,
}

pub fn draw_navmesh_gizmos(navmesh: Res<NavMesh>, mut gizmos: Gizmos, origin: Res<WorldOrigin>) {
    for (&chunk_pos, chunk) in navmesh.chunks.iter() {
        let chunk_origin = chunk_pos_to_world(origin.0, chunk_pos);
        let transform = |pos: Vec2| (chunk_origin + pos).extend(chunk.sample_height(pos) + 0.3);

        let mut line = |a: Vec2, b: Vec2, color: Color| {
            let subdiv = ((b - a).length() * 2.0).ceil();
            for k in 0..subdiv as i32 {
                let k = k as f32;
                let p = a + (b - a) / subdiv * k;
                let q = a + (b - a) / subdiv * (k + 1.0);
                gizmos.line(transform(p), transform(q), color);
            }
        };

        for triangle in &chunk.triangles {
            line(triangle.vertices[0], triangle.vertices[1], Color::RED);
            line(triangle.vertices[1], triangle.vertices[2], Color::RED);
            line(triangle.vertices[2], triangle.vertices[0], Color::RED);

            let center = (triangle.vertices[0] + triangle.vertices[1] + triangle.vertices[2]) / 3.0;

            for link in &triangle.links {
                let mid = (link.segment[0] + link.segment[1]) * 0.5;
                line(center, mid, Color::GREEN);
            }
        }
    }
}

pub fn draw_navmesh_heightmap_gizmos(
    navmesh: Res<NavMesh>,
    mut gizmos: Gizmos,
    origin: Res<WorldOrigin>,
) {
    for (&chunk_pos, chunk) in navmesh.chunks.iter() {
        let chunk_origin = chunk_pos_to_world(origin.0, chunk_pos);

        for (cell, height) in chunk.height_map.entries() {
            if height.is_nan() {
                continue;
            }

            let pos = (chunk_origin
                + (cell.as_vec2() + 0.5) / ((CHUNK_TILES * NAVMESH_QUALITY) as f32) * CHUNK_SIZE)
                .extend(height + 0.1);

            for (i, neighbor) in chunk.height_map.neighborhood_4(cell) {
                if chunk.connections[cell] & (1 << i) as u8 == 0 {
                    continue;
                }

                let neighbor_height = chunk.height_map[neighbor];
                if neighbor_height.is_nan() {
                    continue;
                }

                let neighbor_pos = (chunk_origin
                    + (neighbor.as_vec2() + 0.5) / ((CHUNK_TILES * NAVMESH_QUALITY) as f32)
                        * CHUNK_SIZE)
                    .extend(neighbor_height + 0.1);

                gizmos.line(pos, neighbor_pos, Color::GREEN);
            }
        }
    }
}
