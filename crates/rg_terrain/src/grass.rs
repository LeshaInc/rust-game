use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;
use rg_billboard::{BillboardInstance, MultiBillboard};
use rg_core::PoissonDiscSampling;

use crate::utils::{get_barycentric, is_inside_barycentric};
use crate::CHUNK_SIZE;

pub const MIN_RADIUS: f32 = 0.2;

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

    let mut rng = Pcg32::seed_from_u64(seed ^ (chunk_pos.x as u64) ^ (chunk_pos.y as u64) << 32);
    let grid = PoissonDiscSampling::new(&mut rng, CHUNK_SIZE, MIN_RADIUS).grid;

    let mut instances = Vec::with_capacity(32 * 1024);

    for indices in indices.chunks_exact(3) {
        let pos_a = positions[indices[0] as usize];
        let pos_b = positions[indices[1] as usize];
        let pos_c = positions[indices[2] as usize];

        let cell_a = pos_a.xy() / CHUNK_SIZE * grid.size().as_vec2();
        let cell_b = pos_b.xy() / CHUNK_SIZE * grid.size().as_vec2();
        let cell_c = pos_c.xy() / CHUNK_SIZE * grid.size().as_vec2();

        let min_cell = cell_a.min(cell_b).min(cell_c).floor().as_ivec2();
        let max_cell = cell_a.max(cell_b).max(cell_c).ceil().as_ivec2();

        let cells = (min_cell.x..=max_cell.x)
            .flat_map(|x| (min_cell.y..=max_cell.y).map(move |y| IVec2::new(x, y)));

        let points = cells.flat_map(|cell| grid.get(cell));
        for &pos in points {
            let mut pos = pos.extend(0.0);
            let bary = get_barycentric(pos_a, pos_b, pos_c, pos);
            if !is_inside_barycentric(bary) {
                continue;
            }

            pos.z = bary.dot(Vec3::new(pos_a.z, pos_b.z, pos_c.z));

            instances.push(BillboardInstance {
                pos,
                size: Vec2::new(8.0 / 48.0, 16.0 / 48.0),
                color: Vec3::new(1.0, 1.0, 1.0),
                random: rng.gen_range(0..u32::MAX),
            });
        }
    }

    GeneratedGrass {
        multi_billboard: MultiBillboard {
            instances: instances.into(),
            anchor: Vec2::new(0.5, 1.0),
        },
    }
}
