use bevy::math::Vec3Swizzles;
use bevy::prelude::*;

use crate::utils::{get_barycentric, is_inside_barycentric};
use crate::{ChunkPos, CHUNK_SIZE, NAVGRID_RESOLUTION};

#[derive(Debug, Component)]
pub struct ChunkNavGrid {
    heights: Vec<f32>,
}

impl ChunkNavGrid {
    pub fn generate(positions: &[Vec3], indices: &[u32]) -> ChunkNavGrid {
        let _span = info_span!("chunk navgrid generator").entered();

        let mut heights = vec![0.0; (NAVGRID_RESOLUTION as usize).pow(2)];

        for indices in indices.chunks_exact(3) {
            let pos_a = positions[indices[0] as usize];
            let pos_b = positions[indices[1] as usize];
            let pos_c = positions[indices[2] as usize];

            let cell_a = pos_a.xz() / CHUNK_SIZE * (NAVGRID_RESOLUTION) as f32;
            let cell_b = pos_b.xz() / CHUNK_SIZE * (NAVGRID_RESOLUTION) as f32;
            let cell_c = pos_c.xz() / CHUNK_SIZE * (NAVGRID_RESOLUTION) as f32;

            let min_cell = cell_a.min(cell_b).min(cell_c).floor().as_ivec2();
            let max_cell = cell_a.max(cell_b).max(cell_c).ceil().as_ivec2();

            let cells = (min_cell.x..=max_cell.x)
                .flat_map(|x| (min_cell.y..=max_cell.y).map(move |y| IVec2::new(x, y)));
            for cell in cells {
                let cell_center = cell.as_vec2() + Vec2::new(0.5, 0.5);
                let center_pos = (cell_center / (NAVGRID_RESOLUTION as f32) * CHUNK_SIZE)
                    .extend(0.0)
                    .xzy();
                let bary = get_barycentric(pos_a, pos_b, pos_c, center_pos);
                if !is_inside_barycentric(bary) {
                    continue;
                }

                let cell_index =
                    (cell.x as usize) * (NAVGRID_RESOLUTION as usize) + (cell.y as usize);
                let height = bary.dot(Vec3::new(pos_a.y, pos_b.y, pos_c.y));
                heights[cell_index] = height;
            }
        }

        ChunkNavGrid { heights }
    }
}

#[allow(dead_code)]
pub fn draw_nav_grid_gizmos(q_chunks: Query<(&ChunkPos, &ChunkNavGrid)>, mut gizmos: Gizmos) {
    for (chunk_pos, nav_grid) in &q_chunks {
        for x in 0..NAVGRID_RESOLUTION {
            for y in 0..NAVGRID_RESOLUTION {
                let index = (x as usize) * (NAVGRID_RESOLUTION as usize) + (y as usize);
                let height = nav_grid.heights[index];
                let pos_2d = ((UVec2::new(x, y).as_vec2() + Vec2::new(0.5, 0.5))
                    / (NAVGRID_RESOLUTION as f32)
                    + chunk_pos.0.as_vec2())
                    * CHUNK_SIZE;
                let pos = pos_2d.extend(height + 0.0).xzy();
                gizmos.circle(pos, Vec3::Y, 0.05, Color::RED);
            }
        }
    }
}
