use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use rg_billboard::{BillboardInstance, MultiBillboard};

use crate::poisson::poisson_disc_sampling;
use crate::CHUNK_SIZE;

#[derive(Debug)]
pub struct GeneratedGrass {
    pub multi_billboard: MultiBillboard,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct ChunkGrass(pub Entity);

pub fn generate(
    seed: u64,
    chunk_pos: IVec2,
    positions: &[Vec3],
    indices: &[u32],
) -> GeneratedGrass {
    let _span = info_span!("chunk grass generator").entered();

    let mut instances = Vec::new();

    let grid = poisson_disc_sampling(seed, chunk_pos, 0.005);
    let grid_resolution = grid.resolution() as f32;

    for indices in indices.chunks_exact(3) {
        let pos_a = positions[indices[0] as usize];
        let pos_b = positions[indices[1] as usize];
        let pos_c = positions[indices[2] as usize];

        let cell_a = pos_a.xz() / CHUNK_SIZE * grid_resolution;
        let cell_b = pos_b.xz() / CHUNK_SIZE * grid_resolution;
        let cell_c = pos_c.xz() / CHUNK_SIZE * grid_resolution;

        let min_cell = cell_a.min(cell_b).min(cell_c).floor().as_ivec2();
        let max_cell = cell_a.max(cell_b).max(cell_c).ceil().as_ivec2();

        let cells = (min_cell.x..=max_cell.x)
            .flat_map(|x| (min_cell.y..=max_cell.y).map(move |y| IVec2::new(x, y)));

        let points = cells.flat_map(|cell| grid.get(cell));
        for pos in points {
            let mut pos = (pos * CHUNK_SIZE).extend(0.0).xzy();
            let bary = get_barycentric(pos_a, pos_b, pos_c, pos);
            if !is_inside_barycentric(bary) {
                continue;
            }

            pos.y = bary.dot(Vec3::new(pos_a.y, pos_b.y, pos_c.y));

            instances.push(BillboardInstance {
                pos,
                size: Vec2::new(0.05, 0.2),
                color: Vec3::new(0.1, 0.4, 0.1),
                uv_rect: Vec4::ZERO,
            });
        }
    }

    GeneratedGrass {
        multi_billboard: MultiBillboard {
            instances: instances.into(),
            anchor: Vec2::new(0.5, 0.0),
        },
    }
}

fn get_barycentric(a: Vec3, b: Vec3, c: Vec3, p: Vec3) -> Vec3 {
    let area_abc = ((b - a).cross(c - a)).y;
    let area_pbc = ((b - p).cross(c - p)).y;
    let area_pca = ((c - p).cross(a - p)).y;
    let bary_x = area_pbc / area_abc;
    let bary_y = area_pca / area_abc;
    Vec3::new(bary_x, bary_y, 1.0 - bary_x - bary_y)
}

fn is_inside_barycentric(bary: Vec3) -> bool {
    (0.0 <= bary.x && bary.x <= 1.0)
        && (0.0 <= bary.y && bary.y <= 1.0)
        && (0.0 <= bary.z && bary.z <= 1.0)
}
